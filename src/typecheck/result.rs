use std::collections::HashMap;

use crate::{
    hir::id::{DefId, HirExprId, HirStmtId, LocalId},
    typecheck::{
        error::TypeError,
        ty::{TyId, TyStore},
    },
};

#[derive(Debug, Clone)]
pub struct TypeckResults {
    pub expr_tys: HashMap<HirExprId, TyId>,
    pub stmt_tys: HashMap<HirStmtId, TyId>,
    pub local_tys: HashMap<LocalId, TyId>,
    pub def_tys: HashMap<DefId, TyId>,
}

impl TypeckResults {
    pub fn new() -> Self {
        Self {
            expr_tys: HashMap::new(),
            stmt_tys: HashMap::new(),
            local_tys: HashMap::new(),
            def_tys: HashMap::new(),
        }
    }

    pub fn set_expr_ty(&mut self, expr_id: HirExprId, ty: TyId) {
        self.expr_tys.insert(expr_id, ty);
    }
    pub fn get_expr_ty(&self, expr_id: HirExprId) -> Option<&TyId> {
        self.expr_tys.get(&expr_id)
    }

    pub fn set_stmt_ty(&mut self, stmt_id: HirStmtId, ty: TyId) {
        self.stmt_tys.insert(stmt_id, ty);
    }
    pub fn get_stmt_ty(&self, stmt_id: HirStmtId) -> Option<&TyId> {
        self.stmt_tys.get(&stmt_id)
    }

    pub fn set_local_ty(&mut self, local_id: LocalId, ty: TyId) {
        self.local_tys.insert(local_id, ty);
    }
    pub fn get_local_ty(&self, local_id: LocalId) -> Option<&TyId> {
        self.local_tys.get(&local_id)
    }

    pub fn set_def_ty(&mut self, def_id: DefId, ty: TyId) {
        self.def_tys.insert(def_id, ty);
    }
    pub fn get_def_ty(&self, def_id: DefId) -> Option<&TyId> {
        self.def_tys.get(&def_id)
    }
}

#[derive(Debug, Clone)]
pub struct TypeckOutput {
    pub results: TypeckResults,
    pub tys: TyStore,
    pub errors: Vec<TypeError>,
}
