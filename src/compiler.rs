use crate::ast::lower::Lowerer;
use crate::ast::ty::Program;
use crate::borrowck::check::BorrowCheckCtx;
use crate::compiler::diagnostic::{Diagnostic, Severity};
use crate::compiler::output::CompileOutcome;
use crate::compiler::source::SourceFile;
use crate::compiler::utils::FrontendUtil;
use crate::hir::lower::HirLowerer;
use crate::hir::output::HirOutput;
use crate::ir::lower::IrLowerCtx;
use crate::lexer::error::LexError;
use crate::lexer::token::{Span, Token, TokenKind};
use crate::lexer::{LexOutput, Lexer};
use crate::my_grammar::{GrammarContext, generate_my_grammar_context};
use crate::parser::CstSpanDisplayMode;
use crate::parser::automaton::AutomationBuildErr;
use crate::parser::engine::ParserEngine;
use crate::parser::error::TableBuildError;
use crate::thir::lower::ThirLowerCtx;
use crate::typecheck::check::TypeckCtx;
use serde_binary_adv::Serializer;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

pub mod diagnostic;
pub mod output;
pub mod render;
pub mod source;
#[cfg(test)]
mod test;
pub mod utils;

pub use output::CompileOutput;

const FRONTEND_CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub enum CompilerInitError {
    GrammarBuild(String),
    AutomatonBuild(AutomationBuildErr),
    ParseTableBuild(TableBuildError),
}

pub struct Compiler {
    pub front_end: FrontendUtil,
}

impl Compiler {
    pub fn build(rebuild: bool) -> Result<Self, CompilerInitError> {
        println!("Building grammar context.");
        let grammar_ctx = generate_my_grammar_context().ok_or(CompilerInitError::GrammarBuild(
            "Failed to build grammar context".to_string(),
        ))?;
        let front_end = Self::load_or_build_frontend(grammar_ctx, rebuild);
        Ok(Compiler { front_end })
    }

    /// 编译器主流程
    pub fn compile(&self, source: SourceFile) -> CompileOutcome {
        let lex_output = Self::lex_source(&source);
        let tokens = lex_output.tokens;
        let mut diagnostics = lex_output
            .errors
            .iter()
            .map(|error| Diagnostic::from_lex_error(error, &source))
            .collect::<Vec<_>>();

        let parser = ParserEngine::new(&self.front_end.parse_table, &self.front_end.grammar_ctx);

        let parse_outcome = parser.parse_with_recovering(tokens.iter().cloned());
        diagnostics.extend(parse_outcome.errors.iter().map(|error| {
            Diagnostic::from_parse_error(error, &source, &self.front_end.grammar_ctx)
        }));

        let mut ast: Option<Program> = None;

        if let Some(parse_result) = &parse_outcome.result {
            let lowerer = Lowerer::new(&parse_result.cst, &source, &self.front_end.grammar_ctx);

            match lowerer.lower() {
                Ok(p) => ast = Some(p),
                Err(e) => diagnostics.extend(e.iter().map(|error| {
                    Diagnostic::from_lower_error(error, &source, &self.front_end.grammar_ctx)
                })),
            }
        }

        let mut hir_output = None;
        let mut typeck_output = None;
        let mut thir_output = None;
        let mut ir_output = None;

        if let Some(p) = &ast {
            let hir_lowerer = HirLowerer::new(p);
            let result = hir_lowerer.lower();
            diagnostics.extend(
                result
                    .errors
                    .iter()
                    .map(|error| Diagnostic::from_hir_lower_error(error, &source)),
            );

            hir_output = Some(HirOutput {
                hir: result.hir,
                defs: result.defs,
                locals: result.locals,
            })
        }

        if let Some(hir) = &hir_output {
            let result = TypeckCtx::new(&hir.hir, &hir.defs, &hir.locals).check_program();
            diagnostics.extend(
                result
                    .errors
                    .iter()
                    .map(|error| Diagnostic::from_type_error(error, &source, &result.tys)),
            );
            let has_prior_errors = diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error);
            if result.errors.is_empty() && !has_prior_errors {
                let borrowck_result = BorrowCheckCtx::new(&hir.hir, &hir.locals).check_program();
                diagnostics.extend(
                    borrowck_result
                        .errors
                        .iter()
                        .map(|error| Diagnostic::from_borrow_error(error, &source)),
                );
                if borrowck_result.errors.is_empty() {
                    let thir_result = ThirLowerCtx::new(
                        &hir.hir,
                        &hir.defs,
                        &hir.locals,
                        &result.results,
                        &result.tys,
                    )
                    .lower();
                    diagnostics.extend(
                        thir_result
                            .errors
                            .iter()
                            .map(|error| Diagnostic::from_thir_lower_error(error, &source)),
                    );
                    if thir_result.errors.is_empty() {
                        let ir_result = IrLowerCtx::new(
                            &thir_result.program,
                            &hir.defs,
                            &result.results,
                            &result.tys,
                        )
                        .lower();
                        diagnostics.extend(
                            ir_result
                                .errors
                                .iter()
                                .map(|error| Diagnostic::from_ir_lower_error(error, &source)),
                        );
                        ir_output = Some(ir_result);
                    }
                    thir_output = Some(thir_result);
                }
            }
            typeck_output = Some(result);
        }

        let output = parse_outcome.result.map(|parse_result| {
            CompileOutput::new(
                tokens,
                parse_result,
                ast,
                hir_output,
                typeck_output,
                thir_output,
                ir_output,
            )
        });

        CompileOutcome {
            source,
            output,
            diagnostics,
        }
    }

    pub fn display_cst<'a>(
        &'a self,
        output: &'a CompileOutput,
        source: &'a SourceFile,
    ) -> crate::parser::CSTDisplay<'a> {
        output.display_cst(&self.front_end.grammar_ctx, source)
    }

    pub fn display_cst_with_mode<'a>(
        &'a self,
        output: &'a CompileOutput,
        source: &'a SourceFile,
        span_mode: CstSpanDisplayMode,
    ) -> crate::parser::CSTDisplay<'a> {
        output.display_cst_with_mode(&self.front_end.grammar_ctx, source, span_mode)
    }

    fn lex_source(source: &SourceFile) -> LexOutput {
        let mut lexer = Lexer::new(source.text());
        let mut tokens: Vec<Token> = vec![];
        let mut errors: Vec<LexError> = vec![];

        while let Some(result) = lexer.next() {
            match result {
                Ok(token) => tokens.push(token),
                Err(err) => errors.push(err),
            }
        }
        let eof_pos = source.len_bytes();
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                start: eof_pos,
                end: eof_pos,
            },
        });
        LexOutput { tokens, errors }
    }
}

/// 管理编译器前端的创建和缓存
impl Compiler {
    fn load_or_build_frontend(grammar_ctx: GrammarContext, force_rebuild: bool) -> FrontendUtil {
        let cache_key = Self::frontend_cache_key(&grammar_ctx);
        let cache_path = Self::frontend_cache_path(&cache_key);
        if let Some(front_end) = Self::try_load_frontend_cache(&cache_path)
            && !force_rebuild
        {
            println!("Loaded frontend from cache: {:?}", cache_path);
            front_end
        } else {
            println!("Building frontend...");
            let front_end = FrontendUtil::build(grammar_ctx).expect("Failed to build frontend");
            Self::store_frontend_cache(&cache_path, &front_end);
            println!("Stored frontend to cache: {:?}", cache_path);
            front_end
        }
    }

    fn frontend_cache_key(grammar_ctx: &GrammarContext) -> String {
        let mut hasher = DefaultHasher::new();
        let mut serialized = Serializer::to_bytes(grammar_ctx, false)
            .expect("Failed to serialize grammar context for hashing");
        serialized.push(FRONTEND_CACHE_SCHEMA_VERSION as u8);
        serialized.hash(&mut hasher);
        hasher.write_u32(FRONTEND_CACHE_SCHEMA_VERSION);
        let hash_value = hasher.finish();
        format!("frontend-v{}-{}", FRONTEND_CACHE_SCHEMA_VERSION, hash_value)
    }

    fn frontend_cache_path(cache_key: &str) -> PathBuf {
        let mut cache_path = std::env::current_dir().unwrap();
        cache_path.push("cache");
        cache_path.push("frontend");
        cache_path.push(cache_key);
        cache_path.set_extension("bin");
        cache_path
    }

    fn try_load_frontend_cache(path: &Path) -> Option<FrontendUtil> {
        let mut file = File::open(path).ok()?;
        match FrontendUtil::load_from_file(&mut file) {
            Ok(frontend) => Some(frontend),
            Err(err) => {
                eprintln!("Failed to load frontend from file: {:?}", err);
                None
            }
        }
    }

    fn store_frontend_cache(path: &Path, front_end: &FrontendUtil) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut file = File::create(path).expect("Failed to create frontend cache file");
        front_end
            .save_to_file(&mut file)
            .expect("Failed to save frontend to cache file");
    }
}
