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
    /// 每个 struct 定义按源码声明顺序排列的字段类型。
    pub struct_field_tys: HashMap<DefId, Vec<TyId>>,
    /// 命名字段访问解析后的字段序号。
    ///
    /// HIR 会保留 `p.x` 中的字段名，typecheck 根据 `p` 的 struct 类型查出 `x`
    /// 对应的声明顺序，并把结果记录到这里。THIR/IR 后续只消费 index，不再处理名字。
    pub field_indices: HashMap<HirExprId, usize>,
}

impl TypeckResults {
    pub fn new() -> Self {
        Self {
            expr_tys: HashMap::new(),
            stmt_tys: HashMap::new(),
            local_tys: HashMap::new(),
            def_tys: HashMap::new(),
            struct_field_tys: HashMap::new(),
            field_indices: HashMap::new(),
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

    pub fn set_struct_field_tys(&mut self, def_id: DefId, fields: Vec<TyId>) {
        self.struct_field_tys.insert(def_id, fields);
    }
    pub fn get_struct_field_tys(&self, def_id: DefId) -> Option<&Vec<TyId>> {
        self.struct_field_tys.get(&def_id)
    }

    pub fn set_field_index(&mut self, expr_id: HirExprId, index: usize) {
        self.field_indices.insert(expr_id, index);
    }
    pub fn get_field_index(&self, expr_id: HirExprId) -> Option<&usize> {
        self.field_indices.get(&expr_id)
    }
}

#[derive(Debug, Clone)]
pub struct TypeckOutput {
    pub results: TypeckResults,
    pub tys: TyStore,
    pub errors: Vec<TypeError>,
}
