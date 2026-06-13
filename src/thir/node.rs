use std::collections::HashMap;

use crate::{
    ast::ty::BinaryOp,
    hir::id::{DefId, HirExprId, LocalId},
    lexer::token::Span,
    thir::id::{ThirBodyId, ThirExprId, ThirLocalId, ThirStmtId},
    typecheck::ty::TyId,
};

#[derive(Debug, Clone)]
/// THIR 顶层结构，按函数保存已经完成名字解析和类型检查的函数体。
pub struct ThirProgram {
    pub bodies: Vec<ThirBody>,
    pub body_map: HashMap<DefId, ThirBodyId>,
}

impl ThirProgram {
    pub fn new() -> Self {
        Self {
            bodies: vec![],
            body_map: HashMap::new(),
        }
    }

    pub fn alloc_body(&mut self, owner: DefId, body: ThirBody) -> ThirBodyId {
        let id = ThirBodyId(self.bodies.len());
        self.body_map.insert(owner, id);
        self.bodies.push(body);
        id
    }

    pub fn body(&self, id: ThirBodyId) -> Option<&ThirBody> {
        self.bodies.get(id.index())
    }

    pub fn body_for_def(&self, owner: DefId) -> Option<&ThirBody> {
        let id = self.body_map.get(&owner)?;
        self.body(*id)
    }
}

#[derive(Debug, Clone)]
/// 一个函数体内的 THIR arena。
pub struct ThirBody {
    pub owner: DefId,
    pub params: Vec<ThirLocalId>,
    pub locals: Vec<ThirLocal>,
    pub stmts: Vec<ThirStmt>,
    pub exprs: Vec<ThirExpr>,
    pub value: ThirExprId,
}

impl ThirBody {
    pub fn new(owner: DefId) -> Self {
        Self {
            owner,
            params: vec![],
            locals: vec![],
            stmts: vec![],
            exprs: vec![],
            value: ThirExprId(usize::MAX),
        }
    }

    pub fn alloc_local(&mut self, local: ThirLocal) -> ThirLocalId {
        let id = ThirLocalId(self.locals.len());
        self.locals.push(local);
        id
    }

    pub fn alloc_stmt(&mut self, stmt: ThirStmt) -> ThirStmtId {
        let id = ThirStmtId(self.stmts.len());
        self.stmts.push(stmt);
        id
    }

    pub fn alloc_expr(&mut self, expr: ThirExpr) -> ThirExprId {
        let id = ThirExprId(self.exprs.len());
        self.exprs.push(expr);
        id
    }

    pub fn local(&self, id: ThirLocalId) -> Option<&ThirLocal> {
        self.locals.get(id.index())
    }

    pub fn stmt(&self, id: ThirStmtId) -> Option<&ThirStmt> {
        self.stmts.get(id.index())
    }

    pub fn expr(&self, id: ThirExprId) -> Option<&ThirExpr> {
        self.exprs.get(id.index())
    }
}

#[derive(Debug, Clone)]
pub struct ThirLocal {
    pub hir_local: Option<LocalId>,
    pub name: String,
    pub mutable: bool,
    pub ty: TyId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ThirBlock {
    pub stmts: Vec<ThirStmtId>,
    pub expr: Option<ThirExprId>,
}

#[derive(Debug, Clone)]
pub struct ThirStmt {
    pub kind: ThirStmtKind,
    pub ty: TyId,
    pub span: Span,
    pub hir_id: Option<crate::hir::id::HirStmtId>,
}

#[derive(Debug, Clone)]
pub enum ThirStmtKind {
    Let {
        pat: ThirPat,
        init: Option<ThirExprId>,
    },
    Expr(ThirExprId),
    Semi(ThirExprId),
    Empty,
}

#[derive(Debug, Clone)]
pub struct ThirPat {
    pub kind: ThirPatKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ThirPatKind {
    Wildcard,
    Binding(ThirLocalId),
    Tuple(Vec<ThirPat>),
    Struct {
        def_id: DefId,
        fields: Vec<(usize, ThirPat)>,
    },
}

#[derive(Debug, Clone)]
pub struct ThirExpr {
    pub kind: ThirExprKind,
    pub ty: TyId,
    pub span: Span,
    pub hir_id: Option<HirExprId>,
}

impl ThirExpr {
    pub fn new(kind: ThirExprKind, ty: TyId, span: Span, hir_id: Option<HirExprId>) -> Self {
        Self {
            kind,
            ty,
            span,
            hir_id,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ThirExprKind {
    Int(i32),
    Bool(bool),
    String(String),
    StructLit {
        def_id: DefId,
        fields: Vec<(usize, ThirExprId)>,
    },
    Use(ThirPlace),
    Binary {
        op: BinaryOp,
        lhs: ThirExprId,
        rhs: ThirExprId,
    },
    Call {
        callee: DefId,
        args: Vec<ThirExprId>,
    },
    Assign {
        target: ThirPlace,
        value: ThirExprId,
    },
    Block(ThirBlock),
    If {
        cond: ThirExprId,
        then_expr: ThirExprId,
        else_expr: Option<ThirExprId>,
    },
    While {
        cond: ThirExprId,
        body: ThirBlock,
    },
    Loop {
        body: ThirBlock,
    },
    ForRange {
        local: ThirLocalId,
        start: ThirExprId,
        end: ThirExprId,
        body: ThirBlock,
    },
    Return(Option<ThirExprId>),
    Break(Option<ThirExprId>),
    Continue,
    Borrow {
        mutable: bool,
        expr: ThirExprId,
    },
    DerefValue(ThirExprId),
    IndexValue {
        base: ThirExprId,
        index: ThirExprId,
    },
    FieldValue {
        base: ThirExprId,
        index: usize,
    },
    Array(Vec<ThirExprId>),
    Tuple(Vec<ThirExprId>),
    Range {
        start: ThirExprId,
        end: ThirExprId,
    },
}

#[derive(Debug, Clone)]
pub struct ThirPlace {
    pub kind: ThirPlaceKind,
    pub ty: TyId,
    pub span: Span,
    pub hir_id: Option<HirExprId>,
}

impl ThirPlace {
    pub fn new(kind: ThirPlaceKind, ty: TyId, span: Span, hir_id: Option<HirExprId>) -> Self {
        Self {
            kind,
            ty,
            span,
            hir_id,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ThirPlaceKind {
    Local(ThirLocalId),
    Deref {
        base: ThirExprId,
    },
    Index {
        base: Box<ThirPlace>,
        index: ThirExprId,
    },
    Field {
        base: Box<ThirPlace>,
        index: usize,
    },
}
