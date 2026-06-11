use crate::ast::ty::Program;
use crate::compiler::diagnostic::{Diagnostic, Severity};
use crate::compiler::source::SourceFile;
use crate::hir::output::HirOutput;
use crate::ir::output::IrOutput;
use crate::lexer::token::Token;
use crate::my_grammar::GrammarContext;
use crate::parser::{CST, CSTDisplay, CstSpanDisplayMode, ParseResult};
use crate::thir::output::ThirOutput;
use crate::typecheck::result::TypeckOutput;

#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub tokens: Vec<Token>,
    pub parse_result: ParseResult,
    pub ast: Option<Program>,
    pub hir: Option<HirOutput>,
    pub typeck: Option<TypeckOutput>,
    pub thir: Option<ThirOutput>,
    pub ir: Option<IrOutput>,
}

impl CompileOutput {
    pub fn new(
        tokens: Vec<Token>,
        parse_result: ParseResult,
        ast: Option<Program>,
        hir: Option<HirOutput>,
        typeck: Option<TypeckOutput>,
        thir: Option<ThirOutput>,
        ir: Option<IrOutput>,
    ) -> Self {
        Self {
            tokens,
            parse_result,
            ast,
            hir,
            typeck,
            thir,
            ir,
        }
    }

    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    pub fn parse_result(&self) -> &ParseResult {
        &self.parse_result
    }

    pub fn cst(&self) -> &CST {
        &self.parse_result.cst
    }

    pub fn ast(&self) -> Option<&Program> {
        self.ast.as_ref()
    }

    pub fn hir(&self) -> Option<&HirOutput> {
        self.hir.as_ref()
    }

    pub fn typeck(&self) -> Option<&TypeckOutput> {
        self.typeck.as_ref()
    }

    pub fn thir(&self) -> Option<&ThirOutput> {
        self.thir.as_ref()
    }

    pub fn ir(&self) -> Option<&IrOutput> {
        self.ir.as_ref()
    }

    pub fn display_cst<'a>(
        &'a self,
        grammar_ctx: &'a GrammarContext,
        source: &'a SourceFile,
    ) -> CSTDisplay<'a> {
        grammar_ctx.grammar.display_cst(self.cst(), source.text())
    }

    pub fn display_cst_with_mode<'a>(
        &'a self,
        grammar_ctx: &'a GrammarContext,
        source: &'a SourceFile,
        span_mode: CstSpanDisplayMode,
    ) -> CSTDisplay<'a> {
        self.display_cst(grammar_ctx, source)
            .with_span_mode(span_mode)
    }
}

pub struct CompileOutcome {
    pub source: SourceFile,
    pub output: Option<CompileOutput>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CompileOutcome {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}
