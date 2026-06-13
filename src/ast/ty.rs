use crate::{ast::node::AstNode, typecheck::ty::IntKind};

/*
 * 一些基本的元素
 */
pub type Ident = AstNode<String>;

pub type Ty = AstNode<TyKind>;

#[derive(Debug, Clone)]
pub enum TyKind {
    Int(IntKind),
    Bool,
    Str,
    Adt(Ident),
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
    ExternFn(ExternFnDecl),
    Struct(StructDecl),
}

pub type Item = AstNode<ItemKind>;

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub sig: FnSig,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct ExternFnDecl {
    pub sig: FnSig,
}

/// 命名结构体声明。
///
/// 当前语言子集只支持 `struct Point { x: i32 }` 形式的 named-field struct。
/// tuple struct、unit struct、泛型和 impl 会在后续阶段再扩展。
#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: Ident,
    pub fields: Vec<StructField>,
}

/// 结构体声明中的单个命名字段。
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: Ident,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub struct FnSig {
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret_ty: Option<Ty>,
    pub variadic: bool,
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
        pat: Pat,
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
    Bool(bool),
    String(String),

    Place(Place),

    StructLit {
        name: Ident,
        fields: Vec<StructLitField>,
    },

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

/// 结构体字面量中的单个字段初始化。
#[derive(Debug, Clone)]
pub struct StructLitField {
    pub name: Ident,
    pub expr: Expr,
}

pub type Pat = AstNode<PatKind>;

#[derive(Debug, Clone)]
pub enum PatKind {
    Wildcard,
    Binding {
        mutable: bool,
        name: Ident,
    },
    Tuple(Vec<Pat>),
    Struct {
        name: Ident,
        fields: Vec<StructPatField>,
    },
}

#[derive(Debug, Clone)]
pub struct StructPatField {
    pub name: Ident,
    pub pat: Pat,
}

pub type Place = AstNode<PlaceKind>;

#[derive(Debug, Clone)]
pub enum PlaceKind {
    Local(Ident),

    Deref(Box<Expr>),

    Index { base: Box<Place>, index: Box<Expr> },

    Field { base: Box<Place>, index: usize },

    NamedField { base: Box<Place>, name: Ident },
}
