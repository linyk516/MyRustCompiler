use std::path::PathBuf;

use crate::{
    compiler::source::SourceFile,
    lexer::{
        error::{LexError, LexErrorKind},
        token::{Span, Token},
    },
    my_grammar::GrammarContext,
    parser::{error::ParseError, state::StateID, symbol::TerminalId},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticStage {
    Lexer,
    Parser,
    FrontendBuild,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticCategory {
    UserInput,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
}

impl From<(usize, usize)> for SourcePosition {
    fn from((line, column): (usize, usize)) -> Self {
        Self {
            line: line + 1,
            column: column + 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRange {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceRange {
    pub fn from_span(span: &Span, source: &SourceFile) -> Option<Self> {
        let Some(start_pos) = source.line_utf8_col(span.start) else {
            return None;
        };
        let Some(end_pos) = source.line_utf8_col(span.end) else {
            return None;
        };
        Some(Self {
            start: start_pos.into(),
            end: end_pos.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub range: Option<SourceRange>,
    pub message: String,
    pub primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticDetails {
    LexUnknownCharacter {
        ch: char,
    },
    LexUnterminatedBlockComment,
    ParseUnexpectedToken {
        state: usize,
        found: Option<DiagnosticToken>,
        expected: Vec<ExpectedTerminal>,
    },
    ParseInternal {
        reason: String,
    },
}

impl DiagnosticDetails {
    pub fn from_unexpected_token(
        state: &StateID,
        lookahead: &Option<Token>,
        expected: &[TerminalId],
        source: &SourceFile,
        grammar_ctx: &GrammarContext,
    ) -> Self {
        let found = lookahead
            .as_ref()
            .map(|token| DiagnosticToken::from_token(token, source, grammar_ctx));
        let expected = expected
            .iter()
            .map(|terminal| ExpectedTerminal::from_terminal_id(*terminal, grammar_ctx))
            .collect::<Vec<_>>();
        Self::ParseUnexpectedToken {
            state: state.0,
            found,
            expected,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticToken {
    pub kind: String,
    pub text: String,
    pub span: Span,
    pub range: Option<SourceRange>,
}

impl DiagnosticToken {
    pub fn from_token(token: &Token, source: &SourceFile, _grammar_ctx: &GrammarContext) -> Self {
        Self {
            kind: format!("{:?}", token.kind),
            text: token.span.text(source.text()).unwrap_or("").to_string(),
            span: token.span.clone(),
            range: SourceRange::from_span(&token.span, source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedTerminal {
    pub id: usize,
    pub name: String,    // grammar 内部名，例如 literal_i32
    pub display: String, // 面向用户，例如 integer literal 或 `;`
}

impl ExpectedTerminal {
    pub fn from_terminal_id(id: TerminalId, grammar_ctx: &GrammarContext) -> Self {
        let name = grammar_ctx.grammar.terminals[id.0].name.clone();
        let display = Self::terminal_display_name(&name);
        Self {
            id: id.0,
            name,
            display,
        }
    }

    fn terminal_display_name(name: &str) -> String {
        match name {
            "id" => "identifier".to_string(),
            "literal_i32" => "integer literal".to_string(),
            "eof" => "end of input".to_string(),
            other => format!("`{}`", other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub stage: DiagnosticStage,
    pub category: DiagnosticCategory,
    pub code: &'static str,
    pub message: String,
    pub source_path: Option<PathBuf>,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    pub help: Option<String>,
    pub details: DiagnosticDetails,
}

impl Diagnostic {
    /// 从LexError构造诊断信息
    pub fn from_lex_error(error: &LexError, source: &SourceFile) -> Self {
        let span = error.span.clone();
        let range = SourceRange::from_span(&span, source);
        match &error.kind {
            LexErrorKind::UnknownCharacter(ch) => Self::error(
                DiagnosticStage::Lexer,
                DiagnosticCategory::UserInput,
                "E0001",
                "unknown character while lexing",
                source,
                DiagnosticDetails::LexUnknownCharacter { ch: *ch },
            )
            .help("remove this character or replace it with a valid token")
            .label(span, range, format!("unknown character `{}`", ch), true),
            LexErrorKind::UnterminatedBlockComment => Self::error(
                DiagnosticStage::Lexer,
                DiagnosticCategory::UserInput,
                "E0002",
                "unterminated block comment",
                source,
                DiagnosticDetails::LexUnterminatedBlockComment,
            )
            .help("add a closing `*/`")
            .note("block comments support nesting")
            .label(span, range, "block comment starts here", true),
        }
    }

    /// 从ParseError构造诊断信息
    pub fn from_parse_error(
        error: &ParseError,
        source: &SourceFile,
        grammar_ctx: &GrammarContext,
    ) -> Self {
        match error {
            ParseError::UnexpectedToken {
                state,
                lookahead,
                expected,
            } => {
                let span = lookahead
                    .as_ref()
                    .map(|token| token.span.clone())
                    .unwrap_or_else(|| Span {
                        start: source.len_bytes(),
                        end: source.len_bytes(),
                    });
                let range = SourceRange::from_span(&span, source);
                let expected_terminals = expected
                    .iter()
                    .map(|terminal| ExpectedTerminal::from_terminal_id(*terminal, grammar_ctx))
                    .collect::<Vec<_>>();
                let details = DiagnosticDetails::from_unexpected_token(
                    state,
                    lookahead,
                    expected,
                    source,
                    grammar_ctx,
                );
                let label_message = match lookahead {
                    Some(token) => {
                        format!("found `{}`", Self::token_text(token, source))
                    }
                    None => "reached end of input".to_string(),
                };

                Self::error(
                    DiagnosticStage::Parser,
                    DiagnosticCategory::UserInput,
                    "E0101",
                    "unexpected token while parsing",
                    source,
                    details,
                )
                .note(format!(
                    "parser state I{} has no ACTION entry for this token",
                    state.0
                ))
                .optional_help(Self::expected_help(&expected_terminals))
                .label(span, range, label_message, true)
            }
            ParseError::MissingGoto {
                state,
                non_terminal,
            } => {
                let non_terminal = Self::non_terminal_name(*non_terminal, grammar_ctx);
                Self::internal_parse_error(
                    "E0191",
                    "parser internal error: missing GOTO entry",
                    source,
                    format!(
                        "missing GOTO entry from state I{} on <{}>",
                        state.0, non_terminal
                    ),
                )
                .note(format!(
                    "state I{} has no GOTO entry for <{}>",
                    state.0, non_terminal
                ))
                .note("this may indicate an inconsistent parse table or parser bug")
                .help("rebuild the frontend cache with `--rebuild`; if it persists, check the grammar and parse table")
            }
            ParseError::MissingProduction(production) => Self::internal_parse_error(
                "E0192",
                "parser internal error: missing production",
                source,
                format!("missing production #{}", production.0),
            )
            .note(format!(
                "production #{} was referenced but not found",
                production.0
            ))
            .note("this may indicate an inconsistent grammar or stale frontend cache")
            .help("rebuild the frontend cache with `--rebuild`"),
            ParseError::StackUnderflow => Self::internal_parse_error(
                "E0193",
                "parser internal error: stack underflow",
                source,
                "parser stack underflow",
            )
            .note("the parser attempted to pop from an empty stack")
            .note("this may indicate invalid reduce logic or an inconsistent parse table")
            .help("rebuild the frontend cache with `--rebuild`; if it persists, inspect parser reduce logic"),
        }
    }
}

impl Diagnostic {
    fn error(
        stage: DiagnosticStage,
        category: DiagnosticCategory,
        code: &'static str,
        message: impl Into<String>,
        source: &SourceFile,
        details: DiagnosticDetails,
    ) -> Self {
        Self {
            severity: Severity::Error,
            stage,
            category,
            code,
            message: message.into(),
            source_path: source.path().map(|path| path.to_path_buf()),
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
            details,
        }
    }

    fn internal_parse_error(
        code: &'static str,
        message: impl Into<String>,
        source: &SourceFile,
        reason: impl Into<String>,
    ) -> Self {
        Self::error(
            DiagnosticStage::Parser,
            DiagnosticCategory::Internal,
            code,
            message,
            source,
            DiagnosticDetails::ParseInternal {
                reason: reason.into(),
            },
        )
    }

    fn label(
        mut self,
        span: Span,
        range: Option<SourceRange>,
        message: impl Into<String>,
        primary: bool,
    ) -> Self {
        self.labels.push(DiagnosticLabel {
            span,
            range,
            message: message.into(),
            primary,
        });
        self
    }

    fn optional_help(mut self, help: Option<String>) -> Self {
        if let Some(help) = help {
            self.help = Some(help);
        }
        self
    }

    fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    fn expected_help(expected: &[ExpectedTerminal]) -> Option<String> {
        if expected.is_empty() {
            return Some("no valid token is accepted in the current parser state".to_string());
        }

        let display = expected
            .iter()
            .map(|terminal| terminal.display.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!("expected one of: {}", display))
    }

    fn token_text(token: &Token, source: &SourceFile) -> String {
        token
            .span
            .text(source.text())
            .filter(|text| !text.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{:?}", token.kind))
    }

    fn non_terminal_name(
        non_terminal: crate::parser::symbol::NonTerminalId,
        grammar_ctx: &GrammarContext,
    ) -> String {
        grammar_ctx
            .grammar
            .non_terminals
            .get(non_terminal.0)
            .map(|non_terminal| non_terminal.name.clone())
            .unwrap_or_else(|| format!("#{}", non_terminal.0))
    }
}
