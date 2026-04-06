use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use serde_binary_adv::Serializer;
use crate::compiler::source::SourceFile;
use crate::lexer::token::{Span, Token, TokenKind};
use crate::compiler::utils::FrontendUtil;
use crate::lexer::Lexer;
use crate::my_grammar::{generate_my_grammar_context, GrammarContext};
use crate::parser::automaton::AutomationBuildErr;
use crate::parser::engine::ParserEngine;
use crate::parser::error::{ParseError, TableBuildError};

pub mod utils;
pub mod source;
pub mod output;
mod diagnostic;
#[cfg(test)]
mod test;

pub use output::CompileOutput;

const FRONTEND_CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub enum CompilerInitError {
    GrammarBuild(String),
    AutomatonBuild(AutomationBuildErr),
    ParseTableBuild(TableBuildError),
}


pub struct Compiler{
    pub front_end: FrontendUtil,
}

impl Compiler {
    pub fn build() -> Result<Self, CompilerInitError> {
        let grammar_ctx = generate_my_grammar_context()
            .ok_or(CompilerInitError::GrammarBuild("Failed to build grammar context".to_string()))?;
        let front_end = Self::load_or_build_frontend(grammar_ctx);
        Ok(Compiler { front_end })
    }

    /// 编译器主流程
    pub fn compile(&self, source: SourceFile) -> Result<CompileOutput, ParseError> {
        let tokens = Self::lex_source(&source);
        let parser = ParserEngine::new(&self.front_end.parse_table, &self.front_end.grammar_ctx);
        let parse_result = parser.parse(tokens.iter().cloned())?;
        Ok(CompileOutput::new(source, tokens, parse_result))
    }

    pub fn display_cst<'a>(&'a self, output: &'a CompileOutput) -> crate::parser::CSTDisplay<'a> {
        output.display_cst(&self.front_end.grammar_ctx)
    }

    fn lex_source(source: &SourceFile) -> Vec<Token> {
        let mut tokens: Vec<Token> = Lexer::new(source.text()).collect();
        let eof_pos = source.len_bytes();
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                start: eof_pos,
                end: eof_pos,
            },
        });
        tokens
    }
}

/// 管理编译器前端的创建和缓存
impl Compiler{
    fn load_or_build_frontend(grammar_ctx: GrammarContext) -> FrontendUtil {
        let cache_key = Self::frontend_cache_key(&grammar_ctx);
        let cache_path = Self::frontend_cache_path(&cache_key);
        if let Some(front_end) = Self::try_load_frontend_cache(&cache_path) {
            println!("Loaded frontend from cache: {:?}", cache_path);
            front_end
        } else {
            println!("Building frontend...");
            let front_end = FrontendUtil::build(grammar_ctx)
                .expect("Failed to build frontend");
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
        front_end.save_to_file(&mut file).expect("Failed to save frontend to cache file");
    }

}
