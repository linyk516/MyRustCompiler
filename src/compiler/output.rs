use crate::compiler::diagnostic::{Diagnostic, Severity};
use crate::compiler::source::SourceFile;
use crate::lexer::token::Token;
use crate::my_grammar::GrammarContext;
use crate::parser::{CST, CSTDisplay, CstSpanDisplayMode, ParseResult};

pub struct CompileOutput {
    pub tokens: Vec<Token>,
    pub parse_result: ParseResult,
}

impl CompileOutput {
    pub fn new(tokens: Vec<Token>, parse_result: ParseResult) -> Self {
        Self {
            tokens,
            parse_result,
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
