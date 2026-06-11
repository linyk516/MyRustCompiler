use crate::{hir::id::DefId, lexer::token::Span};

#[derive(Debug, Clone)]
pub struct ThirLowerError {
    pub kind: ThirLowerErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ThirLowerErrorKind {
    MissingItem { id: usize },
    MissingBody { id: usize },
    MissingExpr { id: usize },
    MissingStmt { id: usize },
    MissingType { node: String },
    MissingLocal { id: usize },
    InvalidCallCallee { message: String },
    InvalidPlace { message: String },
    InvalidValue { message: String },
    DefAsValue { def_id: DefId },
    Internal { message: String },
}

impl ThirLowerError {
    pub fn new(kind: ThirLowerErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn message(&self) -> String {
        match &self.kind {
            ThirLowerErrorKind::MissingItem { id } => format!("missing HIR item #{id}"),
            ThirLowerErrorKind::MissingBody { id } => format!("missing HIR body #{id}"),
            ThirLowerErrorKind::MissingExpr { id } => format!("missing HIR expression #{id}"),
            ThirLowerErrorKind::MissingStmt { id } => format!("missing HIR statement #{id}"),
            ThirLowerErrorKind::MissingType { node } => {
                format!("missing typecheck result for {node}")
            }
            ThirLowerErrorKind::MissingLocal { id } => {
                format!("missing THIR local for HIR local #{id}")
            }
            ThirLowerErrorKind::InvalidCallCallee { message }
            | ThirLowerErrorKind::InvalidPlace { message }
            | ThirLowerErrorKind::InvalidValue { message }
            | ThirLowerErrorKind::Internal { message } => message.clone(),
            ThirLowerErrorKind::DefAsValue { def_id } => {
                format!("definition {def_id:?} is used as a value")
            }
        }
    }

    pub fn is_invalid_place(&self) -> bool {
        matches!(self.kind, ThirLowerErrorKind::InvalidPlace { .. })
    }
}
