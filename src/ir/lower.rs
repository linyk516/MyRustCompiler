use std::collections::HashMap;

use crate::{
    ast::ty::BinaryOp,
    hir::table::DefTable,
    ir::{
        error::{IrLowerError, IrLowerErrorKind},
        id::{IrBlockId, IrValueId},
        node::{
            IrBasicBlock, IrBinaryOp, IrFunction, IrIcmpPred, IrInstr, IrInstrKind, IrProgram,
            IrSlot, IrTerminator, IrTy, IrValue, IrValueKind,
        },
        output::IrOutput,
    },
    lexer::token::Span,
    thir::{
        id::{ThirBodyId, ThirExprId, ThirLocalId, ThirStmtId},
        node::{
            ThirBlock, ThirBody, ThirExprKind, ThirPlace, ThirPlaceKind, ThirProgram, ThirStmtKind,
        },
    },
    typecheck::ty::{TyId, TyKind, TyStore},
};

type IrLowerResult<T> = Result<T, IrLowerError>;

#[derive(Debug, Clone)]
struct LoopContext {
    break_bb: IrBlockId,
    continue_bb: IrBlockId,
    break_value: Option<(IrValueId, IrTy)>,
}

/// THIR 到 LLVM-like IR 的 lowering 上下文。
///
/// 这一阶段只消费已经完成类型检查的 THIR 和 TyStore，不重新做名字解析或类型推导。
pub struct IrLowerCtx<'thir> {
    thir: &'thir ThirProgram,
    defs: &'thir DefTable,
    tys: &'thir TyStore,

    program: IrProgram,
    errors: Vec<IrLowerError>,
}

impl<'thir> IrLowerCtx<'thir> {
    pub fn new(thir: &'thir ThirProgram, defs: &'thir DefTable, tys: &'thir TyStore) -> Self {
        Self {
            thir,
            defs,
            tys,
            program: IrProgram::new(),
            errors: vec![],
        }
    }

    pub fn lower(mut self) -> IrOutput {
        self.lower_program();
        IrOutput {
            program: self.program,
            errors: self.errors,
        }
    }

    fn lower_program(&mut self) {
        for index in 0..self.thir.bodies.len() {
            let body_id = ThirBodyId(index);
            match self.lower_function(body_id) {
                Ok(function) => {
                    let owner = function.owner;
                    self.program.alloc_function(owner, function);
                }
                Err(error) => self.errors.push(error),
            }
        }
    }

    fn lower_function(&self, body_id: ThirBodyId) -> IrLowerResult<IrFunction> {
        let body = self.thir.body(body_id).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingBody {
                    id: body_id.index(),
                },
                Span::default(),
            )
        })?;

        let ret_ty = self.ir_ty_from_source_ty(self.infer_return_ty(body)?);
        let param_tys = body
            .params
            .iter()
            .map(|param| {
                body.local(*param)
                    .map(|local| self.ir_ty_from_source_ty(local.ty))
                    .ok_or_else(|| {
                        self.error(
                            IrLowerErrorKind::MissingLocal { id: param.index() },
                            Span::default(),
                        )
                    })
            })
            .collect::<IrLowerResult<Vec<_>>>()?;

        let symbol_name = self.function_symbol_name(body.owner);
        let lowerer = FunctionLowerer::new(self.tys, body, symbol_name, ret_ty, param_tys);
        lowerer.lower()
    }

    fn infer_return_ty(&self, body: &ThirBody) -> IrLowerResult<TyId> {
        if let Some(value) = body.expr(body.value) {
            if !matches!(
                self.tys.kind(value.ty),
                TyKind::Unit | TyKind::Never | TyKind::Error
            ) {
                return Ok(value.ty);
            }
        }

        for expr in &body.exprs {
            if let ThirExprKind::Return(Some(value)) = &expr.kind {
                if let Some(value) = body.expr(*value) {
                    return Ok(value.ty);
                }
            }
        }

        for expr in &body.exprs {
            if let ThirExprKind::Break(Some(value)) = &expr.kind {
                if let Some(value) = body.expr(*value) {
                    return Ok(value.ty);
                }
            }
        }

        body.expr(body.value).map(|expr| expr.ty).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingExpr {
                    id: body.value.index(),
                },
                Span::default(),
            )
        })
    }

    fn ir_ty_from_source_ty(&self, ty: TyId) -> IrTy {
        ir_ty_from_source_ty(self.tys, ty)
    }

    fn function_symbol_name(&self, owner: crate::hir::id::DefId) -> String {
        self.defs
            .get(owner)
            .map(|def| llvm_symbol_name(&def.name, owner))
            .unwrap_or_else(|| format!("fn{}", owner.index()))
    }

    fn error(&self, kind: IrLowerErrorKind, span: Span) -> IrLowerError {
        IrLowerError::new(kind, span)
    }
}

struct FunctionLowerer<'a> {
    tys: &'a TyStore,
    body: &'a ThirBody,
    builder: IrBuilder,
    local_map: HashMap<ThirLocalId, IrValueId>,
    loop_stack: Vec<LoopContext>,
    unit_value: Option<IrValueId>,
}

impl<'a> FunctionLowerer<'a> {
    fn new(
        tys: &'a TyStore,
        body: &'a ThirBody,
        symbol_name: String,
        ret_ty: IrTy,
        param_tys: Vec<IrTy>,
    ) -> Self {
        Self {
            tys,
            body,
            builder: IrBuilder::new(body.owner, symbol_name, ret_ty, param_tys),
            local_map: HashMap::new(),
            loop_stack: vec![],
            unit_value: None,
        }
    }

    fn lower(mut self) -> IrLowerResult<IrFunction> {
        self.alloc_local_slots()?;
        self.store_params_into_slots()?;

        let body_value = self.lower_expr_value(self.body.value)?;
        if self.builder.can_emit()? {
            if self.builder.function.ret_ty.is_void() {
                self.builder.set_terminator(IrTerminator::Ret {
                    ty: IrTy::Void,
                    value: None,
                })?;
            } else {
                let value = self.coerce_value(body_value, self.builder.function.ret_ty.clone())?;
                self.builder.set_terminator(IrTerminator::Ret {
                    ty: self.builder.function.ret_ty.clone(),
                    value: Some(value),
                })?;
            }
        }

        self.builder.validate()?;
        Ok(self.builder.finish())
    }

    fn alloc_local_slots(&mut self) -> IrLowerResult<()> {
        for (index, local) in self.body.locals.iter().enumerate() {
            let thir_local = ThirLocalId(index);
            let value_ty = self.ir_ty(local.ty);
            let name = format!("local{}", index);
            let addr = self.builder.emit_alloca_slot(
                IrSlot {
                    thir_local: Some(thir_local),
                    name,
                    mutable: local.mutable,
                    source_ty: local.ty,
                    value_ty: value_ty.clone(),
                    addr: None,
                    span: local.span.clone(),
                },
                value_ty,
                local.span.clone(),
            )?;
            self.local_map.insert(thir_local, addr);
        }

        Ok(())
    }

    fn store_params_into_slots(&mut self) -> IrLowerResult<()> {
        for (index, param) in self.body.params.iter().enumerate() {
            let local = self.body.local(*param).ok_or_else(|| {
                self.error(
                    IrLowerErrorKind::MissingLocal { id: param.index() },
                    Span::default(),
                )
            })?;
            let addr = self.local_addr(*param, &local.span)?;
            let param_value = self.builder.param_value(index).ok_or_else(|| {
                self.error(
                    IrLowerErrorKind::Internal {
                        message: format!("missing IR parameter #{index}"),
                    },
                    local.span.clone(),
                )
            })?;
            self.emit_store_to_addr(param_value, addr, self.ir_ty(local.ty), local.span.clone())?;
        }

        Ok(())
    }

    fn lower_block(&mut self, block: &ThirBlock) -> IrLowerResult<Option<IrValueId>> {
        for &stmt in &block.stmts {
            self.lower_stmt(stmt)?;
        }

        match block.expr {
            Some(expr) if self.builder.can_emit()? => Ok(Some(self.lower_expr_value(expr)?)),
            Some(_) => Ok(None),
            None => Ok(None),
        }
    }

    fn lower_stmt(&mut self, stmt: ThirStmtId) -> IrLowerResult<()> {
        if !self.builder.can_emit()? {
            return Ok(());
        }

        let stmt_data = self.body.stmt(stmt).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingStmt { id: stmt.index() },
                Span::default(),
            )
        })?;

        match &stmt_data.kind {
            ThirStmtKind::Let { local, init } => {
                if let Some(init) = init {
                    let local_data = self.body.local(*local).ok_or_else(|| {
                        self.error(
                            IrLowerErrorKind::MissingLocal { id: local.index() },
                            stmt_data.span.clone(),
                        )
                    })?;
                    let addr = self.local_addr(*local, &stmt_data.span)?;
                    self.lower_expr_into_addr(
                        *init,
                        addr,
                        self.ir_ty(local_data.ty),
                        stmt_data.span.clone(),
                    )?;
                }
            }
            ThirStmtKind::Expr(expr) | ThirStmtKind::Semi(expr) => {
                self.lower_expr_value(*expr)?;
            }
            ThirStmtKind::Empty => {}
        }

        Ok(())
    }

    fn lower_expr_value(&mut self, expr: ThirExprId) -> IrLowerResult<IrValueId> {
        let expr_data = self.expr(expr)?;
        let span = expr_data.span.clone();
        let expr_ty = expr_data.ty;
        let kind = expr_data.kind.clone();

        match kind {
            ThirExprKind::Int(value) => Ok(self.builder.const_int(value, IrTy::I32)),
            ThirExprKind::Use(place) => self.load_place_value(&place, span),
            ThirExprKind::Binary { op, lhs, rhs } => {
                self.lower_binary_expr(op, lhs, rhs, expr_ty, span)
            }
            ThirExprKind::Call { callee, args } => {
                let mut lowered_args = Vec::with_capacity(args.len());
                for arg in args {
                    let value = self.lower_expr_value(arg)?;
                    let ty = self.ir_ty(self.expr(arg)?.ty);
                    let value = self.coerce_value(value, ty.clone())?;
                    lowered_args.push((ty, value));
                }

                let ret_ty = self.ir_ty(expr_ty);
                let result =
                    self.builder
                        .emit_call(callee, ret_ty.clone(), lowered_args, span.clone())?;
                Ok(result.unwrap_or_else(|| self.unit_value()))
            }
            ThirExprKind::Assign { target, value } => {
                let addr = self.lower_place_addr(&target)?;
                self.lower_expr_into_addr(value, addr, self.ir_ty(target.ty), span)?;
                Ok(self.unit_value())
            }
            ThirExprKind::Block(block) => {
                let value = self.lower_block(&block)?;
                Ok(value.unwrap_or_else(|| self.unit_value()))
            }
            ThirExprKind::If {
                cond,
                then_expr,
                else_expr,
            } => self.lower_if_expr(cond, then_expr, else_expr, expr_ty, span),
            ThirExprKind::While { cond, body } => self.lower_while_expr(cond, &body, expr_ty, span),
            ThirExprKind::Loop { body } => self.lower_loop_expr(&body, expr_ty, span),
            ThirExprKind::ForRange {
                local,
                start,
                end,
                body,
            } => self.lower_for_range_expr(local, start, end, &body, expr_ty, span),
            ThirExprKind::Return(value) => {
                let value = match value {
                    Some(value) => {
                        let value = self.lower_expr_value(value)?;
                        if self.builder.function.ret_ty.is_void() {
                            None
                        } else {
                            Some(self.coerce_value(value, self.builder.function.ret_ty.clone())?)
                        }
                    }
                    None => None,
                };
                self.builder.set_terminator(IrTerminator::Ret {
                    ty: self.builder.function.ret_ty.clone(),
                    value,
                })?;
                Ok(self.unit_value())
            }
            ThirExprKind::Break(value) => {
                let context =
                    self.loop_stack.last().cloned().ok_or_else(|| {
                        self.error(IrLowerErrorKind::BreakOutsideLoop, span.clone())
                    })?;
                if let (Some(value), Some((addr, ty))) = (value, context.break_value.clone()) {
                    let value = self.lower_expr_value(value)?;
                    self.emit_store_to_addr(value, addr, ty, span.clone())?;
                }
                self.builder.set_terminator(IrTerminator::Br {
                    target: context.break_bb,
                })?;
                Ok(self.unit_value())
            }
            ThirExprKind::Continue => {
                let context = self.loop_stack.last().cloned().ok_or_else(|| {
                    self.error(IrLowerErrorKind::ContinueOutsideLoop, span.clone())
                })?;
                self.builder.set_terminator(IrTerminator::Br {
                    target: context.continue_bb,
                })?;
                Ok(self.unit_value())
            }
            ThirExprKind::Borrow { expr, .. } => self.lower_expr_addr(expr),
            ThirExprKind::DerefValue(base) => {
                let ptr = self.lower_expr_value(base)?;
                self.builder.emit_load(self.ir_ty(expr_ty), ptr, span)
            }
            ThirExprKind::IndexValue { base, index } => {
                let base_addr = self.lower_expr_addr(base)?;
                let base_ty = self.ir_ty(self.expr(base)?.ty);
                let index = self.lower_expr_value(index)?;
                let zero = self.builder.const_int(0, IrTy::I32);
                let elem_addr =
                    self.builder
                        .emit_gep(base_ty, base_addr, vec![zero, index], span.clone())?;
                self.builder.emit_load(self.ir_ty(expr_ty), elem_addr, span)
            }
            ThirExprKind::FieldValue { base, index } => {
                let base_addr = self.lower_expr_addr(base)?;
                let base_ty = self.ir_ty(self.expr(base)?.ty);
                let zero = self.builder.const_int(0, IrTy::I32);
                let field = self.builder.const_int(index as i32, IrTy::I32);
                let field_addr =
                    self.builder
                        .emit_gep(base_ty, base_addr, vec![zero, field], span.clone())?;
                self.builder
                    .emit_load(self.ir_ty(expr_ty), field_addr, span)
            }
            ThirExprKind::Array(_) | ThirExprKind::Tuple(_) | ThirExprKind::Range { .. } => {
                let addr = self.create_temp_slot(expr_ty, "agg", span.clone())?;
                self.lower_expr_into_addr(expr, addr, self.ir_ty(expr_ty), span.clone())?;
                self.builder.emit_load(self.ir_ty(expr_ty), addr, span)
            }
        }
    }

    fn lower_binary_expr(
        &mut self,
        op: BinaryOp,
        lhs: ThirExprId,
        rhs: ThirExprId,
        result_source_ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrValueId> {
        let lhs = self.lower_expr_value(lhs)?;
        let rhs = self.lower_expr_value(rhs)?;
        let operand_ty = IrTy::I32;

        if let Some(op) = IrBinaryOp::from_binary(op.clone()) {
            return self.builder.emit_binary(op, operand_ty, lhs, rhs, span);
        }

        let Some(pred) = IrIcmpPred::from_binary(op) else {
            return Err(self.error(
                IrLowerErrorKind::Internal {
                    message: "unsupported binary operator".to_string(),
                },
                span,
            ));
        };
        let cmp = self
            .builder
            .emit_icmp(pred, operand_ty, lhs, rhs, span.clone())?;
        let result_ty = self.ir_ty(result_source_ty);
        if result_ty == IrTy::I1 {
            Ok(cmp)
        } else {
            self.coerce_value(cmp, result_ty)
        }
    }

    fn lower_if_expr(
        &mut self,
        cond: ThirExprId,
        then_expr: ThirExprId,
        else_expr: Option<ThirExprId>,
        ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrValueId> {
        let result_ty = self.ir_ty(ty);
        let result_addr = if result_ty.is_void() {
            None
        } else {
            Some((
                self.create_temp_slot(ty, "if.result", span.clone())?,
                result_ty.clone(),
            ))
        };
        let cond = self.lower_cond_expr(cond)?;
        let then_bb = self.builder.alloc_block("if.then");
        let else_bb = self.builder.alloc_block("if.else");
        let join_bb = self.builder.alloc_block("if.end");
        let mut join_is_reachable = false;

        self.builder.set_terminator(IrTerminator::CondBr {
            cond,
            then_bb,
            else_bb,
        })?;

        self.builder.switch_to(then_bb);
        if let Some(value) = self.lower_expr_to_optional_value(then_expr)? {
            if let Some((addr, ty)) = result_addr.clone() {
                self.emit_store_to_addr(value, addr, ty, span.clone())?;
            }
        }
        if self.builder.can_emit()? {
            self.builder
                .set_terminator(IrTerminator::Br { target: join_bb })?;
            join_is_reachable = true;
        }

        self.builder.switch_to(else_bb);
        if let Some(else_expr) = else_expr {
            if let Some(value) = self.lower_expr_to_optional_value(else_expr)? {
                if let Some((addr, ty)) = result_addr.clone() {
                    self.emit_store_to_addr(value, addr, ty, span.clone())?;
                }
            }
        }
        if self.builder.can_emit()? {
            self.builder
                .set_terminator(IrTerminator::Br { target: join_bb })?;
            join_is_reachable = true;
        }

        self.builder.switch_to(join_bb);
        if !join_is_reachable {
            self.builder.set_terminator(IrTerminator::Unreachable)?;
            return Ok(self.unit_value());
        }

        match result_addr {
            Some((addr, ty)) => self.builder.emit_load(ty, addr, span),
            None => Ok(self.unit_value()),
        }
    }

    fn lower_while_expr(
        &mut self,
        cond: ThirExprId,
        body: &ThirBlock,
        _ty: TyId,
        _span: Span,
    ) -> IrLowerResult<IrValueId> {
        let cond_bb = self.builder.alloc_block("while.cond");
        let body_bb = self.builder.alloc_block("while.body");
        let exit_bb = self.builder.alloc_block("while.end");

        self.builder
            .set_terminator(IrTerminator::Br { target: cond_bb })?;

        self.builder.switch_to(cond_bb);
        let cond = self.lower_cond_expr(cond)?;
        self.builder.set_terminator(IrTerminator::CondBr {
            cond,
            then_bb: body_bb,
            else_bb: exit_bb,
        })?;

        self.builder.switch_to(body_bb);
        self.loop_stack.push(LoopContext {
            break_bb: exit_bb,
            continue_bb: cond_bb,
            break_value: None,
        });
        self.lower_block(body)?;
        self.loop_stack.pop();
        if self.builder.can_emit()? {
            self.builder
                .set_terminator(IrTerminator::Br { target: cond_bb })?;
        }

        self.builder.switch_to(exit_bb);
        Ok(self.unit_value())
    }

    fn lower_loop_expr(
        &mut self,
        body: &ThirBlock,
        ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrValueId> {
        let body_bb = self.builder.alloc_block("loop.body");
        let exit_bb = self.builder.alloc_block("loop.end");
        let result_ty = self.ir_ty(ty);
        let result_addr = if result_ty.is_void() {
            None
        } else {
            Some((
                self.create_temp_slot(ty, "loop.result", span.clone())?,
                result_ty.clone(),
            ))
        };

        self.builder
            .set_terminator(IrTerminator::Br { target: body_bb })?;
        self.builder.switch_to(body_bb);
        self.loop_stack.push(LoopContext {
            break_bb: exit_bb,
            continue_bb: body_bb,
            break_value: result_addr.clone(),
        });
        self.lower_block(body)?;
        self.loop_stack.pop();
        if self.builder.can_emit()? {
            self.builder
                .set_terminator(IrTerminator::Br { target: body_bb })?;
        }

        self.builder.switch_to(exit_bb);
        match result_addr {
            Some((addr, ty)) => self.builder.emit_load(ty, addr, span),
            None => Ok(self.unit_value()),
        }
    }

    fn lower_for_range_expr(
        &mut self,
        local: ThirLocalId,
        start: ThirExprId,
        end: ThirExprId,
        body: &ThirBlock,
        _ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrValueId> {
        let local_data = self.body.local(local).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingLocal { id: local.index() },
                span.clone(),
            )
        })?;
        let local_ty = self.ir_ty(local_data.ty);
        let local_addr = self.local_addr(local, &span)?;
        let start = self.lower_expr_value(start)?;
        let end = self.lower_expr_value(end)?;
        self.emit_store_to_addr(start, local_addr, local_ty.clone(), span.clone())?;

        let cond_bb = self.builder.alloc_block("for.cond");
        let body_bb = self.builder.alloc_block("for.body");
        let exit_bb = self.builder.alloc_block("for.end");
        self.builder
            .set_terminator(IrTerminator::Br { target: cond_bb })?;

        self.builder.switch_to(cond_bb);
        let current = self
            .builder
            .emit_load(local_ty.clone(), local_addr, span.clone())?;
        let cond = self.builder.emit_icmp(
            IrIcmpPred::Slt,
            local_ty.clone(),
            current,
            end,
            span.clone(),
        )?;
        self.builder.set_terminator(IrTerminator::CondBr {
            cond,
            then_bb: body_bb,
            else_bb: exit_bb,
        })?;

        self.builder.switch_to(body_bb);
        self.loop_stack.push(LoopContext {
            break_bb: exit_bb,
            continue_bb: cond_bb,
            break_value: None,
        });
        self.lower_block(body)?;
        self.loop_stack.pop();
        if self.builder.can_emit()? {
            let current = self
                .builder
                .emit_load(local_ty.clone(), local_addr, span.clone())?;
            let one = self.builder.const_int(1, local_ty.clone());
            let next = self.builder.emit_binary(
                IrBinaryOp::Add,
                local_ty.clone(),
                current,
                one,
                span.clone(),
            )?;
            self.emit_store_to_addr(next, local_addr, local_ty, span.clone())?;
            self.builder
                .set_terminator(IrTerminator::Br { target: cond_bb })?;
        }

        self.builder.switch_to(exit_bb);
        Ok(self.unit_value())
    }

    fn lower_expr_to_optional_value(
        &mut self,
        expr: ThirExprId,
    ) -> IrLowerResult<Option<IrValueId>> {
        if !self.builder.can_emit()? {
            return Ok(None);
        }
        let value = self.lower_expr_value(expr)?;
        if self.builder.can_emit()? {
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    fn lower_cond_expr(&mut self, expr: ThirExprId) -> IrLowerResult<IrValueId> {
        let expr_data = self.expr(expr)?;
        let span = expr_data.span.clone();
        let kind = expr_data.kind.clone();
        if let ThirExprKind::Binary { op, lhs, rhs } = kind {
            if let Some(pred) = IrIcmpPred::from_binary(op) {
                let lhs = self.lower_expr_value(lhs)?;
                let rhs = self.lower_expr_value(rhs)?;
                return self.builder.emit_icmp(pred, IrTy::I32, lhs, rhs, span);
            }
        }

        let value = self.lower_expr_value(expr)?;
        self.ensure_i1(value, span)
    }

    fn ensure_i1(&mut self, value: IrValueId, span: Span) -> IrLowerResult<IrValueId> {
        let ty = self.value_ty(value)?;
        if ty.is_i1() {
            return Ok(value);
        }

        if ty == IrTy::I32 {
            let zero = self.builder.const_int(0, IrTy::I32);
            return self
                .builder
                .emit_icmp(IrIcmpPred::Ne, IrTy::I32, value, zero, span);
        }

        Err(self.error(
            IrLowerErrorKind::UnsupportedValue {
                message: format!("cannot use value of IR type {ty:?} as condition"),
            },
            span,
        ))
    }

    fn lower_expr_into_addr(
        &mut self,
        expr: ThirExprId,
        dst_addr: IrValueId,
        dst_ty: IrTy,
        span: Span,
    ) -> IrLowerResult<()> {
        if self.lower_aggregate_expr_into_addr(expr, dst_addr, span.clone())? {
            return Ok(());
        }

        let value = self.lower_expr_value(expr)?;
        self.emit_store_to_addr(value, dst_addr, dst_ty, span)
    }

    fn lower_aggregate_expr_into_addr(
        &mut self,
        expr: ThirExprId,
        dst_addr: IrValueId,
        span: Span,
    ) -> IrLowerResult<bool> {
        let expr_data = self.expr(expr)?;
        let expr_ty = expr_data.ty;
        let kind = expr_data.kind.clone();
        match kind {
            ThirExprKind::Array(elems) | ThirExprKind::Tuple(elems) => {
                self.lower_aggregate_elems_into_addr(&elems, expr_ty, dst_addr, span)?;
                Ok(true)
            }
            ThirExprKind::Range { start, end } => {
                let elems = [start, end];
                self.lower_aggregate_elems_into_addr(&elems, expr_ty, dst_addr, span)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn lower_aggregate_elems_into_addr(
        &mut self,
        elems: &[ThirExprId],
        aggregate_ty: TyId,
        dst_addr: IrValueId,
        span: Span,
    ) -> IrLowerResult<()> {
        let source_ty = self.ir_ty(aggregate_ty);
        for (index, elem) in elems.iter().enumerate() {
            let elem_ty = self.ir_ty(self.expr(*elem)?.ty);
            let zero = self.builder.const_int(0, IrTy::I32);
            let index = self.builder.const_int(index as i32, IrTy::I32);
            let elem_addr = self.builder.emit_gep(
                source_ty.clone(),
                dst_addr,
                vec![zero, index],
                span.clone(),
            )?;
            self.lower_expr_into_addr(*elem, elem_addr, elem_ty, span.clone())?;
        }

        Ok(())
    }

    fn lower_expr_addr(&mut self, expr: ThirExprId) -> IrLowerResult<IrValueId> {
        let expr_data = self.expr(expr)?;
        let span = expr_data.span.clone();
        let expr_ty = expr_data.ty;
        let kind = expr_data.kind.clone();
        match kind {
            ThirExprKind::Use(place) => self.lower_place_addr(&place),
            ThirExprKind::DerefValue(base) => self.lower_expr_value(base),
            _ => {
                let addr = self.create_temp_slot(expr_ty, "addr.tmp", span.clone())?;
                self.lower_expr_into_addr(expr, addr, self.ir_ty(expr_ty), span)?;
                Ok(addr)
            }
        }
    }

    fn load_place_value(&mut self, place: &ThirPlace, span: Span) -> IrLowerResult<IrValueId> {
        let addr = self.lower_place_addr(place)?;
        self.builder.emit_load(self.ir_ty(place.ty), addr, span)
    }

    fn lower_place_addr(&mut self, place: &ThirPlace) -> IrLowerResult<IrValueId> {
        match &place.kind {
            ThirPlaceKind::Local(local) => self.local_addr(*local, &place.span),
            ThirPlaceKind::Deref { base } => self.lower_expr_value(*base),
            ThirPlaceKind::Index { base, index } => {
                let base_addr = self.lower_place_addr(base)?;
                let base_ty = self.ir_ty(base.ty);
                let index = self.lower_expr_value(*index)?;
                let zero = self.builder.const_int(0, IrTy::I32);
                self.builder
                    .emit_gep(base_ty, base_addr, vec![zero, index], place.span.clone())
            }
            ThirPlaceKind::Field { base, index } => {
                let base_addr = self.lower_place_addr(base)?;
                let base_ty = self.ir_ty(base.ty);
                let zero = self.builder.const_int(0, IrTy::I32);
                let field = self.builder.const_int(*index as i32, IrTy::I32);
                self.builder
                    .emit_gep(base_ty, base_addr, vec![zero, field], place.span.clone())
            }
        }
    }

    fn emit_store_to_addr(
        &mut self,
        value: IrValueId,
        addr: IrValueId,
        target_ty: IrTy,
        span: Span,
    ) -> IrLowerResult<()> {
        let value = self.coerce_value(value, target_ty.clone())?;
        self.builder.emit_store(target_ty, value, addr, span)
    }

    fn coerce_value(&mut self, value: IrValueId, target_ty: IrTy) -> IrLowerResult<IrValueId> {
        let value_ty = self.value_ty(value)?;
        if value_ty == target_ty {
            return Ok(value);
        }

        match (&value_ty, &target_ty) {
            (IrTy::I1, IrTy::I32) => self.builder.emit_zext(value_ty, value, target_ty),
            (IrTy::I32, IrTy::I1) => self.ensure_i1(value, Span::default()),
            _ => Err(self.error(
                IrLowerErrorKind::UnsupportedValue {
                    message: format!("cannot coerce IR value from {value_ty:?} to {target_ty:?}"),
                },
                Span::default(),
            )),
        }
    }

    fn create_temp_slot(
        &mut self,
        source_ty: TyId,
        name: &str,
        span: Span,
    ) -> IrLowerResult<IrValueId> {
        let value_ty = self.ir_ty(source_ty);
        self.builder.emit_alloca_slot(
            IrSlot {
                thir_local: None,
                name: name.to_string(),
                mutable: true,
                source_ty,
                value_ty: value_ty.clone(),
                addr: None,
                span: span.clone(),
            },
            value_ty,
            span,
        )
    }

    fn local_addr(&self, local: ThirLocalId, span: &Span) -> IrLowerResult<IrValueId> {
        self.local_map.get(&local).copied().ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingLocal { id: local.index() },
                span.clone(),
            )
        })
    }

    fn value_ty(&self, value: IrValueId) -> IrLowerResult<IrTy> {
        self.builder
            .function
            .value(value)
            .map(|value| value.ty.clone())
            .ok_or_else(|| {
                self.error(
                    IrLowerErrorKind::UnsupportedValue {
                        message: format!("missing IR value #{:?}", value.index()),
                    },
                    Span::default(),
                )
            })
    }

    fn expr(&self, expr: ThirExprId) -> IrLowerResult<&crate::thir::node::ThirExpr> {
        self.body.expr(expr).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingExpr { id: expr.index() },
                Span::default(),
            )
        })
    }

    fn unit_value(&mut self) -> IrValueId {
        if let Some(value) = self.unit_value {
            return value;
        }
        let value = self.builder.unit_value();
        self.unit_value = Some(value);
        value
    }

    fn ir_ty(&self, ty: TyId) -> IrTy {
        ir_ty_from_source_ty(self.tys, ty)
    }

    fn error(&self, kind: IrLowerErrorKind, span: Span) -> IrLowerError {
        IrLowerError::new(kind, span)
    }
}

struct IrBuilder {
    function: IrFunction,
    current_block: IrBlockId,
    next_temp_slot: usize,
}

impl IrBuilder {
    fn new(
        owner: crate::hir::id::DefId,
        symbol_name: String,
        ret_ty: IrTy,
        param_tys: Vec<IrTy>,
    ) -> Self {
        let mut function = IrFunction::new(owner, symbol_name, ret_ty);
        for ty in param_tys {
            function.alloc_param(ty);
        }
        let entry = function.alloc_block("entry");
        function.entry = entry;
        Self {
            function,
            current_block: entry,
            next_temp_slot: 0,
        }
    }

    fn finish(self) -> IrFunction {
        self.function
    }

    fn param_value(&self, index: usize) -> Option<IrValueId> {
        self.function.params.get(index).map(|param| param.value)
    }

    fn alloc_block(&mut self, prefix: &str) -> IrBlockId {
        let label = if prefix == "entry" {
            "entry".to_string()
        } else {
            format!("{}.{}", prefix, self.function.blocks.len())
        };
        self.function.alloc_block(label)
    }

    fn switch_to(&mut self, block: IrBlockId) {
        self.current_block = block;
    }

    fn can_emit(&self) -> IrLowerResult<bool> {
        let block = self.current_block_ref()?;
        Ok(block.terminator.is_none())
    }

    fn const_int(&mut self, value: i32, ty: IrTy) -> IrValueId {
        self.function.alloc_value(IrValue {
            ty,
            kind: IrValueKind::ConstInt(value),
            name: None,
        })
    }

    fn unit_value(&mut self) -> IrValueId {
        self.function.alloc_value(IrValue {
            ty: IrTy::Void,
            kind: IrValueKind::Unit,
            name: None,
        })
    }

    fn emit_alloca_slot(
        &mut self,
        mut slot: IrSlot,
        alloc_ty: IrTy,
        span: Span,
    ) -> IrLowerResult<IrValueId> {
        let id = self.function.alloc_slot(slot.clone());
        let name = if slot.thir_local.is_some() {
            format!("{}.addr", slot.name)
        } else {
            let n = self.next_temp_slot;
            self.next_temp_slot += 1;
            format!("{}.{}.addr", slot.name, n)
        };
        let addr = self.function.alloc_value(IrValue {
            ty: IrTy::Ptr,
            kind: IrValueKind::SlotAddr(id),
            name: Some(name),
        });
        slot.addr = Some(addr);
        if let Some(slot_data) = self.function.slot_mut(id) {
            *slot_data = slot;
        }

        let instr = IrInstr {
            result: Some(addr),
            kind: IrInstrKind::Alloca { alloc_ty },
            span,
        };
        let entry = self.function.entry;
        let block = self.block_mut(entry, Span::default())?;
        block.instrs.push(instr);
        Ok(addr)
    }

    fn emit_load(&mut self, ty: IrTy, ptr: IrValueId, span: Span) -> IrLowerResult<IrValueId> {
        self.emit_instr(Some(ty.clone()), IrInstrKind::Load { ty, ptr }, span)
    }

    fn emit_store(
        &mut self,
        ty: IrTy,
        value: IrValueId,
        ptr: IrValueId,
        span: Span,
    ) -> IrLowerResult<()> {
        self.emit_void_instr(IrInstrKind::Store { ty, value, ptr }, span)
    }

    fn emit_gep(
        &mut self,
        source_ty: IrTy,
        base: IrValueId,
        indices: Vec<IrValueId>,
        span: Span,
    ) -> IrLowerResult<IrValueId> {
        self.emit_instr(
            Some(IrTy::Ptr),
            IrInstrKind::Gep {
                source_ty,
                base,
                indices,
            },
            span,
        )
    }

    fn emit_binary(
        &mut self,
        op: IrBinaryOp,
        ty: IrTy,
        lhs: IrValueId,
        rhs: IrValueId,
        span: Span,
    ) -> IrLowerResult<IrValueId> {
        self.emit_instr(
            Some(ty.clone()),
            IrInstrKind::Binary { op, ty, lhs, rhs },
            span,
        )
    }

    fn emit_icmp(
        &mut self,
        pred: IrIcmpPred,
        ty: IrTy,
        lhs: IrValueId,
        rhs: IrValueId,
        span: Span,
    ) -> IrLowerResult<IrValueId> {
        self.emit_instr(
            Some(IrTy::I1),
            IrInstrKind::Icmp { pred, ty, lhs, rhs },
            span,
        )
    }

    fn emit_zext(
        &mut self,
        from_ty: IrTy,
        value: IrValueId,
        to_ty: IrTy,
    ) -> IrLowerResult<IrValueId> {
        self.emit_instr(
            Some(to_ty.clone()),
            IrInstrKind::Zext {
                from_ty,
                value,
                to_ty,
            },
            Span::default(),
        )
    }

    fn emit_call(
        &mut self,
        callee: crate::hir::id::DefId,
        ret_ty: IrTy,
        args: Vec<(IrTy, IrValueId)>,
        span: Span,
    ) -> IrLowerResult<Option<IrValueId>> {
        if ret_ty.is_void() {
            self.emit_void_instr(
                IrInstrKind::Call {
                    callee,
                    ret_ty,
                    args,
                },
                span,
            )?;
            Ok(None)
        } else {
            let result = self.emit_instr(
                Some(ret_ty.clone()),
                IrInstrKind::Call {
                    callee,
                    ret_ty,
                    args,
                },
                span,
            )?;
            Ok(Some(result))
        }
    }

    fn emit_instr(
        &mut self,
        result_ty: Option<IrTy>,
        kind: IrInstrKind,
        span: Span,
    ) -> IrLowerResult<IrValueId> {
        let result = result_ty.map(|ty| {
            self.function.alloc_value(IrValue {
                ty,
                kind: IrValueKind::InstrResult,
                name: None,
            })
        });
        let instr = IrInstr { result, kind, span };
        self.current_block_mut(instr.span.clone())?
            .instrs
            .push(instr);
        result.ok_or_else(|| {
            IrLowerError::new(
                IrLowerErrorKind::Internal {
                    message: "void instruction was emitted through emit_instr".to_string(),
                },
                Span::default(),
            )
        })
    }

    fn emit_void_instr(&mut self, kind: IrInstrKind, span: Span) -> IrLowerResult<()> {
        let instr = IrInstr {
            result: None,
            kind,
            span,
        };
        self.current_block_mut(instr.span.clone())?
            .instrs
            .push(instr);
        Ok(())
    }

    fn set_terminator(&mut self, terminator: IrTerminator) -> IrLowerResult<()> {
        if let IrTerminator::CondBr { cond, .. } = &terminator {
            let cond_ty = self.function.value(*cond).map(|value| value.ty.clone());
            if cond_ty != Some(IrTy::I1) {
                return Err(IrLowerError::new(
                    IrLowerErrorKind::Internal {
                        message: "conditional branch condition must be i1".to_string(),
                    },
                    Span::default(),
                ));
            }
        }
        if let IrTerminator::Ret { ty, value } = &terminator {
            match (ty.is_void(), value) {
                (true, None) => {}
                (true, Some(_)) => {
                    return Err(IrLowerError::new(
                        IrLowerErrorKind::Internal {
                            message: "void return cannot carry a value".to_string(),
                        },
                        Span::default(),
                    ));
                }
                (false, Some(value)) => {
                    let value_ty = self.function.value(*value).map(|value| value.ty.clone());
                    if value_ty.as_ref() != Some(ty) {
                        return Err(IrLowerError::new(
                            IrLowerErrorKind::Internal {
                                message: format!(
                                    "return value type mismatch: expected {ty:?}, found {value_ty:?}"
                                ),
                            },
                            Span::default(),
                        ));
                    }
                }
                (false, None) => {
                    return Err(IrLowerError::new(
                        IrLowerErrorKind::Internal {
                            message: format!("non-void return of type {ty:?} needs a value"),
                        },
                        Span::default(),
                    ));
                }
            }
        }

        let block = self.current_block_mut(Span::default())?;
        if block.terminator.is_some() {
            return Err(IrLowerError::new(
                IrLowerErrorKind::Internal {
                    message: "basic block already has a terminator".to_string(),
                },
                Span::default(),
            ));
        }
        block.terminator = Some(terminator);
        Ok(())
    }

    fn validate(&self) -> IrLowerResult<()> {
        for (index, block) in self.function.blocks.iter().enumerate() {
            if block.terminator.is_none() {
                return Err(IrLowerError::new(
                    IrLowerErrorKind::MissingBlock { id: index },
                    Span::default(),
                ));
            }
        }
        Ok(())
    }

    fn current_block_ref(&self) -> IrLowerResult<&IrBasicBlock> {
        self.function.block(self.current_block).ok_or_else(|| {
            IrLowerError::new(
                IrLowerErrorKind::MissingBlock {
                    id: self.current_block.index(),
                },
                Span::default(),
            )
        })
    }

    fn current_block_mut(&mut self, span: Span) -> IrLowerResult<&mut IrBasicBlock> {
        self.block_mut(self.current_block, span)
    }

    fn block_mut(&mut self, block: IrBlockId, span: Span) -> IrLowerResult<&mut IrBasicBlock> {
        self.function.block_mut(block).ok_or_else(|| {
            IrLowerError::new(IrLowerErrorKind::MissingBlock { id: block.index() }, span)
        })
    }
}

fn ir_ty_from_source_ty(tys: &TyStore, ty: TyId) -> IrTy {
    match tys.kind(ty) {
        TyKind::Int => IrTy::I32,
        TyKind::Unit => IrTy::Void,
        TyKind::Never => IrTy::Void,
        TyKind::Tuple(elems) => IrTy::Struct(
            elems
                .iter()
                .map(|elem| ir_ty_from_source_ty(tys, *elem))
                .collect(),
        ),
        TyKind::Array { elem, len } => IrTy::Array {
            elem: Box::new(ir_ty_from_source_ty(tys, *elem)),
            len: *len,
        },
        TyKind::Ref { .. } => IrTy::Ptr,
        TyKind::Fn { .. } => IrTy::Ptr,
        TyKind::Infer(_) | TyKind::Error => IrTy::Error,
    }
}

fn llvm_symbol_name(name: &str, owner: crate::hir::id::DefId) -> String {
    if is_c_style_symbol_name(name) {
        name.to_string()
    } else {
        format!("fn{}", owner.index())
    }
}

fn is_c_style_symbol_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}
