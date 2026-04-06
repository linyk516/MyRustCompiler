use crate::compiler::source::SourceFile;
use crate::lexer::token::Token;
use crate::my_grammar::GrammarContext;
use crate::parser::{CST, CSTDisplay, ParseResult};

pub struct CompileOutput {
    pub source: SourceFile,
    pub tokens: Vec<Token>,
    pub parse_result: ParseResult,
}

impl CompileOutput {
    pub fn new(source: SourceFile, tokens: Vec<Token>, parse_result: ParseResult) -> Self {
        Self {
            source,
            tokens,
            parse_result,
        }
    }

    pub fn source(&self) -> &SourceFile {
        &self.source
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

    pub fn display_cst<'a>(&'a self, grammar_ctx: &'a GrammarContext) -> CSTDisplay<'a> {
        grammar_ctx
            .grammar
            .display_cst(self.cst(), self.source.text())
    }
}
