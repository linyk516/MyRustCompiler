use crate::lexer::token::Span;

#[derive(Debug, Clone)]
pub struct HirTy {
    pub span: Span,
    pub kind: HirTyKind,
}

#[derive(Debug, Clone)]
pub enum HirTyKind {
    I32,
    Unit,
    Ref { mutable: bool, inner: Box<HirTy> },

    Array { elem: Box<HirTy>, len: usize },

    Tuple(Vec<HirTy>),

    Err,
}

impl HirTy {
    pub fn new(kind: HirTyKind, span: Span) -> Self {
        Self { span, kind }
    }

    pub fn unit(span: Span) -> Self {
        Self {
            span,
            kind: HirTyKind::Unit,
        }
    }

    pub fn err(span: Span) -> Self {
        Self {
            span,
            kind: HirTyKind::Err,
        }
    }
}
