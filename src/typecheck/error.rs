use crate::{
    lexer::token::Span,
    typecheck::ty::{TyId, TyVarId},
};

#[derive(Debug, Clone)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeErrorKind {
    MismatchedTypes { expected: TyId, actual: TyId },

    CannotInferType { ty: TyId },

    OccursCheckFailed { var: TyVarId, ty: TyId },

    NotCallable { callee: TyId },

    WrongArgCount { expected: usize, actual: usize },

    //InvalidUnaryOp { op: UnaryOp, operand: TyId },

    //InvalidBinaryOp { op: BinaryOp, lhs: TyId, rhs: TyId },
    InvalidIndex { base: TyId, index: TyId },

    NotAssignable { target: TyId },

    CannotBorrow { mutable: bool, ty: TyId },

    CannotDeref { ty: TyId },

    BreakOutsideLoop,
    ContinueOutsideLoop,

    ReturnTypeMismatch { expected: TyId, actual: TyId },

    IfBranchMismatch { then_ty: TyId, else_ty: TyId },

    MissingElseForValueIf { then_ty: TyId },

    Internal { message: String },
}
