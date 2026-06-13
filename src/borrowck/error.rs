use crate::{hir::id::LocalId, lexer::token::Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

impl BorrowKind {
    pub fn name(self) -> &'static str {
        match self {
            BorrowKind::Shared => "shared",
            BorrowKind::Mutable => "mutable",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BorrowError {
    pub kind: BorrowErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum BorrowErrorKind {
    ConflictingBorrow {
        local_id: LocalId,
        local_name: String,
        requested: BorrowKind,
        existing: BorrowKind,
        existing_span: Span,
    },
    MutationWhileBorrowed {
        local_id: LocalId,
        local_name: String,
        existing: BorrowKind,
        existing_span: Span,
    },
}

impl BorrowError {
    pub fn new(kind: BorrowErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn message(&self) -> String {
        match &self.kind {
            BorrowErrorKind::ConflictingBorrow {
                local_name,
                requested,
                existing,
                ..
            } => format!(
                "cannot create {} borrow of `{local_name}` while a {} borrow is active",
                requested.name(),
                existing.name()
            ),
            BorrowErrorKind::MutationWhileBorrowed {
                local_name,
                existing,
                ..
            } => format!(
                "cannot modify `{local_name}` while a {} borrow is active",
                existing.name()
            ),
        }
    }
}
