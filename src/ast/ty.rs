use crate::ast::node::AstNode;

/*
 * 一些基本的元素
 */
pub type Ident = AstNode<String>;

pub type Ty = AstNode<TyKind>;

#[derive(Debug, Clone)]
pub enum TyKind {
    I32,
    Ref { mutable: bool, inner: Box<Ty> },
    Array { elem: Box<Ty>, len: usize },
    Tuple(Vec<Ty>),
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// AST根
#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    Fn(FnDecl),
}

pub type Item = AstNode<ItemKind>;

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret_ty: Option<Ty>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub mutable: bool,
    pub name: Ident,
    pub ty: Ty,
}

pub type Block = AstNode<BlockKind>;

#[derive(Debug, Clone)]
pub struct BlockKind {
    pub stmts: Vec<Stmt>,
    pub tail_expr: Option<Box<Expr>>,
}

pub type Stmt = AstNode<StmtKind>;

#[derive(Debug, Clone)]
pub enum StmtKind {
    Let {
        mutable: bool,
        name: Ident,
        ty: Option<Ty>,
        init: Option<Expr>,
    },

    Assign {
        target: Place,
        value: Expr,
    },

    Expr(Expr),
    Semi(Expr),

    Return(Option<Expr>),
    Break(Option<Expr>),
    Continue,

    While {
        cond: Expr,
        body: Block,
    },

    For {
        mutable: bool,
        var: Ident,
        ty: Option<Ty>,
        iter: Expr,
        body: Block,
    },

    Loop {
        body: Box<Block>,
    },

    If {
        cond: Expr,
        then_block: Block,
        else_branch: Option<ElseBranch>,
    },

    Empty,
}

#[derive(Debug, Clone)]
pub enum ElseBranch {
    Block(Block),
    If {
        cond: Expr,
        then_block: Block,
        else_branch: Option<Box<ElseBranch>>,
    },
}

pub type Expr = AstNode<ExprKind>;

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(i32),

    Place(Place),

    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    Call {
        callee: Ident,
        args: Vec<Expr>,
    },

    If {
        cond: Box<Expr>,
        then_block: Box<Block>,
        else_block: Box<Block>,
    },

    Loop {
        body: Box<Block>,
    },

    Block(Box<Block>),

    Array(Vec<Expr>),
    Tuple(Vec<Expr>),

    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },

    Range {
        start: Box<Expr>,
        end: Box<Expr>,
    },

    Borrow {
        mutable: bool,
        expr: Box<Expr>,
    },
}

pub type Place = AstNode<PlaceKind>;

#[derive(Debug, Clone)]
pub enum PlaceKind {
    Local(Ident),

    Deref(Box<Expr>),

    Index { base: Box<Place>, index: Box<Expr> },

    Field { base: Box<Place>, index: usize },
}
