use crate::lexer::token::Span;

pub struct HirLowerError {
    pub kind: HirLowerErrorKind,
    pub span: Span,
}

pub enum HirLowerErrorKind {
    DuplicateDef { name: String, previous: Span },

    UnsupportedItem { message: String },

    UndefinedName { name: String },

    DuplicateParam { name: String, previous: Span },

    DuplicateLocal { name: String, previous: Span },

    Internal { message: String },
}
