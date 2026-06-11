use crate::lexer::token::Span;

#[derive(Debug, Clone)]
pub struct IrLowerError {
    pub kind: IrLowerErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum IrLowerErrorKind {
    MissingFunction { id: usize },
    MissingBody { id: usize },
    MissingBlock { id: usize },
    MissingExpr { id: usize },
    MissingStmt { id: usize },
    MissingLocal { id: usize },
    MissingCurrentFunction,
    MissingCurrentBlock,
    BreakOutsideLoop,
    ContinueOutsideLoop,
    InvalidPlace { message: String },
    UnsupportedValue { message: String },
    Internal { message: String },
}

impl IrLowerError {
    pub fn new(kind: IrLowerErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn message(&self) -> String {
        match &self.kind {
            IrLowerErrorKind::MissingFunction { id } => format!("missing IR function #{id}"),
            IrLowerErrorKind::MissingBody { id } => format!("missing THIR body #{id}"),
            IrLowerErrorKind::MissingBlock { id } => format!("missing IR basic block #{id}"),
            IrLowerErrorKind::MissingExpr { id } => format!("missing THIR expression #{id}"),
            IrLowerErrorKind::MissingStmt { id } => format!("missing THIR statement #{id}"),
            IrLowerErrorKind::MissingLocal { id } => {
                format!("missing IR local for THIR local #{id}")
            }
            IrLowerErrorKind::MissingCurrentFunction => {
                "IR lowering attempted to work outside a function".to_string()
            }
            IrLowerErrorKind::MissingCurrentBlock => {
                "IR lowering attempted to emit outside a basic block".to_string()
            }
            IrLowerErrorKind::BreakOutsideLoop => "`break` appeared outside a loop".to_string(),
            IrLowerErrorKind::ContinueOutsideLoop => {
                "`continue` appeared outside a loop".to_string()
            }
            IrLowerErrorKind::InvalidPlace { message }
            | IrLowerErrorKind::UnsupportedValue { message }
            | IrLowerErrorKind::Internal { message } => message.clone(),
        }
    }
}
