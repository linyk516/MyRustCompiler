use std::path::PathBuf;

use crate::{
    ast::lower::LowerError,
    borrowck::error::{BorrowError, BorrowErrorKind},
    compiler::source::SourceFile,
    hir::error::{HirLowerError, HirLowerErrorKind},
    ir::error::IrLowerError,
    lexer::{
        error::{LexError, LexErrorKind},
        token::{Span, Token},
    },
    my_grammar::GrammarContext,
    parser::{error::ParseError, state::StateID, symbol::TerminalId},
    thir::error::ThirLowerError,
    typecheck::{
        error::{TypeError, TypeErrorKind},
        ty::{TyId, TyKind, TyStore},
    },
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
    Lower,
    HirLower,
    Typecheck,
    Borrowck,
    ThirLower,
    IrLower,
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
    LexUnterminatedStringLiteral,
    LexInvalidStringEscape {
        ch: char,
    },
    ParseUnexpectedToken {
        state: usize,
        found: Option<DiagnosticToken>,
        expected: Vec<ExpectedTerminal>,
    },
    ParseInternal {
        reason: String,
    },
    LowerInternal {
        reason: String,
    },
    HirLowerDuplicateDef {
        name: String,
        previous: Span,
    },
    HirLowerUnsupportedItem {
        message: String,
    },
    HirLowerUndefinedName {
        name: String,
    },
    HirLowerDuplicateParam {
        name: String,
        previous: Span,
    },
    HirLowerDuplicateLocal {
        name: String,
        previous: Span,
    },
    HirLowerInternal {
        reason: String,
    },
    Typecheck {
        reason: String,
    },
    Borrowck {
        reason: String,
    },
    ThirLower {
        reason: String,
    },
    IrLower {
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
            LexErrorKind::UnterminatedStringLiteral => Self::error(
                DiagnosticStage::Lexer,
                DiagnosticCategory::UserInput,
                "E0003",
                "unterminated string literal",
                source,
                DiagnosticDetails::LexUnterminatedStringLiteral,
            )
            .help("add a closing `\"`")
            .label(span, range, "string literal starts here", true),
            LexErrorKind::InvalidStringEscape(ch) => Self::error(
                DiagnosticStage::Lexer,
                DiagnosticCategory::UserInput,
                "E0004",
                "invalid string escape",
                source,
                DiagnosticDetails::LexInvalidStringEscape { ch: *ch },
            )
            .help("supported escapes are `\\n`, `\\t`, `\\\\`, `\\\"`, and `\\0`")
            .label(span, range, format!("invalid escape `\\{}`", ch), true),
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

    /// 从LowerError构造诊断信息
    pub fn from_lower_error(
        error: &LowerError,
        source: &SourceFile,
        _grammar_ctx: &GrammarContext,
    ) -> Self {
        let span = error.span.clone();
        let range = SourceRange::from_span(&span, source);

        Self::error(
            DiagnosticStage::Lower,
            DiagnosticCategory::Internal,
            "E0201",
            "failed to lower CST into AST",
            source,
            DiagnosticDetails::LowerInternal {
                reason: error.message.clone(),
            },
        )
        .label(span, range, error.message.clone(), true)
        .note("this usually indicates a mismatch between grammar productions and AST lowering")
        .help("check the production tag handling in the lowerer")
    }

    /// 从HirLowerError构造诊断信息
    pub fn from_hir_lower_error(error: &HirLowerError, source: &SourceFile) -> Self {
        let span = error.span.clone();
        let range = SourceRange::from_span(&span, source);

        match &error.kind {
            HirLowerErrorKind::DuplicateDef { name, previous } => Self::error(
                DiagnosticStage::HirLower,
                DiagnosticCategory::UserInput,
                "E0301",
                "duplicate item definition",
                source,
                DiagnosticDetails::HirLowerDuplicateDef {
                    name: name.clone(),
                    previous: previous.clone(),
                },
            )
            .label(
                span,
                range,
                format!("`{}` is defined more than once", name),
                true,
            )
            .label(
                previous.clone(),
                SourceRange::from_span(previous, source),
                "previous definition is here",
                false,
            )
            .help("rename one of the definitions"),
            HirLowerErrorKind::UnsupportedItem { message } => Self::error(
                DiagnosticStage::HirLower,
                DiagnosticCategory::UserInput,
                "E0302",
                "unsupported HIR lowering construct",
                source,
                DiagnosticDetails::HirLowerUnsupportedItem {
                    message: message.clone(),
                },
            )
            .label(span, range, message.clone(), true),
            HirLowerErrorKind::UndefinedName { name } => Self::error(
                DiagnosticStage::HirLower,
                DiagnosticCategory::UserInput,
                "E0303",
                "cannot resolve name",
                source,
                DiagnosticDetails::HirLowerUndefinedName { name: name.clone() },
            )
            .label(
                span,
                range,
                format!("`{}` is not defined in this scope", name),
                true,
            )
            .help("declare it before use or check the spelling"),
            HirLowerErrorKind::DuplicateParam { name, previous } => Self::error(
                DiagnosticStage::HirLower,
                DiagnosticCategory::UserInput,
                "E0304",
                "duplicate function parameter",
                source,
                DiagnosticDetails::HirLowerDuplicateParam {
                    name: name.clone(),
                    previous: previous.clone(),
                },
            )
            .label(
                span,
                range,
                format!("parameter `{}` is declared again", name),
                true,
            )
            .label(
                previous.clone(),
                SourceRange::from_span(previous, source),
                "previous parameter is here",
                false,
            )
            .help("use a unique name for each function parameter"),
            HirLowerErrorKind::DuplicateLocal { name, previous } => Self::error(
                DiagnosticStage::HirLower,
                DiagnosticCategory::UserInput,
                "E0305",
                "duplicate local binding",
                source,
                DiagnosticDetails::HirLowerDuplicateLocal {
                    name: name.clone(),
                    previous: previous.clone(),
                },
            )
            .label(
                span,
                range,
                format!("local `{}` is declared again", name),
                true,
            )
            .label(
                previous.clone(),
                SourceRange::from_span(previous, source),
                "previous local binding is here",
                false,
            ),
            HirLowerErrorKind::Internal { message } => Self::error(
                DiagnosticStage::HirLower,
                DiagnosticCategory::Internal,
                "E0391",
                "HIR lowering internal error",
                source,
                DiagnosticDetails::HirLowerInternal {
                    reason: message.clone(),
                },
            )
            .label(span, range, message.clone(), true)
            .note("this usually indicates a bug in the AST to HIR lowering state"),
        }
    }

    /// 从TypeError构造诊断信息
    pub fn from_type_error(error: &TypeError, source: &SourceFile, tys: &TyStore) -> Self {
        let span = error.span.clone();
        let range = SourceRange::from_span(&span, source);

        match &error.kind {
            TypeErrorKind::MismatchedTypes { expected, actual } => {
                let expected_text = Self::type_text(*expected, tys);
                let actual_text = Self::type_text(*actual, tys);
                Self::type_error(
                    "E0401",
                    "mismatched types",
                    source,
                    format!("expected `{expected_text}`, found `{actual_text}`"),
                )
                .label(
                    span,
                    range,
                    format!("expected `{expected_text}`, found `{actual_text}`"),
                    true,
                )
            }
            TypeErrorKind::CannotInferType { ty } => {
                let ty_text = Self::type_text(*ty, tys);
                Self::type_error(
                    "E0402",
                    "cannot infer type",
                    source,
                    format!("cannot infer `{ty_text}`"),
                )
                .label(span, range, "type annotations are needed here", true)
            }
            TypeErrorKind::OccursCheckFailed { var, ty } => {
                let ty_text = Self::type_text(*ty, tys);
                Self::type_error(
                    "E0403",
                    "recursive inferred type",
                    source,
                    format!("type variable `?T{}` occurs inside `{}`", var, ty_text),
                )
                .label(span, range, "recursive type would be required here", true)
            }
            TypeErrorKind::NotCallable { callee } => {
                let callee_text = Self::type_text(*callee, tys);
                Self::type_error(
                    "E0404",
                    "called value is not a function",
                    source,
                    format!("type `{callee_text}` is not callable"),
                )
                .label(
                    span,
                    range,
                    format!("`{callee_text}` cannot be called"),
                    true,
                )
            }
            TypeErrorKind::WrongArgCount { expected, actual } => Self::type_error(
                "E0405",
                "wrong number of function arguments",
                source,
                format!("expected {expected} arguments, found {actual}"),
            )
            .label(
                span,
                range,
                format!("expected {expected} arguments, found {actual}"),
                true,
            ),
            TypeErrorKind::WrongVariadicArgCount {
                expected_at_least,
                actual,
            } => Self::type_error(
                "E0415",
                "wrong number of function arguments",
                source,
                format!("expected at least {expected_at_least} arguments, found {actual}"),
            )
            .label(
                span,
                range,
                format!("expected at least {expected_at_least} arguments, found {actual}"),
                true,
            ),
            TypeErrorKind::InvalidVariadicArgType { ty } => {
                let ty_text = Self::type_text(*ty, tys);
                Self::type_error(
                    "E0416",
                    "invalid variadic argument type",
                    source,
                    format!("type `{ty_text}` cannot be passed as a C variadic argument"),
                )
                .label(
                    span,
                    range,
                    format!("`{ty_text}` is not supported as a variadic argument"),
                    true,
                )
            }
            TypeErrorKind::UninitializedLocal { name } => Self::type_error(
                "E0421",
                "uninitialized local variable",
                source,
                format!("local `{name}` may be uninitialized when read"),
            )
            .label(span, range, format!("`{name}` is uninitialized here"), true),
            TypeErrorKind::UnitValueUsedAsRvalue => Self::type_error(
                "E0422",
                "unit value used as rvalue",
                source,
                "function returning `()` cannot be used as a value",
            )
            .label(span, range, "unit result cannot be used as a rvalue", true),
            TypeErrorKind::InvalidIndex { base, index } => {
                let base_text = Self::type_text(*base, tys);
                let index_text = Self::type_text(*index, tys);
                Self::type_error(
                    "E0406",
                    "invalid index expression",
                    source,
                    format!("cannot index `{base_text}` with `{index_text}`"),
                )
                .label(
                    span,
                    range,
                    format!("cannot index `{base_text}` with `{index_text}`"),
                    true,
                )
            }
            TypeErrorKind::ArrayIndexOutOfBounds { index, len } => Self::type_error(
                "E0423",
                "array index out of bounds",
                source,
                format!("array index {index} is out of bounds for length {len}"),
            )
            .label(
                span,
                range,
                format!("index {index} is out of bounds for array length {len}"),
                true,
            ),
            TypeErrorKind::InvalidArrayLength { len } => Self::type_error(
                "E0424",
                "invalid array length",
                source,
                format!("array length must be positive, found {len}"),
            )
            .label(span, range, "array length must be a positive integer", true),
            TypeErrorKind::InvalidForIterator { ty } => {
                let ty_text = Self::type_text(*ty, tys);
                Self::type_error(
                    "E0425",
                    "invalid for iterator",
                    source,
                    format!("type `{ty_text}` cannot be iterated by `for`"),
                )
                .label(span, range, "expected an array expression here", true)
            }
            TypeErrorKind::UnknownField { base, field } => {
                let base_text = Self::type_text(*base, tys);
                Self::type_error(
                    "E0417",
                    "unknown struct field",
                    source,
                    format!("type `{base_text}` has no field `{field}`"),
                )
                .label(
                    span,
                    range,
                    format!("unknown field `{field}` on `{base_text}`"),
                    true,
                )
            }
            TypeErrorKind::MissingStructField { def_id, field } => Self::type_error(
                "E0418",
                "missing struct field",
                source,
                format!("struct `{def_id:?}` is missing field `{field}`"),
            )
            .label(span, range, format!("missing field `{field}`"), true),
            TypeErrorKind::DuplicateStructField { field } => Self::type_error(
                "E0419",
                "duplicate struct field",
                source,
                format!("field `{field}` is specified more than once"),
            )
            .label(span, range, format!("duplicate field `{field}`"), true),
            TypeErrorKind::NotStruct { ty } => {
                let ty_text = Self::type_text(*ty, tys);
                Self::type_error(
                    "E0420",
                    "expected struct type",
                    source,
                    format!("type `{ty_text}` is not a struct"),
                )
                .label(
                    span,
                    range,
                    format!("`{ty_text}` has no named fields"),
                    true,
                )
            }
            TypeErrorKind::NotAssignable { target } => {
                let target_text = Self::type_text(*target, tys);
                Self::type_error(
                    "E0407",
                    "invalid assignment target",
                    source,
                    format!("cannot assign to target of type `{target_text}`"),
                )
                .label(span, range, "this expression cannot be assigned to", true)
            }
            TypeErrorKind::AssignThroughImmutableReference => Self::type_error(
                "E0426",
                "cannot assign through immutable reference",
                source,
                "cannot assign through an immutable reference",
            )
            .label(
                span,
                range,
                "immutable reference cannot be used for assignment",
                true,
            ),
            TypeErrorKind::CannotBorrow { mutable, ty } => {
                let ty_text = Self::type_text(*ty, tys);
                let borrow = if *mutable { "mutable" } else { "shared" };
                Self::type_error(
                    "E0408",
                    "cannot borrow expression",
                    source,
                    format!("cannot take a {borrow} borrow of `{ty_text}`"),
                )
                .label(
                    span,
                    range,
                    format!("cannot take a {borrow} borrow here"),
                    true,
                )
            }
            TypeErrorKind::CannotDeref { ty } => {
                let ty_text = Self::type_text(*ty, tys);
                Self::type_error(
                    "E0409",
                    "cannot dereference type",
                    source,
                    format!("type `{ty_text}` cannot be dereferenced"),
                )
                .label(
                    span,
                    range,
                    format!("`{ty_text}` cannot be dereferenced"),
                    true,
                )
            }
            TypeErrorKind::BreakOutsideLoop => Self::type_error(
                "E0410",
                "`break` outside of loop",
                source,
                "`break` can only be used inside a loop",
            )
            .label(span, range, "`break` is not inside a loop", true),
            TypeErrorKind::ContinueOutsideLoop => Self::type_error(
                "E0411",
                "`continue` outside of loop",
                source,
                "`continue` can only be used inside a loop",
            )
            .label(span, range, "`continue` is not inside a loop", true),
            TypeErrorKind::ReturnTypeMismatch { expected, actual } => {
                let expected_text = Self::type_text(*expected, tys);
                let actual_text = Self::type_text(*actual, tys);
                Self::type_error(
                    "E0412",
                    "function body has wrong return type",
                    source,
                    format!("expected `{expected_text}`, found `{actual_text}`"),
                )
                .label(
                    span,
                    range,
                    format!("expected `{expected_text}`, found `{actual_text}`"),
                    true,
                )
            }
            TypeErrorKind::IfBranchMismatch { then_ty, else_ty } => {
                let then_text = Self::type_text(*then_ty, tys);
                let else_text = Self::type_text(*else_ty, tys);
                Self::type_error(
                    "E0413",
                    "`if` branches have incompatible types",
                    source,
                    format!("then branch is `{then_text}`, else branch is `{else_text}`"),
                )
                .label(
                    span,
                    range,
                    format!("branches have incompatible types: `{then_text}` and `{else_text}`"),
                    true,
                )
            }
            TypeErrorKind::MissingElseForValueIf { then_ty } => {
                let then_text = Self::type_text(*then_ty, tys);
                Self::type_error(
                    "E0414",
                    "`if` expression is missing an `else` branch",
                    source,
                    format!("`if` with then type `{then_text}` needs an else branch"),
                )
                .label(
                    span,
                    range,
                    "add an `else` branch for this value expression",
                    true,
                )
            }
            TypeErrorKind::Internal { message } => Self::error(
                DiagnosticStage::Typecheck,
                DiagnosticCategory::Internal,
                "E0491",
                "typecheck internal error",
                source,
                DiagnosticDetails::Typecheck {
                    reason: message.clone(),
                },
            )
            .label(span, range, message.clone(), true)
            .note("this usually indicates a bug in the HIR type checker"),
        }
    }

    /// 从BorrowError构造诊断信息
    pub fn from_borrow_error(error: &BorrowError, source: &SourceFile) -> Self {
        let span = error.span.clone();
        let range = SourceRange::from_span(&span, source);
        let message = error.message();

        let diagnostic = Self::error(
            DiagnosticStage::Borrowck,
            DiagnosticCategory::UserInput,
            "E0501",
            "borrow conflict",
            source,
            DiagnosticDetails::Borrowck {
                reason: message.clone(),
            },
        )
        .label(span, range, message, true);

        match &error.kind {
            BorrowErrorKind::ConflictingBorrow {
                existing_span,
                existing,
                ..
            }
            | BorrowErrorKind::MutationWhileBorrowed {
                existing_span,
                existing,
                ..
            } => diagnostic.label(
                existing_span.clone(),
                SourceRange::from_span(existing_span, source),
                format!("{} borrow is active here", existing.name()),
                false,
            ),
        }
    }

    /// 从ThirLowerError构造诊断信息
    pub fn from_thir_lower_error(error: &ThirLowerError, source: &SourceFile) -> Self {
        let span = error.span.clone();
        let range = SourceRange::from_span(&span, source);
        let message = error.message();

        Self::error(
            DiagnosticStage::ThirLower,
            DiagnosticCategory::Internal,
            "E0591",
            "THIR lowering internal error",
            source,
            DiagnosticDetails::ThirLower {
                reason: message.clone(),
            },
        )
        .label(span, range, message, true)
        .note("this usually indicates a mismatch between HIR, TypeckResults, and THIR lowering")
    }

    /// 从IrLowerError构造诊断信息
    pub fn from_ir_lower_error(error: &IrLowerError, source: &SourceFile) -> Self {
        let span = error.span.clone();
        let range = SourceRange::from_span(&span, source);
        let message = error.message();

        Self::error(
            DiagnosticStage::IrLower,
            DiagnosticCategory::Internal,
            "E0691",
            "IR lowering internal error",
            source,
            DiagnosticDetails::IrLower {
                reason: message.clone(),
            },
        )
        .label(span, range, message, true)
        .note("this usually indicates a mismatch between THIR and IR lowering")
    }
}

impl Diagnostic {
    fn type_error(
        code: &'static str,
        message: impl Into<String>,
        source: &SourceFile,
        reason: impl Into<String>,
    ) -> Self {
        Self::error(
            DiagnosticStage::Typecheck,
            DiagnosticCategory::UserInput,
            code,
            message,
            source,
            DiagnosticDetails::Typecheck {
                reason: reason.into(),
            },
        )
    }

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

    fn type_text(ty: TyId, tys: &TyStore) -> String {
        match tys.kind(ty) {
            TyKind::Int(kind) => kind.name().to_string(),
            TyKind::Bool => "bool".to_string(),
            TyKind::Str => "str".to_string(),
            TyKind::Adt(def_id) => format!("{def_id:?}"),
            TyKind::Unit => "()".to_string(),
            TyKind::Never => "!".to_string(),
            TyKind::Tuple(elems) => {
                let elems = elems
                    .iter()
                    .map(|&elem| Self::type_text(elem, tys))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({elems})")
            }
            TyKind::Array { elem, len } => {
                format!("[{}; {}]", Self::type_text(*elem, tys), len)
            }
            TyKind::Ref { mutable, inner } => {
                if *mutable {
                    format!("&mut {}", Self::type_text(*inner, tys))
                } else {
                    format!("&{}", Self::type_text(*inner, tys))
                }
            }
            TyKind::Fn {
                params,
                ret,
                variadic,
            } => {
                let mut params = params
                    .iter()
                    .map(|&param| Self::type_text(param, tys))
                    .collect::<Vec<_>>();
                if *variadic {
                    params.push("...".to_string());
                }
                let params = params.join(", ");
                format!("fn({params}) -> {}", Self::type_text(*ret, tys))
            }
            TyKind::Infer(var) => format!("?T{}", var),
            TyKind::Error => "<error>".to_string(),
        }
    }
}
