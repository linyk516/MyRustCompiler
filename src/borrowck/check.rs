use crate::{
    borrowck::{
        error::{BorrowError, BorrowErrorKind, BorrowKind},
        output::BorrowckOutput,
    },
    hir::{
        id::{HirExprId, LocalId},
        node::{HirBlock, HirExprKind, HirItemKind, HirPat, HirPatKind, HirProgram, HirStmtKind},
        res::Res,
        table::LocalTable,
    },
    lexer::token::Span,
};

#[derive(Debug, Clone)]
struct ActiveBorrow {
    local_id: LocalId,
    kind: BorrowKind,
    span: Span,
}

/// 轻量级借用检查器。
///
/// 当前版本只做词法作用域内的别名安全检查：
/// - 一个局部变量存在活跃 `&mut` 时，不能再创建任何 `&` 或 `&mut`。
/// - 一个局部变量存在任意活跃借用时，不能修改这个局部变量。
///
/// 这不是完整 Rust NLL。它故意以 `LocalId` 为根做保守检查，适合当前教学编译器
/// 在 typecheck 之后、THIR lowering 之前拦截最直接的别名错误。
pub struct BorrowCheckCtx<'hir> {
    hir: &'hir HirProgram,
    locals: &'hir LocalTable,
    scopes: Vec<Vec<ActiveBorrow>>,
    errors: Vec<BorrowError>,
}

impl<'hir> BorrowCheckCtx<'hir> {
    pub fn new(hir: &'hir HirProgram, locals: &'hir LocalTable) -> Self {
        Self {
            hir,
            locals,
            scopes: vec![],
            errors: vec![],
        }
    }

    /// 检查整个 HIR 程序，并返回本阶段产生的错误序列。
    pub fn check_program(mut self) -> BorrowckOutput {
        for item_id in &self.hir.root_items {
            let Some(item) = self.hir.item(*item_id) else {
                continue;
            };
            if let HirItemKind::Fn(function) = &item.kind {
                self.scopes.clear();
                self.push_scope();
                if let Some(body) = self.hir.body(function.body) {
                    self.check_expr(body.value);
                }
                self.pop_scope();
            }
        }

        BorrowckOutput {
            errors: self.errors,
        }
    }

    fn check_block(&mut self, block: &HirBlock) {
        self.push_scope();
        for stmt_id in &block.stmts {
            let Some(stmt) = self.hir.stmt(*stmt_id) else {
                continue;
            };
            match &stmt.kind {
                HirStmtKind::Let { pat, init, .. } => {
                    if let Some(init) = init {
                        self.check_expr_as_stored_value(*init);
                    }
                    self.check_pat(pat);
                }
                HirStmtKind::Expr(expr) | HirStmtKind::Semi(expr) => {
                    self.check_expr_as_temporary(*expr);
                }
                HirStmtKind::Empty => {}
            }
        }
        if let Some(expr) = block.expr {
            self.check_expr(expr);
        }
        self.pop_scope();
    }

    fn check_pat(&mut self, pat: &HirPat) {
        match &pat.kind {
            HirPatKind::Wildcard | HirPatKind::Binding { .. } => {}
            HirPatKind::Tuple(elems) => {
                for elem in elems {
                    self.check_pat(elem);
                }
            }
            HirPatKind::Struct { fields, .. } => {
                for field in fields {
                    self.check_pat(&field.pat);
                }
            }
        }
    }

    fn check_expr_as_temporary(&mut self, expr: HirExprId) {
        self.push_scope();
        self.check_expr(expr);
        self.pop_scope();
    }

    fn check_expr_as_stored_value(&mut self, expr: HirExprId) {
        let Some(expr_data) = self.hir.expr(expr) else {
            return;
        };
        if let HirExprKind::Borrow { mutable, expr } = expr_data.kind {
            self.check_borrow_expr(mutable, expr, expr_data.span.clone());
        } else {
            self.check_expr_as_temporary(expr);
        }
    }

    fn check_expr(&mut self, expr: HirExprId) {
        let Some(expr_data) = self.hir.expr(expr) else {
            return;
        };
        match &expr_data.kind {
            HirExprKind::Int(_) | HirExprKind::Bool(_) | HirExprKind::String(_) => {}
            HirExprKind::Path(_) | HirExprKind::Err => {}
            HirExprKind::StructLit { fields, .. } => {
                for field in fields {
                    self.check_expr(field.expr);
                }
            }
            HirExprKind::Binary { lhs, rhs, .. } => {
                self.check_expr(*lhs);
                self.check_expr(*rhs);
            }
            HirExprKind::Call { args, .. } => {
                self.push_scope();
                for arg in args {
                    self.check_expr(*arg);
                }
                self.pop_scope();
            }
            HirExprKind::Assign { lhs, rhs } => {
                self.check_assignment(*lhs, *rhs);
            }
            HirExprKind::Block(block) => {
                self.check_block(block);
            }
            HirExprKind::If {
                cond,
                then_block,
                else_expr,
            } => {
                self.check_expr_as_temporary(*cond);
                self.check_block(then_block);
                if let Some(else_expr) = else_expr {
                    self.check_expr(*else_expr);
                }
            }
            HirExprKind::While { cond, body } => {
                self.check_expr_as_temporary(*cond);
                self.check_block(body);
            }
            HirExprKind::Loop { body } => {
                self.check_block(body);
            }
            HirExprKind::ForRange {
                start, end, body, ..
            } => {
                self.check_expr_as_temporary(*start);
                self.check_expr_as_temporary(*end);
                self.check_block(body);
            }
            HirExprKind::Return(value) | HirExprKind::Break(value) => {
                if let Some(value) = value {
                    self.check_expr_as_temporary(*value);
                }
            }
            HirExprKind::Continue => {}
            HirExprKind::Borrow { mutable, expr } => {
                self.check_borrow_expr(*mutable, *expr, expr_data.span.clone());
            }
            HirExprKind::Deref(base) => {
                self.check_expr(*base);
            }
            HirExprKind::Index { base, index } => {
                self.check_expr(*base);
                self.check_expr_as_temporary(*index);
            }
            HirExprKind::Field { base, .. } | HirExprKind::NamedField { base, .. } => {
                self.check_expr(*base);
            }
            HirExprKind::Array(elems) | HirExprKind::Tuple(elems) => {
                for elem in elems {
                    self.check_expr(*elem);
                }
            }
            HirExprKind::Range { start, end } => {
                self.check_expr_as_temporary(*start);
                self.check_expr_as_temporary(*end);
            }
        }
    }

    fn check_assignment(&mut self, lhs: HirExprId, rhs: HirExprId) {
        if let Some(local_id) = self.place_root_local(lhs) {
            self.check_mutation(local_id, self.expr_span(lhs));
        }
        self.check_expr(lhs);
        self.check_expr_as_stored_value(rhs);
    }

    fn check_borrow_expr(&mut self, mutable: bool, expr: HirExprId, span: Span) {
        self.check_expr(expr);
        let Some(local_id) = self.place_root_local(expr) else {
            return;
        };
        let kind = if mutable {
            BorrowKind::Mutable
        } else {
            BorrowKind::Shared
        };
        self.register_borrow(local_id, kind, span);
    }

    fn register_borrow(&mut self, local_id: LocalId, kind: BorrowKind, span: Span) {
        if let Some(existing) = self.find_conflicting_borrow(local_id, kind) {
            let local_name = self.local_name(local_id);
            self.errors.push(BorrowError::new(
                BorrowErrorKind::ConflictingBorrow {
                    local_id,
                    local_name,
                    requested: kind,
                    existing: existing.kind,
                    existing_span: existing.span,
                },
                span,
            ));
            return;
        }

        if let Some(scope) = self.scopes.last_mut() {
            scope.push(ActiveBorrow {
                local_id,
                kind,
                span,
            });
        }
    }

    fn check_mutation(&mut self, local_id: LocalId, span: Span) {
        if let Some(existing) = self.find_active_borrow(local_id) {
            let local_name = self.local_name(local_id);
            self.errors.push(BorrowError::new(
                BorrowErrorKind::MutationWhileBorrowed {
                    local_id,
                    local_name,
                    existing: existing.kind,
                    existing_span: existing.span,
                },
                span,
            ));
        }
    }

    fn find_conflicting_borrow(
        &self,
        local_id: LocalId,
        requested: BorrowKind,
    ) -> Option<ActiveBorrow> {
        self.active_borrows(local_id)
            .find(|borrow| match requested {
                BorrowKind::Shared => borrow.kind == BorrowKind::Mutable,
                BorrowKind::Mutable => true,
            })
    }

    fn find_active_borrow(&self, local_id: LocalId) -> Option<ActiveBorrow> {
        self.active_borrows(local_id).next()
    }

    fn active_borrows(&self, local_id: LocalId) -> impl Iterator<Item = ActiveBorrow> + '_ {
        self.scopes
            .iter()
            .rev()
            .flat_map(|scope| scope.iter().rev())
            .filter(move |borrow| borrow.local_id == local_id)
            .cloned()
    }

    fn place_root_local(&self, expr: HirExprId) -> Option<LocalId> {
        let expr_data = self.hir.expr(expr)?;
        match &expr_data.kind {
            HirExprKind::Path(Res::Local(local_id)) => Some(*local_id),
            HirExprKind::Field { base, .. } | HirExprKind::NamedField { base, .. } => {
                self.place_root_local(*base)
            }
            HirExprKind::Index { base, .. } => self.place_root_local(*base),
            HirExprKind::Deref(_) => None,
            _ => None,
        }
    }

    fn expr_span(&self, expr: HirExprId) -> Span {
        self.hir
            .expr(expr)
            .map(|expr| expr.span.clone())
            .unwrap_or(Span { start: 0, end: 0 })
    }

    fn local_name(&self, local_id: LocalId) -> String {
        self.locals
            .get(local_id)
            .map(|local| local.name.clone())
            .unwrap_or_else(|| format!("{local_id:?}"))
    }

    fn push_scope(&mut self) {
        self.scopes.push(vec![]);
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}
