use std::collections::HashSet;

use crate::{
    ast::ty::BinaryOp,
    hir::{
        id::{DefId, HirBodyId, HirExprId, HirItemId, LocalId},
        node::{
            HirBlock, HirExprKind, HirFnSig, HirItemKind, HirPat, HirPatKind, HirProgram,
            HirStmtKind, HirStructLitField,
        },
        res::Res,
        table::{DefKind, DefTable, LocalTable},
        ty::{HirTy, HirTyKind},
    },
    lexer::token::Span,
    typecheck::{
        error::{TypeError, TypeErrorKind},
        infer::InferCtx,
        result::{TypeckOutput, TypeckResults},
        ty::{TyId, TyKind, TyStore},
    },
};

/// HIR 类型检查上下文。
///
/// `TypeckCtx` 是类型检查阶段的执行器：它读取已经完成名字解析的 HIR，
/// 为函数、局部变量、语句和表达式生成类型约束，调用 `InferCtx` 统一这些约束，
/// 并把最终结果写入 `TypeckResults` 旁表。它不修改 HIR 本身。
pub struct TypeckCtx<'hir> {
    hir: &'hir HirProgram,
    defs: &'hir DefTable,
    locals: &'hir LocalTable,

    tys: TyStore,
    infer: InferCtx,
    results: TypeckResults,
    errors: Vec<TypeError>,

    current_ret_ty: Option<TyId>,
    loop_break_tys: Vec<TyId>,
    initialized_locals: HashSet<LocalId>,
}

impl<'hir> TypeckCtx<'hir> {
    /// 创建一个新的类型检查上下文。
    ///
    /// 这个函数只保存 HIR、定义表和局部变量表的引用，并初始化类型仓库、
    /// 类型推导上下文、结果旁表和错误列表。真正的检查从 `check_program` 开始。
    pub fn new(hir: &'hir HirProgram, defs: &'hir DefTable, locals: &'hir LocalTable) -> Self {
        Self {
            hir,
            defs,
            locals,
            tys: TyStore::new(),
            infer: InferCtx::new(),
            results: TypeckResults::new(),
            errors: vec![],
            current_ret_ty: None,
            loop_break_tys: vec![],
            initialized_locals: HashSet::new(),
        }
    }

    /// 检查整个 HIR 程序，并返回类型检查输出。
    ///
    /// 入口流程分成三步：
    ///
    /// 1. 先收集所有顶层定义的类型，让函数体中可以调用后面定义的函数。
    /// 2. 再检查每个顶层 item 的函数体，并逐步填充表达式、语句和局部变量类型。
    /// 3. 最后递归解析仍然可以确定的推导变量，把旁表中的类型尽量变成最终类型。
    pub fn check_program(mut self) -> TypeckOutput {
        self.collect_def_tys();
        for &root_item in &self.hir.root_items {
            self.check_item(root_item);
        }
        self.resolve_result_tys();
        TypeckOutput {
            results: self.results,
            tys: self.tys,
            errors: self.errors,
        }
    }

    /// 收集所有顶层定义的类型。
    ///
    /// 当前语言有普通函数和外部函数 item，所以这一步主要是把每个函数签名转成
    /// `TyKind::Fn { params, ret, variadic }`，写入 `TypeckResults::def_tys`。
    fn collect_def_tys(&mut self) {
        for &root_item in &self.hir.root_items {
            self.collect_item_ty(root_item);
        }
    }

    /// 收集单个顶层 item 的类型。
    ///
    /// 该函数只处理签名，不检查函数体。这样可以让函数之间互相调用，
    /// 只要名字解析阶段已经把 callee 解析成了 `DefId`。
    fn collect_item_ty(&mut self, item: HirItemId) {
        let Some(item) = self.hir.item(item) else {
            self.emit_internal("Missing item when collecting definition type!");
            return;
        };

        match &item.kind {
            HirItemKind::Fn(hir_fn) => {
                let fn_ty = self.collect_fn_ty(&hir_fn.sig);
                self.results.set_def_ty(item.def_id, fn_ty);
            }
            HirItemKind::ExternFn(hir_fn) => {
                let fn_ty = self.collect_fn_ty(&hir_fn.sig);
                self.results.set_def_ty(item.def_id, fn_ty);
            }
            HirItemKind::Struct(_) => {
                let fields = self
                    .defs
                    .get(item.def_id)
                    .map(|def| def.struct_fields.clone())
                    .unwrap_or_default()
                    .iter()
                    .map(|field| self.lower_hir_ty(&field.ty))
                    .collect();
                self.results.set_struct_field_tys(item.def_id, fields);
                let struct_ty = self.tys.intern(TyKind::Adt(item.def_id));
                self.results.set_def_ty(item.def_id, struct_ty);
            }
        }
    }

    /// 把函数签名转换成语义类型。
    ///
    /// 参数和返回值在 HIR 中仍然是语法类型 `HirTy`；类型检查阶段需要先把它们
    /// 转换为 `TyStore` 中的 `TyId`，再组合成函数类型。
    fn collect_fn_ty(&mut self, sig: &HirFnSig) -> TyId {
        let params = sig
            .params
            .iter()
            .map(|param| self.lower_hir_ty(&param.ty))
            .collect();
        let ret = self.lower_hir_ty(&sig.ret_ty);

        self.tys.intern(TyKind::Fn {
            params,
            ret,
            variadic: sig.variadic,
        })
    }

    /// 把 HIR 类型语法转换成类型检查阶段使用的语义类型。
    ///
    /// HIR 类型描述的是用户写出来的类型结构；`TyKind` 描述的是编译器后续阶段
    /// 统一使用的类型表示。这里不做推导，只做结构转换。
    fn lower_hir_ty(&mut self, ty: &HirTy) -> TyId {
        let ty_kind = match &ty.kind {
            HirTyKind::Int(kind) => TyKind::Int(*kind),
            HirTyKind::Bool => TyKind::Bool,
            HirTyKind::Str => TyKind::Str,
            HirTyKind::Adt(def_id) => TyKind::Adt(*def_id),
            HirTyKind::Ref { mutable, inner } => TyKind::Ref {
                mutable: *mutable,
                inner: self.lower_hir_ty(inner),
            },
            HirTyKind::Array { elem, len } => TyKind::Array {
                elem: self.lower_hir_ty(elem),
                len: {
                    if *len == 0 {
                        self.emit_error(
                            TypeErrorKind::InvalidArrayLength { len: *len },
                            ty.span.clone(),
                        );
                    }
                    *len
                },
            },
            HirTyKind::Tuple(ty_list) => {
                TyKind::Tuple(ty_list.iter().map(|ty| self.lower_hir_ty(ty)).collect())
            }
            HirTyKind::Unit => TyKind::Unit,
            HirTyKind::Err => TyKind::Error,
        };

        self.tys.intern(ty_kind)
    }

    /// 检查单个顶层 item。
    ///
    /// 目前只支持函数，因此它会取出函数签名和函数体，交给 `check_fn`。
    fn check_item(&mut self, item: HirItemId) {
        let Some(item) = self.hir.item(item).cloned() else {
            self.emit_internal("Missing item when checking type!");
            return;
        };

        match item.kind {
            HirItemKind::Fn(fn_item) => self.check_fn(item.def_id, fn_item.body, &fn_item.sig),
            HirItemKind::ExternFn(_) => {}
            HirItemKind::Struct(_) => {}
        }
    }

    /// 检查函数体，并把参数类型写入局部变量类型旁表。
    ///
    /// 函数签名已经在收集阶段变成 `TyKind::Fn`。这里取出参数类型和返回类型，
    /// 先给每个参数对应的 `LocalId` 记录类型，再使用返回类型检查函数体。
    fn check_fn(&mut self, def_id: DefId, body: HirBodyId, sig: &HirFnSig) {
        let fn_ty_id = self.results.get_def_ty(def_id).copied().unwrap_or_else(|| {
            self.emit_internal("Function definition type not collected before checking body!");
            self.tys.error()
        });

        let fn_ty = self.tys.kind(fn_ty_id);
        let (param_tys, ret_ty) = match fn_ty {
            TyKind::Fn { params, ret, .. } => (params.clone(), *ret),
            _ => {
                self.emit_internal("Expected function type, got something else!");
                return;
            }
        };

        let original_ret_ty = self.current_ret_ty;
        self.current_ret_ty = Some(ret_ty);

        for (param, param_ty) in sig.params.iter().zip(param_tys) {
            self.results.set_local_ty(param.local_id, param_ty);
            self.mark_local_initialized(param.local_id);
        }

        self.check_body(body, ret_ty);
        self.current_ret_ty = original_ret_ty;
    }

    /// 检查一个函数体表达式是否满足函数返回类型。
    ///
    /// HIR 中函数体是一个表达式，正常情况下是 `HirExprKind::Block`。该函数先检查
    /// 函数体表达式本身，再把它的最终类型和函数签名中的返回类型统一；若无法统一，
    /// 报告 `ReturnTypeMismatch`。
    fn check_body(&mut self, body: HirBodyId, expected_ret: TyId) {
        let Some(body) = self.hir.body(body) else {
            self.emit_internal("Body not found!");
            return;
        };

        let value_span = self.expr_span(body.value);
        let actual = self.check_expr(body.value, Some(expected_ret));
        self.expect_return_ty(expected_ret, actual, value_span);
    }

    /// 检查一个 block，并返回 block 的类型。
    ///
    /// block 的类型由尾表达式决定；没有尾表达式时通常类型为 `Unit`。
    ///
    /// 如果没有尾表达式，但最后一条语句是 `return`、`break`、`continue` 这类发散
    /// 语句，则 block 本身也不会正常产生值，类型应保留为 `Never`。这样
    /// `fn f() -> i32 { return 1; }` 可以通过检查，因为 `!` 能统一到任意返回类型。
    fn check_block(&mut self, block: &HirBlock, expected: Option<TyId>) -> TyId {
        let mut last_stmt_ty = None;
        for &stmt in &block.stmts {
            last_stmt_ty = Some(self.check_stmt(stmt));
        }

        match block.expr {
            Some(expr) => self.check_expr(expr, expected),
            None if last_stmt_ty.map(|ty| self.is_never_ty(ty)).unwrap_or(false) => {
                self.tys.never()
            }
            None => self.tys.unit(),
        }
    }

    /// 检查一条 HIR 语句，并返回该语句类型。
    ///
    /// 大多数语句的类型都是 `Unit`。`let` 语句会根据显式类型和初始化表达式推导
    /// 局部变量类型；分号语句会检查内部表达式。
    ///
    /// 分号通常会丢弃表达式值，因此 `x + 1;` 的语句类型是 `Unit`。但是如果内部
    /// 表达式是 `return`、`break`、`continue` 这种 `Never` 表达式，语句也应保持
    /// `Never`，用于向外层 block 传播“不会正常继续执行”的信息。
    fn check_stmt(&mut self, stmt: crate::hir::id::HirStmtId) -> TyId {
        let Some(stmt_data) = self.hir.stmt(stmt).cloned() else {
            self.emit_internal("Statement not found!");
            return self.tys.error();
        };

        let ty = match stmt_data.kind {
            HirStmtKind::Let { pat, ty, init, .. } => {
                self.check_let_stmt(&pat, ty.as_ref(), init, stmt_data.span)
            }
            HirStmtKind::Expr(expr) => self.check_expr(expr, None),
            HirStmtKind::Semi(expr) => {
                let expected_unit = self.tys.unit();
                let expr_ty = self.check_expr(expr, Some(expected_unit));
                if self.is_never_ty(expr_ty) {
                    expr_ty
                } else {
                    expected_unit
                }
            }
            HirStmtKind::Empty => self.tys.unit(),
        };

        self.results.set_stmt_ty(stmt, ty);
        ty
    }

    /// 检查 `let` 语句，并确定局部变量类型。
    ///
    /// 如果有显式类型，就以显式类型为准；如果没有显式类型，则创建一个推导变量。
    /// 存在初始化表达式时，会把变量类型和初始化表达式类型统一。
    fn check_let_stmt(
        &mut self,
        pat: &HirPat,
        explicit_ty: Option<&HirTy>,
        init: Option<HirExprId>,
        span: Span,
    ) -> TyId {
        let declared_ty = explicit_ty
            .map(|ty| self.lower_hir_ty(ty))
            .unwrap_or_else(|| self.infer.new_ty_var(&mut self.tys));

        let final_ty = match init {
            Some(init) => {
                let init_ty = self.check_expr(init, Some(declared_ty));
                self.unify_at(declared_ty, init_ty, span)
            }
            None => declared_ty,
        };

        self.check_pat(pat, final_ty);
        if init.is_some() {
            self.mark_pat_initialized(pat);
        }
        self.tys.unit()
    }

    /// 用一个已知类型检查 pattern，并为其中所有绑定写入局部变量类型。
    ///
    /// 例如 `let (a, b) = pair` 会先确定 `pair` 是一个二元组，然后把第 0 个字段类型
    /// 写给 `a`，第 1 个字段类型写给 `b`。
    fn check_pat(&mut self, pat: &HirPat, expected: TyId) {
        let expected = self.infer.resolve_ty(&self.tys, expected);
        match &pat.kind {
            HirPatKind::Wildcard => {}
            HirPatKind::Binding { local_id, .. } => {
                self.results.set_local_ty(*local_id, expected);
            }
            HirPatKind::Tuple(elems) => match self.tys.kind(expected).clone() {
                TyKind::Tuple(field_tys) if field_tys.len() == elems.len() => {
                    for (elem, field_ty) in elems.iter().zip(field_tys) {
                        self.check_pat(elem, field_ty);
                    }
                }
                TyKind::Error => {}
                _ => {
                    let expected_tuple = self.tuple_pat_expected_ty(elems.len());
                    self.emit_error(
                        TypeErrorKind::MismatchedTypes {
                            expected: expected_tuple,
                            actual: expected,
                        },
                        pat.span.clone(),
                    );
                }
            },
            HirPatKind::Struct { def_id, fields } => match self.tys.kind(expected).clone() {
                TyKind::Adt(actual_def) if actual_def == *def_id => {
                    for field in fields {
                        match self
                            .results
                            .get_struct_field_tys(*def_id)
                            .and_then(|field_tys| field_tys.get(field.index))
                            .copied()
                        {
                            Some(field_ty) => self.check_pat(&field.pat, field_ty),
                            None => self.emit_error(
                                TypeErrorKind::UnknownField {
                                    base: expected,
                                    field: field.name.clone(),
                                },
                                field.span.clone(),
                            ),
                        }
                    }
                }
                TyKind::Error => {}
                _ => {
                    let expected_struct = self.tys.intern(TyKind::Adt(*def_id));
                    self.emit_error(
                        TypeErrorKind::MismatchedTypes {
                            expected: expected_struct,
                            actual: expected,
                        },
                        pat.span.clone(),
                    );
                }
            },
        }
    }

    fn tuple_pat_expected_ty(&mut self, len: usize) -> TyId {
        let elems = (0..len)
            .map(|_| self.infer.new_ty_var(&mut self.tys))
            .collect();
        self.tys.intern(TyKind::Tuple(elems))
    }

    /// 检查表达式，并返回表达式类型。
    ///
    /// `expected` 是外层上下文给出的期望类型。当前实现不会在所有表达式末尾自动
    /// 使用 `expected` 做统一，而是在需要上下文信息的结构中使用它，例如函数体尾
    /// 表达式中的 `if` 缺少 `else` 时需要知道外部是否真的需要一个值。
    fn check_expr(&mut self, expr: HirExprId, expected: Option<TyId>) -> TyId {
        let Some(expr_data) = self.hir.expr(expr).cloned() else {
            self.emit_internal("Expression not found!");
            return self.tys.error();
        };

        let span = expr_data.span.clone();
        let ty = match expr_data.kind {
            HirExprKind::Int(_) => self.integer_literal_ty(expected),
            HirExprKind::Bool(_) => self.tys.bool(),
            HirExprKind::String(_) => self.tys.str(),
            HirExprKind::Path(res) => self.check_res(res, span.clone()),
            HirExprKind::StructLit { def_id, fields } => {
                self.check_struct_lit_expr(def_id, &fields, span)
            }
            HirExprKind::Binary { op, lhs, rhs } => self.check_binary_expr(op, lhs, rhs, span),
            HirExprKind::Call { callee, args } => {
                self.check_call_expr(callee, &args, expected, span)
            }
            HirExprKind::Assign { lhs, rhs } => self.check_assign_expr(lhs, rhs, span),
            HirExprKind::Block(block) => self.check_block(&block, expected),
            HirExprKind::If {
                cond,
                then_block,
                else_expr,
            } => self.check_if_expr(cond, &then_block, else_expr, expected, span),
            HirExprKind::While { cond, body } => self.check_while_expr(cond, &body),
            HirExprKind::Loop { body } => self.check_loop_expr(&body, expected),
            HirExprKind::ForRange {
                local_id,
                ty,
                start,
                end,
                body,
                ..
            } => self.check_for_range_expr(local_id, ty.as_ref(), start, end, &body, span),
            HirExprKind::ForIter {
                local_id,
                ty,
                iter,
                body,
                ..
            } => self.check_for_iter_expr(local_id, ty.as_ref(), iter, &body, span),
            HirExprKind::Return(value) => self.check_return_expr(value, span),
            HirExprKind::Break(value) => self.check_break_expr(value, span),
            HirExprKind::Continue => self.check_continue_expr(span),
            HirExprKind::Borrow { mutable, expr } => self.check_borrow_expr(mutable, expr, span),
            HirExprKind::Deref(_)
            | HirExprKind::Index { .. }
            | HirExprKind::Field { .. }
            | HirExprKind::NamedField { .. } => self.check_place_expr(expr),
            HirExprKind::Array(elems) => self.check_array_expr(&elems, span),
            HirExprKind::Tuple(elems) => self.check_tuple_expr(&elems),
            HirExprKind::Range { start, end } => self.check_range_expr(start, end, span),
            HirExprKind::Err => self.tys.error(),
        };

        self.results.set_expr_ty(expr, ty);
        ty
    }

    /// 检查已解析名字 `Res` 的类型。
    ///
    /// 局部变量从 `local_tys` 中读取，顶层定义从 `def_tys` 中读取。`Res::Err`
    /// 表示名字解析阶段已经报过错，这里返回 Error 类型以减少连锁错误。
    fn check_res(&mut self, res: Res, span: Span) -> TyId {
        match res {
            Res::Local(local) => {
                let ty = self.local_ty_without_read(local, span.clone());
                if !self.initialized_locals.contains(&local) {
                    let name = self
                        .locals
                        .get(local)
                        .map(|local| local.name.clone())
                        .unwrap_or_else(|| format!("{local:?}"));
                    self.emit_error(TypeErrorKind::UninitializedLocal { name }, span);
                }
                ty
            }
            Res::Def(def) => {
                if self.defs.get(def).is_none() {
                    self.emit_internal("Resolved definition id is not present in DefTable!");
                    return self.tys.error();
                }
                self.results.get_def_ty(def).copied().unwrap_or_else(|| {
                    self.emit_internal("Resolved definition has no collected type!");
                    self.tys.error()
                })
            }
            Res::Err => self.tys.error(),
        }
    }

    /// 检查二元表达式。
    ///
    /// 当前语言没有 trait 和运算符重载，所以二元运算采用硬编码规则：
    /// 算术和比较运算的两个操作数都要求是 `i32`。由于当前类型系统还没有 `bool`，
    /// 比较运算暂时也返回 `i32`，作为后续加入布尔类型前的占位策略。
    fn check_binary_expr(
        &mut self,
        op: BinaryOp,
        lhs: HirExprId,
        rhs: HirExprId,
        span: Span,
    ) -> TyId {
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let lhs_ty = self.check_expr(lhs, None);
                let rhs_ty = self.check_expr(rhs, Some(lhs_ty));
                let ty = self.unify_at(lhs_ty, rhs_ty, span.clone());
                if !self.is_int_ty(ty) {
                    let int_ty = self.tys.int();
                    self.unify_at(int_ty, ty, span);
                    self.tys.error()
                } else {
                    ty
                }
            }
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => {
                let lhs_ty = self.check_expr(lhs, None);
                let rhs_ty = self.check_expr(rhs, Some(lhs_ty));
                let ty = self.unify_at(lhs_ty, rhs_ty, span.clone());
                if !self.is_int_ty(ty) {
                    let int_ty = self.tys.int();
                    self.unify_at(int_ty, ty, span);
                }
                self.tys.bool()
            }
        }
    }

    /// 检查函数调用表达式。
    ///
    /// callee 必须解析到函数类型；参数数量必须一致；每个实参类型要能和对应形参类型
    /// 统一。调用表达式的类型是函数返回类型。
    fn check_call_expr(
        &mut self,
        callee: Res,
        args: &[HirExprId],
        expected: Option<TyId>,
        span: Span,
    ) -> TyId {
        let callee_ty = self.check_res(callee, span.clone());
        let callee_ty = self.infer.resolve_ty(&self.tys, callee_ty);

        let TyKind::Fn {
            params,
            ret,
            variadic,
        } = self.tys.kind(callee_ty).clone()
        else {
            if !matches!(self.tys.kind(callee_ty), TyKind::Error) {
                self.emit_error(TypeErrorKind::NotCallable { callee: callee_ty }, span);
            }
            for &arg in args {
                self.check_expr(arg, None);
            }
            return self.tys.error();
        };

        if variadic {
            if args.len() < params.len() {
                self.emit_error(
                    TypeErrorKind::WrongVariadicArgCount {
                        expected_at_least: params.len(),
                        actual: args.len(),
                    },
                    span.clone(),
                );
            }
        } else if params.len() != args.len() {
            self.emit_error(
                TypeErrorKind::WrongArgCount {
                    expected: params.len(),
                    actual: args.len(),
                },
                span.clone(),
            );
        }

        for (&arg, param_ty) in args.iter().zip(params.iter().copied()) {
            let arg_ty = self.check_expr(arg, Some(param_ty));
            self.unify_at(param_ty, arg_ty, self.expr_span(arg));
        }
        for &arg in args.iter().skip(params.len()) {
            let arg_ty = self.check_expr(arg, None);
            if variadic && !self.is_valid_variadic_arg_ty(arg_ty) {
                self.emit_error(
                    TypeErrorKind::InvalidVariadicArgType { ty: arg_ty },
                    self.expr_span(arg),
                );
            }
        }

        let ret = self.infer.resolve_ty(&self.tys, ret);
        if matches!(self.tys.kind(ret), TyKind::Unit) && !self.expected_is_unit(expected) {
            self.emit_error(TypeErrorKind::UnitValueUsedAsRvalue, span);
        }

        ret
    }

    /// 检查赋值表达式。
    ///
    /// 赋值左侧必须是可赋值的 place，右侧类型必须能和左侧类型统一。赋值表达式
    /// 本身类型为 `Unit`。
    fn check_assign_expr(&mut self, lhs: HirExprId, rhs: HirExprId, span: Span) -> TyId {
        let lhs_ty = self.check_place_expr(lhs);
        if !self.is_assignable(lhs) {
            self.emit_error(
                TypeErrorKind::NotAssignable { target: lhs_ty },
                span.clone(),
            );
        }
        let rhs_ty = self.check_expr(rhs, Some(lhs_ty));
        self.unify_at(lhs_ty, rhs_ty, span);
        self.mark_assignment_target_initialized(lhs);
        self.tys.unit()
    }

    /// 检查 if 表达式。
    ///
    /// 条件当前要求为 `i32`。有 else 时，then 和 else 的类型必须一致；没有 else 时，
    /// if 只能作为语句式 unit 使用，若外部期望一个非 unit 值则报告
    /// `MissingElseForValueIf`。
    fn check_if_expr(
        &mut self,
        cond: HirExprId,
        then_block: &HirBlock,
        else_expr: Option<HirExprId>,
        expected: Option<TyId>,
        span: Span,
    ) -> TyId {
        let bool_ty = self.tys.bool();
        let cond_ty = self.check_expr(cond, Some(bool_ty));
        self.unify_at(bool_ty, cond_ty, self.expr_span(cond));

        let then_ty = self.check_block(then_block, expected);
        match else_expr {
            Some(else_expr) => {
                let else_ty = self.check_expr(else_expr, expected);
                match self.infer.unify(&mut self.tys, then_ty, else_ty) {
                    Ok(ty) => ty,
                    Err(_) => {
                        self.emit_error(TypeErrorKind::IfBranchMismatch { then_ty, else_ty }, span);
                        self.tys.error()
                    }
                }
            }
            None => {
                let unit_ty = self.tys.unit();
                let expects_value = expected
                    .map(|expected| {
                        let expected = self.infer.resolve_ty(&self.tys, expected);
                        !matches!(self.tys.kind(expected), TyKind::Unit | TyKind::Error)
                    })
                    .unwrap_or_else(|| {
                        let then_ty = self.infer.resolve_ty(&self.tys, then_ty);
                        !matches!(
                            self.tys.kind(then_ty),
                            TyKind::Unit | TyKind::Never | TyKind::Error
                        )
                    });

                if expects_value {
                    self.emit_error(TypeErrorKind::MissingElseForValueIf { then_ty }, span);
                    self.tys.error()
                } else {
                    unit_ty
                }
            }
        }
    }

    /// 检查 while 表达式。
    ///
    /// while 条件当前要求为 `i32`，循环体按 unit 上下文检查。while 作为表达式时
    /// 结果类型为 `Unit`。
    fn check_while_expr(&mut self, cond: HirExprId, body: &HirBlock) -> TyId {
        let bool_ty = self.tys.bool();
        let cond_ty = self.check_expr(cond, Some(bool_ty));
        self.unify_at(bool_ty, cond_ty, self.expr_span(cond));

        let unit_ty = self.tys.unit();
        self.loop_break_tys.push(unit_ty);
        self.check_block(body, Some(unit_ty));
        self.loop_break_tys.pop();

        unit_ty
    }

    /// 检查 loop 表达式。
    ///
    /// `loop` 可以作为值表达式使用：`loop { break x; }` 的类型由 `break x`
    /// 的值决定。如果外层上下文已经给出期望类型，就用它约束所有 `break value`；
    /// 否则创建一个推导变量，交给 break 分支反向推导。
    fn check_loop_expr(&mut self, body: &HirBlock, expected: Option<TyId>) -> TyId {
        let loop_ty = expected.unwrap_or_else(|| self.infer.new_ty_var(&mut self.tys));
        let unit_ty = self.tys.unit();
        self.loop_break_tys.push(loop_ty);
        self.check_block(body, Some(unit_ty));
        self.loop_break_tys.pop();

        loop_ty
    }

    /// 检查专门的范围 for 表达式。
    ///
    /// HIR 已经把 `for x in a..b` 降成 `ForRange`，所以这里不需要 trait/iterator
    /// 逻辑，只需要要求起点和终点为 `i32`，并把循环变量类型记录为 `i32`。
    fn check_for_range_expr(
        &mut self,
        local_id: LocalId,
        explicit_ty: Option<&HirTy>,
        start: HirExprId,
        end: HirExprId,
        body: &HirBlock,
        span: Span,
    ) -> TyId {
        let int_ty = self.tys.int();
        let local_ty = explicit_ty
            .map(|ty| self.lower_hir_ty(ty))
            .unwrap_or(int_ty);
        self.unify_at(int_ty, local_ty, span.clone());
        self.results.set_local_ty(local_id, int_ty);
        self.mark_local_initialized(local_id);

        let start_ty = self.check_expr(start, Some(int_ty));
        let end_ty = self.check_expr(end, Some(int_ty));
        self.unify_at(int_ty, start_ty, self.expr_span(start));
        self.unify_at(int_ty, end_ty, self.expr_span(end));

        let unit_ty = self.tys.unit();
        self.loop_break_tys.push(unit_ty);
        self.check_block(body, Some(unit_ty));
        self.loop_break_tys.pop();

        unit_ty
    }

    /// 检查数组/表达式形式的 `for` 循环。
    ///
    /// 当前语言没有通用迭代器 trait，因此 v1 只把数组视为可迭代结构。循环变量类型
    /// 来自数组元素类型，若用户写了显式类型则需要和元素类型统一。
    fn check_for_iter_expr(
        &mut self,
        local_id: LocalId,
        explicit_ty: Option<&HirTy>,
        iter: HirExprId,
        body: &HirBlock,
        span: Span,
    ) -> TyId {
        let iter_ty = self.check_expr(iter, None);
        let iter_ty = self.infer.resolve_ty(&self.tys, iter_ty);
        let elem_ty = match self.tys.kind(iter_ty).clone() {
            TyKind::Array { elem, .. } => elem,
            TyKind::Error => self.tys.error(),
            _ => {
                self.emit_error(
                    TypeErrorKind::InvalidForIterator { ty: iter_ty },
                    span.clone(),
                );
                self.tys.error()
            }
        };

        let local_ty = explicit_ty
            .map(|ty| self.lower_hir_ty(ty))
            .map(|explicit| self.unify_at(elem_ty, explicit, span.clone()))
            .unwrap_or(elem_ty);
        self.results.set_local_ty(local_id, local_ty);
        self.mark_local_initialized(local_id);

        let unit_ty = self.tys.unit();
        self.loop_break_tys.push(unit_ty);
        self.check_block(body, Some(unit_ty));
        self.loop_break_tys.pop();

        unit_ty
    }

    /// 检查 return 表达式。
    ///
    /// return 的值必须和当前函数返回类型统一。return 自身不会正常产生值，
    /// 因此表达式类型为 `Never`。
    fn check_return_expr(&mut self, value: Option<HirExprId>, span: Span) -> TyId {
        let expected = self.current_ret_ty.unwrap_or_else(|| {
            self.emit_internal("Return expression checked outside a function!");
            self.tys.error()
        });

        let actual = match value {
            Some(value) => self.check_expr(value, Some(expected)),
            None => self.tys.unit(),
        };
        self.expect_return_ty(expected, actual, span);
        self.tys.never()
    }

    /// 检查 break 表达式。
    ///
    /// break 必须出现在循环中。若循环是 `loop` 表达式，`break value` 会和该
    /// loop 的结果类型统一；若是 `while/for`，其 break 目标类型为 `Unit`。
    /// break 自身不会正常产生值，因此表达式类型为 `Never`。
    fn check_break_expr(&mut self, value: Option<HirExprId>, span: Span) -> TyId {
        let Some(&break_ty) = self.loop_break_tys.last() else {
            self.emit_error(TypeErrorKind::BreakOutsideLoop, span);
            if let Some(value) = value {
                self.check_expr(value, None);
            }
            return self.tys.never();
        };

        let actual = match value {
            Some(value) => self.check_expr(value, Some(break_ty)),
            None => self.tys.unit(),
        };
        self.unify_at(break_ty, actual, span);
        self.tys.never()
    }

    /// 检查 continue 表达式。
    ///
    /// continue 必须出现在循环中。它不会正常产生值，因此类型为 `Never`。
    fn check_continue_expr(&mut self, span: Span) -> TyId {
        if self.loop_break_tys.is_empty() {
            self.emit_error(TypeErrorKind::ContinueOutsideLoop, span);
        }
        self.tys.never()
    }

    /// 检查借用表达式。
    ///
    /// 借用表达式的结果类型是 `&T` 或 `&mut T`。当前阶段不做完整借用检查，
    /// 只在可变借用明显作用于不可变局部变量时报告 `CannotBorrow`。
    fn check_borrow_expr(&mut self, mutable: bool, expr: HirExprId, span: Span) -> TyId {
        let inner_ty = self.check_expr(expr, None);
        if mutable && !self.can_mut_borrow(expr) {
            self.emit_error(
                TypeErrorKind::CannotBorrow {
                    mutable,
                    ty: inner_ty,
                },
                span,
            );
        }
        self.tys.intern(TyKind::Ref {
            mutable,
            inner: inner_ty,
        })
    }

    /// 检查数组表达式。
    ///
    /// 非空数组要求所有元素类型一致，数组长度直接来自元素数量。空数组没有足够信息
    /// 决定元素类型，因此会创建一个推导变量，等待外层上下文约束。
    fn check_array_expr(&mut self, elems: &[HirExprId], span: Span) -> TyId {
        let elem_ty = if let Some((&first, rest)) = elems.split_first() {
            let elem_ty = self.check_expr(first, None);
            for &elem in rest {
                let ty = self.check_expr(elem, Some(elem_ty));
                self.unify_at(elem_ty, ty, self.expr_span(elem));
            }
            elem_ty
        } else {
            self.infer.new_ty_var(&mut self.tys)
        };

        let array_ty = self.tys.intern(TyKind::Array {
            elem: elem_ty,
            len: elems.len(),
        });
        if elems.is_empty() {
            self.emit_error(TypeErrorKind::CannotInferType { ty: elem_ty }, span);
        }
        array_ty
    }

    /// 检查元组表达式。
    ///
    /// 元组类型由每个元素表达式的类型顺序组成，例如 `(i32, &i32)` 会得到
    /// `TyKind::Tuple(vec![i32, ref_i32])`。
    fn check_tuple_expr(&mut self, elems: &[HirExprId]) -> TyId {
        let elems = elems
            .iter()
            .map(|&elem| self.check_expr(elem, None))
            .collect();
        self.tys.intern(TyKind::Tuple(elems))
    }

    /// 检查 named-field struct literal。
    ///
    /// 字段表来自 HIR `DefTable`，因此这里不再做类型名解析，只校验字段集合和每个
    /// 字段初始化表达式的类型。字段顺序不要求和声明一致，但不能重复，也不能遗漏。
    fn check_struct_lit_expr(
        &mut self,
        def_id: DefId,
        fields: &[HirStructLitField],
        span: Span,
    ) -> TyId {
        let Some(def) = self.defs.get(def_id).cloned() else {
            self.emit_internal_at("Struct definition not found!", span);
            return self.tys.error();
        };

        if def.kind != DefKind::Struct {
            let ty = self.tys.intern(TyKind::Adt(def_id));
            self.emit_error(TypeErrorKind::NotStruct { ty }, span);
            for field in fields {
                self.check_expr(field.expr, None);
            }
            return self.tys.error();
        }

        let mut seen = HashSet::new();
        for field in fields {
            if !seen.insert(field.name.clone()) {
                self.emit_error(
                    TypeErrorKind::DuplicateStructField {
                        field: field.name.clone(),
                    },
                    field.span.clone(),
                );
            }

            match def
                .struct_fields
                .iter()
                .position(|decl_field| decl_field.name == field.name)
            {
                Some(index) => {
                    let decl_ty = def.struct_fields[index].ty.clone();
                    let expected = self.lower_hir_ty(&decl_ty);
                    let actual = self.check_expr(field.expr, Some(expected));
                    self.unify_at(expected, actual, self.expr_span(field.expr));
                }
                None => {
                    let base = self.tys.intern(TyKind::Adt(def_id));
                    self.emit_error(
                        TypeErrorKind::UnknownField {
                            base,
                            field: field.name.clone(),
                        },
                        field.span.clone(),
                    );
                    self.check_expr(field.expr, None);
                }
            }
        }

        for decl_field in &def.struct_fields {
            if !seen.contains(&decl_field.name) {
                self.emit_error(
                    TypeErrorKind::MissingStructField {
                        def_id,
                        field: decl_field.name.clone(),
                    },
                    span.clone(),
                );
            }
        }

        self.tys.intern(TyKind::Adt(def_id))
    }

    /// 检查范围表达式。
    ///
    /// 当前 HIR 的 `ForRange` 已经专门处理 for 循环，普通 range 表达式还没有独立的
    /// 语义类型。这里仍然检查两端是 `i32`，然后返回 Error 类型避免误用。
    fn check_range_expr(&mut self, start: HirExprId, end: HirExprId, span: Span) -> TyId {
        let int_ty = self.tys.int();
        let start_ty = self.check_expr(start, Some(int_ty));
        let end_ty = self.check_expr(end, Some(int_ty));
        self.unify_at(int_ty, start_ty, self.expr_span(start));
        self.unify_at(int_ty, end_ty, self.expr_span(end));
        self.emit_internal_at("Standalone range expression has no type yet!", span);
        self.tys.error()
    }

    /// 检查 place-like 表达式，并返回它指向位置的值类型。
    ///
    /// 赋值左侧、解引用、索引和字段访问都需要这类检查。该函数只计算 place 类型，
    /// 是否允许写入由 `is_assignable` 决定。
    fn check_place_expr(&mut self, expr: HirExprId) -> TyId {
        let Some(expr_data) = self.hir.expr(expr).cloned() else {
            self.emit_internal("Place expression not found!");
            return self.tys.error();
        };

        let ty = match expr_data.kind {
            HirExprKind::Path(Res::Local(local)) => {
                self.local_ty_without_read(local, expr_data.span)
            }
            HirExprKind::Path(res) => self.check_res(res, expr_data.span),
            HirExprKind::Deref(base) => {
                let base_ty = self.check_expr(base, None);
                let base_ty = self.infer.resolve_ty(&self.tys, base_ty);
                match self.tys.kind(base_ty).clone() {
                    TyKind::Ref { inner, .. } => inner,
                    TyKind::Error => self.tys.error(),
                    _ => {
                        self.emit_error(TypeErrorKind::CannotDeref { ty: base_ty }, expr_data.span);
                        self.tys.error()
                    }
                }
            }
            HirExprKind::Index { base, index } => {
                let int_ty = self.tys.int();
                let index_ty = self.check_expr(index, Some(int_ty));
                self.unify_at(int_ty, index_ty, self.expr_span(index));

                let base_ty = self.check_expr(base, None);
                let base_ty = self.infer.resolve_ty(&self.tys, base_ty);
                match self.tys.kind(base_ty).clone() {
                    TyKind::Array { elem, len } => {
                        if let Some(index) = self.const_int_expr_value(index) {
                            if index < 0 || index as usize >= len {
                                self.emit_error(
                                    TypeErrorKind::ArrayIndexOutOfBounds { index, len },
                                    expr_data.span.clone(),
                                );
                            }
                        }
                        elem
                    }
                    TyKind::Error => self.tys.error(),
                    _ => {
                        self.emit_error(
                            TypeErrorKind::InvalidIndex {
                                base: base_ty,
                                index: index_ty,
                            },
                            expr_data.span,
                        );
                        self.tys.error()
                    }
                }
            }
            HirExprKind::Field { base, index } => {
                let base_ty = self.check_expr(base, None);
                let base_ty = self.infer.resolve_ty(&self.tys, base_ty);
                match self.tys.kind(base_ty).clone() {
                    TyKind::Tuple(elems) => elems.get(index).copied().unwrap_or_else(|| {
                        let index_ty = self.tys.int();
                        self.emit_error(
                            TypeErrorKind::InvalidIndex {
                                base: base_ty,
                                index: index_ty,
                            },
                            expr_data.span.clone(),
                        );
                        self.tys.error()
                    }),
                    TyKind::Array { elem, len } => {
                        if index < len {
                            elem
                        } else {
                            let index_ty = self.tys.int();
                            self.emit_error(
                                TypeErrorKind::InvalidIndex {
                                    base: base_ty,
                                    index: index_ty,
                                },
                                expr_data.span.clone(),
                            );
                            self.tys.error()
                        }
                    }
                    TyKind::Error => self.tys.error(),
                    _ => {
                        let index_ty = self.tys.int();
                        self.emit_error(
                            TypeErrorKind::InvalidIndex {
                                base: base_ty,
                                index: index_ty,
                            },
                            expr_data.span,
                        );
                        self.tys.error()
                    }
                }
            }
            HirExprKind::NamedField { base, name } => {
                let base_ty = self.check_expr(base, None);
                let base_ty = self.infer.resolve_ty(&self.tys, base_ty);
                match self.tys.kind(base_ty).clone() {
                    TyKind::Adt(def_id) => match self.struct_field_index_and_ty(def_id, &name) {
                        Some((index, ty)) => {
                            self.results.set_field_index(expr, index);
                            ty
                        }
                        None => {
                            self.emit_error(
                                TypeErrorKind::UnknownField {
                                    base: base_ty,
                                    field: name,
                                },
                                expr_data.span,
                            );
                            self.tys.error()
                        }
                    },
                    TyKind::Error => self.tys.error(),
                    _ => {
                        self.emit_error(TypeErrorKind::NotStruct { ty: base_ty }, expr_data.span);
                        self.tys.error()
                    }
                }
            }
            _ => {
                let ty = self.check_expr(expr, None);
                self.emit_error(TypeErrorKind::NotAssignable { target: ty }, expr_data.span);
                self.tys.error()
            }
        };

        self.results.set_expr_ty(expr, ty);
        ty
    }

    /// 判断一个表达式是否可以作为赋值目标。
    ///
    /// 局部变量需要声明为 mutable；索引和字段访问继承 base 的可赋值性；解引用在
    /// 当前阶段暂时允许写入，完整限制留给后续借用检查。
    fn is_assignable(&mut self, expr: HirExprId) -> bool {
        let Some(expr_data) = self.hir.expr(expr) else {
            return false;
        };

        match &expr_data.kind {
            HirExprKind::Path(Res::Local(local)) => self
                .locals
                .get(*local)
                .map(|local| local.mutable)
                .unwrap_or(false),
            HirExprKind::Deref(base) => {
                self.deref_base_is_mutable_ref(*base, expr_data.span.clone())
            }
            HirExprKind::Index { base, .. }
            | HirExprKind::Field { base, .. }
            | HirExprKind::NamedField { base, .. } => self.is_assignable(*base),
            _ => false,
        }
    }

    /// 判断一个表达式是否可以被可变借用。
    ///
    /// 这不是完整借用检查，只用于捕获最直接的错误：对不可变局部变量执行 `&mut`。
    fn can_mut_borrow(&self, expr: HirExprId) -> bool {
        let Some(expr_data) = self.hir.expr(expr) else {
            return false;
        };

        match &expr_data.kind {
            HirExprKind::Path(Res::Local(local)) => self
                .locals
                .get(*local)
                .map(|local| local.mutable)
                .unwrap_or(false),
            HirExprKind::Deref(_) => true,
            HirExprKind::Index { base, .. }
            | HirExprKind::Field { base, .. }
            | HirExprKind::NamedField { base, .. } => self.can_mut_borrow(*base),
            _ => true,
        }
    }

    /// 根据结构体定义查询字段序号和字段类型。
    ///
    /// 返回的 index 是声明顺序中的位置，后续 THIR 和 IR 会用它生成 GEP 字段访问。
    fn struct_field_index_and_ty(&mut self, def_id: DefId, name: &str) -> Option<(usize, TyId)> {
        let fields = self.defs.get(def_id)?.struct_fields.clone();
        let (index, field) = fields
            .into_iter()
            .enumerate()
            .find(|(_, field)| field.name == name)?;
        let ty = self.lower_hir_ty(&field.ty);
        Some((index, ty))
    }

    /// 统一两个类型；失败时报告普通类型不匹配，并返回 Error 类型。
    ///
    /// 这个函数负责把 `InferCtx` 的低层错误转换成带当前位置的类型错误，
    /// 同时保证检查器可以继续向后检查，避免第一个错误直接中断整个阶段。
    fn unify_at(&mut self, expected: TyId, actual: TyId, span: Span) -> TyId {
        match self.infer.unify(&mut self.tys, expected, actual) {
            Ok(ty) => ty,
            Err(_) => {
                self.emit_error(TypeErrorKind::MismatchedTypes { expected, actual }, span);
                self.tys.error()
            }
        }
    }

    /// 检查函数返回类型；失败时报告 `ReturnTypeMismatch`。
    ///
    /// return 检查需要和普通表达式类型不匹配区分开，所以单独保留这个辅助函数。
    fn expect_return_ty(&mut self, expected: TyId, actual: TyId, span: Span) -> TyId {
        match self.infer.unify(&mut self.tys, expected, actual) {
            Ok(ty) => ty,
            Err(_) => {
                self.emit_error(TypeErrorKind::ReturnTypeMismatch { expected, actual }, span);
                self.tys.error()
            }
        }
    }

    /// 获取表达式 span；若 ID 损坏则返回空 span。
    fn expr_span(&self, expr: HirExprId) -> Span {
        self.hir
            .expr(expr)
            .map(|expr| expr.span.clone())
            .unwrap_or_else(Span::default)
    }

    /// 在检查结束前解析结果旁表中的推导变量。
    ///
    /// 这一阶段会把已经被约束确定的 `?T` 展开成最终类型。若某个局部变量或表达式
    /// 仍然保留未绑定推导变量，则报告 `CannotInferType`。
    fn resolve_result_tys(&mut self) {
        let expr_entries = self
            .results
            .expr_tys
            .iter()
            .map(|(&id, &ty)| (id, ty))
            .collect::<Vec<_>>();
        for (id, ty) in expr_entries {
            let resolved = self.infer.deep_resolve_ty(&mut self.tys, ty);
            self.results.set_expr_ty(id, resolved);
            self.report_unresolved_ty(resolved, self.expr_span(id));
        }

        let stmt_entries = self
            .results
            .stmt_tys
            .iter()
            .map(|(&id, &ty)| (id, ty))
            .collect::<Vec<_>>();
        for (id, ty) in stmt_entries {
            let resolved = self.infer.deep_resolve_ty(&mut self.tys, ty);
            self.results.set_stmt_ty(id, resolved);
        }

        let local_entries = self
            .results
            .local_tys
            .iter()
            .map(|(&id, &ty)| (id, ty))
            .collect::<Vec<_>>();
        for (id, ty) in local_entries {
            let resolved = self.infer.deep_resolve_ty(&mut self.tys, ty);
            self.results.set_local_ty(id, resolved);
            let span = self
                .locals
                .get(id)
                .map(|local| local.span.clone())
                .unwrap_or_else(Span::default);
            self.report_unresolved_ty(resolved, span);
        }

        let def_entries = self
            .results
            .def_tys
            .iter()
            .map(|(&id, &ty)| (id, ty))
            .collect::<Vec<_>>();
        for (id, ty) in def_entries {
            let resolved = self.infer.deep_resolve_ty(&mut self.tys, ty);
            self.results.set_def_ty(id, resolved);
        }
    }

    /// 如果类型中仍有无法确定的推导变量，则报告 `CannotInferType`。
    fn report_unresolved_ty(&mut self, ty: TyId, span: Span) {
        if self.contains_infer_ty(ty) {
            self.emit_error(TypeErrorKind::CannotInferType { ty }, span);
        }
    }

    /// 判断类型结构中是否仍包含推导变量。
    fn contains_infer_ty(&self, ty: TyId) -> bool {
        match self.tys.kind(ty) {
            TyKind::Infer(_) => true,
            TyKind::Tuple(elems) => elems.iter().any(|&elem| self.contains_infer_ty(elem)),
            TyKind::Array { elem, .. } => self.contains_infer_ty(*elem),
            TyKind::Ref { inner, .. } => self.contains_infer_ty(*inner),
            TyKind::Fn { params, ret, .. } => {
                params.iter().any(|&param| self.contains_infer_ty(param))
                    || self.contains_infer_ty(*ret)
            }
            TyKind::Int(_)
            | TyKind::Bool
            | TyKind::Str
            | TyKind::Adt(_)
            | TyKind::Unit
            | TyKind::Never
            | TyKind::Error => false,
        }
    }

    /// 判断一个类型是否可以安全地作为 C varargs 实参传给外部函数。
    ///
    /// v1 允许整数、`str` 和引用类型。它们分别映射到 LLVM 整数或 `ptr`，
    /// 足以覆盖 `printf("%d", x)` 与 `scanf("%d", &mut x)` 这类 C 接口。
    fn is_valid_variadic_arg_ty(&mut self, ty: TyId) -> bool {
        let ty = self.infer.resolve_ty(&self.tys, ty);
        matches!(
            self.tys.kind(ty),
            TyKind::Int(_) | TyKind::Str | TyKind::Ref { .. } | TyKind::Error
        )
    }

    /// 判断一个类型是否已经解析为 `Never`。
    ///
    /// `Never` 表示表达式不会正常返回，例如 `return`、`break`、`continue`。
    /// 这个辅助函数用于在分号语句和 block 中传播发散信息。
    fn is_never_ty(&mut self, ty: TyId) -> bool {
        let ty = self.infer.resolve_ty(&self.tys, ty);
        matches!(self.tys.kind(ty), TyKind::Never)
    }

    /// 判断一个类型是否已经解析为整数类型。
    ///
    /// 整数类型包括所有有符号、无符号和指针宽度整数；推导变量在调用者需要时应先
    /// 通过期望类型约束成具体整数。
    fn is_int_ty(&mut self, ty: TyId) -> bool {
        let ty = self.infer.resolve_ty(&self.tys, ty);
        matches!(self.tys.kind(ty), TyKind::Int(_))
    }

    /// 为整数字面量选择类型。
    ///
    /// 如果上下文已经给出具体整数类型，例如 `let x: i64 = 1`，字面量采用该类型；
    /// 如果没有上下文或上下文不是整数类型，则使用语言默认的 `i32`。
    fn integer_literal_ty(&mut self, expected: Option<TyId>) -> TyId {
        if let Some(expected) = expected {
            let expected = self.infer.resolve_ty(&self.tys, expected);
            if matches!(self.tys.kind(expected), TyKind::Int(_)) {
                return expected;
            }
        }
        self.tys.int()
    }

    /// 判断调用所在上下文是否明确期望 `()`。
    ///
    /// 用于区分 `f();` 这种丢弃 unit 返回值的语句和 `let x = f();` 这种把 unit
    /// 当作右值保存的用法。
    fn expected_is_unit(&mut self, expected: Option<TyId>) -> bool {
        expected
            .map(|ty| {
                let ty = self.infer.resolve_ty(&self.tys, ty);
                matches!(self.tys.kind(ty), TyKind::Unit | TyKind::Error)
            })
            .unwrap_or(false)
    }

    /// 读取局部变量类型，但不把这个动作视为“读取变量值”。
    ///
    /// 赋值左侧 `a = 1` 需要知道 `a` 的类型，却不能因为 `a` 尚未初始化就报错。
    fn local_ty_without_read(&mut self, local: LocalId, span: Span) -> TyId {
        self.results
            .get_local_ty(local)
            .copied()
            .unwrap_or_else(|| {
                let error_ty = self.tys.error();
                self.emit_error(TypeErrorKind::CannotInferType { ty: error_ty }, span);
                error_ty
            })
    }

    /// 把一个局部变量标记为已经初始化。
    fn mark_local_initialized(&mut self, local: LocalId) {
        self.initialized_locals.insert(local);
    }

    /// 把 pattern 中的所有绑定标记为已经初始化。
    fn mark_pat_initialized(&mut self, pat: &HirPat) {
        match &pat.kind {
            HirPatKind::Wildcard => {}
            HirPatKind::Binding { local_id, .. } => self.mark_local_initialized(*local_id),
            HirPatKind::Tuple(elems) => {
                for elem in elems {
                    self.mark_pat_initialized(elem);
                }
            }
            HirPatKind::Struct { fields, .. } => {
                for field in fields {
                    self.mark_pat_initialized(&field.pat);
                }
            }
        }
    }

    /// 赋值完成后，若左侧是直接局部变量，则该变量变为已初始化。
    ///
    /// 字段、索引和解引用赋值不会让整个 base 变量从未初始化变为已初始化；它们的
    /// base 在 `check_place_expr` 中已经作为 place base 被检查过。
    fn mark_assignment_target_initialized(&mut self, expr: HirExprId) {
        let Some(expr_data) = self.hir.expr(expr) else {
            return;
        };
        if let HirExprKind::Path(Res::Local(local)) = expr_data.kind {
            self.mark_local_initialized(local);
        }
    }

    /// 判断解引用赋值的 base 是否为可变引用。
    fn deref_base_is_mutable_ref(&mut self, base: HirExprId, span: Span) -> bool {
        let base_ty = self.check_expr(base, None);
        let base_ty = self.infer.resolve_ty(&self.tys, base_ty);
        match self.tys.kind(base_ty) {
            TyKind::Ref { mutable: true, .. } | TyKind::Error => true,
            TyKind::Ref { mutable: false, .. } => {
                self.emit_error(TypeErrorKind::AssignThroughImmutableReference, span);
                false
            }
            _ => false,
        }
    }

    /// 尝试读取常量整数字面量，用于数组下标静态范围检查。
    fn const_int_expr_value(&self, expr: HirExprId) -> Option<i32> {
        match self.hir.expr(expr).map(|expr| &expr.kind) {
            Some(HirExprKind::Int(value)) => Some(*value),
            _ => None,
        }
    }

    /// 报告一个内部错误，位置使用空 span。
    fn emit_internal(&mut self, message: &str) {
        self.emit_internal_at(message, Span::default());
    }

    /// 报告一个带指定位置的内部错误。
    fn emit_internal_at(&mut self, message: &str, span: Span) {
        self.emit_error(
            TypeErrorKind::Internal {
                message: message.to_string(),
            },
            span,
        );
    }

    /// 记录一个类型检查错误。
    fn emit_error(&mut self, kind: TypeErrorKind, span: Span) {
        self.errors.push(TypeError { kind, span });
    }
}
