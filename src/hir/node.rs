use crate::{
    ast::ty::BinaryOp,
    hir::{
        id::{DefId, HirBodyId, HirExprId, HirItemId, HirStmtId, LocalId},
        res::Res,
        ty::HirTy,
    },
    lexer::token::Span,
};

#[derive(Debug, Clone)]
/// Hir顶层节点，持有Hir树状结构中的所有节点
pub struct HirProgram {
    pub root_items: Vec<HirItemId>,
    pub items: Vec<HirItem>,
    pub bodies: Vec<HirBody>,
    pub exprs: Vec<HirExpr>,
    pub stmts: Vec<HirStmt>,
}

impl HirProgram {
    pub fn new() -> Self {
        Self {
            root_items: vec![],
            items: vec![],
            bodies: vec![],
            exprs: vec![],
            stmts: vec![],
        }
    }

    pub fn alloc_item(&mut self, item: HirItem) -> HirItemId {
        let id = self.items.len().into();
        self.items.push(item);
        id
    }

    pub fn alloc_body(&mut self, item: HirBody) -> HirBodyId {
        let id = self.bodies.len().into();
        self.bodies.push(item);
        id
    }

    pub fn alloc_expr(&mut self, item: HirExpr) -> HirExprId {
        let id = self.exprs.len().into();
        self.exprs.push(item);
        id
    }

    pub fn alloc_stmt(&mut self, item: HirStmt) -> HirStmtId {
        let id = self.stmts.len().into();
        self.stmts.push(item);
        id
    }

    pub fn item(&self, id: HirItemId) -> Option<&HirItem> {
        self.items.get(id.index())
    }

    pub fn body(&self, id: HirBodyId) -> Option<&HirBody> {
        self.bodies.get(id.index())
    }

    pub fn expr(&self, id: HirExprId) -> Option<&HirExpr> {
        self.exprs.get(id.index())
    }

    pub fn stmt(&self, id: HirStmtId) -> Option<&HirStmt> {
        self.stmts.get(id.index())
    }
}

#[derive(Debug, Clone)]
pub struct HirItem {
    pub def_id: DefId,
    pub span: Span,
    pub kind: HirItemKind,
}

#[derive(Debug, Clone)]
pub enum HirItemKind {
    Fn(HirFn),
    ExternFn(HirExternFn),
    Struct(HirStruct),
}

#[derive(Debug, Clone)]
pub struct HirFn {
    pub name: String,
    pub sig: HirFnSig,
    pub body: HirBodyId,
}

#[derive(Debug, Clone)]
pub struct HirExternFn {
    pub name: String,
    pub sig: HirFnSig,
}

#[derive(Debug, Clone)]
/// HIR 中的 named-field struct item。
pub struct HirStruct {
    pub name: String,
    pub fields: Vec<HirStructField>,
}

#[derive(Debug, Clone)]
pub struct HirStructField {
    pub name: String,
    pub ty: HirTy,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirFnSig {
    pub params: Vec<HirParam>,
    pub ret_ty: HirTy,
    pub variadic: bool,
}

#[derive(Debug, Clone)]
pub struct HirParam {
    pub local_id: LocalId,
    pub name: String,
    pub mutable: bool,
    pub ty: HirTy,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirBody {
    pub owner: DefId,
    pub params: Vec<LocalId>,
    pub value: HirExprId,
}

#[derive(Debug, Clone)]
pub struct HirBlock {
    pub stmts: Vec<HirStmtId>,
    pub expr: Option<HirExprId>,
}

#[derive(Debug, Clone)]
pub struct HirStmt {
    pub span: Span,
    pub kind: HirStmtKind,
}

#[derive(Debug, Clone)]
pub enum HirStmtKind {
    Let {
        pat: HirPat,
        ty: Option<HirTy>,
        init: Option<HirExprId>,
    },

    Expr(HirExprId),

    Semi(HirExprId),

    Empty,
}

#[derive(Debug, Clone)]
pub struct HirPat {
    pub span: Span,
    pub kind: HirPatKind,
}

#[derive(Debug, Clone)]
pub enum HirPatKind {
    Wildcard,
    Binding {
        local_id: LocalId,
        name: String,
        mutable: bool,
    },
    Tuple(Vec<HirPat>),
    Struct {
        def_id: DefId,
        fields: Vec<HirStructPatField>,
    },
}

#[derive(Debug, Clone)]
pub struct HirStructPatField {
    pub name: String,
    pub index: usize,
    pub pat: HirPat,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirExpr {
    pub span: Span,
    pub kind: HirExprKind,
}

#[derive(Debug, Clone)]
pub enum HirExprKind {
    Int(i32),
    Bool(bool),
    String(String),

    Path(Res),

    StructLit {
        def_id: DefId,
        fields: Vec<HirStructLitField>,
    },

    Binary {
        op: BinaryOp,
        lhs: HirExprId,
        rhs: HirExprId,
    },

    Call {
        callee: Res,
        args: Vec<HirExprId>,
    },

    Assign {
        lhs: HirExprId,
        rhs: HirExprId,
    },

    Block(HirBlock),

    If {
        cond: HirExprId,
        then_block: HirBlock,
        else_expr: Option<HirExprId>,
    },

    While {
        cond: HirExprId,
        body: HirBlock,
    },

    Loop {
        body: HirBlock,
    },

    /// 目前的设计中没有trait, match, option等，无法和rust标准设计一样进行desugar，因此暂时使用专门的范围for类型
    ForRange {
        local_id: LocalId,
        name: String,
        mutable: bool,
        ty: Option<HirTy>,
        start: HirExprId,
        end: HirExprId,
        body: HirBlock,
    },

    /// 数组/表达式形式的 for 循环。
    ///
    /// 当前类型检查只接受数组类型作为 `iter`，循环变量类型由数组元素类型决定。
    ForIter {
        local_id: LocalId,
        name: String,
        mutable: bool,
        ty: Option<HirTy>,
        iter: HirExprId,
        body: HirBlock,
    },

    Return(Option<HirExprId>),

    Break(Option<HirExprId>),

    Continue,

    Borrow {
        mutable: bool,
        expr: HirExprId,
    },

    Deref(HirExprId),

    Index {
        base: HirExprId,
        index: HirExprId,
    },

    Field {
        base: HirExprId,
        index: usize,
    },

    NamedField {
        base: HirExprId,
        name: String,
    },

    Array(Vec<HirExprId>),

    Tuple(Vec<HirExprId>),

    Range {
        start: HirExprId,
        end: HirExprId,
    },

    Err,
}

#[derive(Debug, Clone)]
pub struct HirStructLitField {
    pub name: String,
    pub expr: HirExprId,
    pub span: Span,
}

impl HirExpr {
    pub fn new(kind: HirExprKind, span: Span) -> Self {
        Self { span, kind }
    }

    pub fn err(span: Span) -> Self {
        Self {
            span,
            kind: HirExprKind::Err,
        }
    }
}

impl HirStmt {
    pub fn new(kind: HirStmtKind, span: Span) -> Self {
        Self { span, kind }
    }
}
