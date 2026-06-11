use std::collections::HashMap;

use crate::{
    ir::{
        error::{IrLowerError, IrLowerErrorKind},
        id::{IrBlockId, IrLocalId, IrTempId},
        node::{
            IrFunction, IrLocal, IrOperand, IrPlace, IrProgram, IrTemp, Quad, QuadOp, Terminator,
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
    break_value: Option<IrTempId>,
}

/// 四元式 IR lowering 上下文。
///
/// 这一阶段只负责把树状 THIR 线性化为基本块和四元式，不重新做名字解析或类型检查。
pub struct IrLowerCtx<'thir> {
    thir: &'thir ThirProgram,
    tys: &'thir TyStore,

    program: IrProgram,
    current_fn: Option<IrFunction>,
    current_block: Option<IrBlockId>,
    local_map: HashMap<ThirLocalId, IrLocalId>,
    loop_stack: Vec<LoopContext>,
    errors: Vec<IrLowerError>,
}

impl<'thir> IrLowerCtx<'thir> {
    pub fn new(thir: &'thir ThirProgram, tys: &'thir TyStore) -> Self {
        Self {
            thir,
            tys,
            program: IrProgram::new(),
            current_fn: None,
            current_block: None,
            local_map: HashMap::new(),
            loop_stack: vec![],
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
            if let Err(error) = self.lower_function(ThirBodyId(index)) {
                self.emit_error(error);
            }
        }
    }

    fn lower_function(&mut self, body_id: ThirBodyId) -> IrLowerResult<()> {
        let body = self.thir.body(body_id).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingBody {
                    id: body_id.index(),
                },
                Span::default(),
            )
        })?;

        self.current_fn = Some(IrFunction::new(body.owner));
        self.current_block = None;
        self.local_map.clear();
        self.loop_stack.clear();

        let entry = self.alloc_block()?;
        self.function_mut()?.entry = entry;
        self.current_block = Some(entry);

        for (index, local) in body.locals.iter().enumerate() {
            let thir_local = ThirLocalId(index);
            let ir_local = self.function_mut()?.alloc_local(IrLocal {
                thir_local: Some(thir_local),
                name: local.name.clone(),
                mutable: local.mutable,
                ty: local.ty,
                span: local.span.clone(),
            });
            self.local_map.insert(thir_local, ir_local);
        }

        for (index, local) in body.locals.iter().enumerate() {
            let ir_local = self.ir_local(ThirLocalId(index), &local.span)?;
            self.emit_quad(Quad::new(
                QuadOp::Alloca,
                None,
                None,
                Some(IrPlace::Local(ir_local)),
                local.span.clone(),
            ))?;
        }

        for (index, param) in body.params.iter().enumerate() {
            let local = body.local(*param).ok_or_else(|| {
                self.error(
                    IrLowerErrorKind::MissingLocal { id: param.index() },
                    Span::default(),
                )
            })?;
            let ir_local = self.ir_local(*param, &local.span)?;
            self.emit_quad(Quad::new(
                QuadOp::Store,
                Some(IrOperand::Param(index)),
                None,
                Some(IrPlace::Local(ir_local)),
                local.span.clone(),
            ))?;
        }

        let value = self.lower_expr(body, body.value)?;
        if self.can_emit()? {
            if self.is_unit_ty(body.expr(body.value).map(|expr| expr.ty)) {
                self.set_terminator(Terminator::Return(None))?;
            } else {
                self.set_terminator(Terminator::Return(Some(value)))?;
            }
        }

        let function = self
            .current_fn
            .take()
            .ok_or_else(|| self.error(IrLowerErrorKind::MissingCurrentFunction, Span::default()))?;
        self.program.alloc_function(body.owner, function);
        self.current_block = None;
        self.local_map.clear();

        Ok(())
    }

    fn lower_block(
        &mut self,
        body: &ThirBody,
        block: &ThirBlock,
    ) -> IrLowerResult<Option<IrOperand>> {
        for &stmt in &block.stmts {
            self.lower_stmt(body, stmt)?;
        }

        match block.expr {
            Some(expr) if self.can_emit()? => Ok(Some(self.lower_expr(body, expr)?)),
            Some(_) => Ok(None),
            None => Ok(None),
        }
    }

    fn lower_stmt(&mut self, body: &ThirBody, stmt: ThirStmtId) -> IrLowerResult<()> {
        if !self.can_emit()? {
            return Ok(());
        }

        let stmt_data = body.stmt(stmt).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingStmt { id: stmt.index() },
                Span::default(),
            )
        })?;

        match &stmt_data.kind {
            ThirStmtKind::Let { local, init } => {
                if let Some(init) = init {
                    let place = IrPlace::Local(self.ir_local(*local, &stmt_data.span)?);
                    self.lower_expr_into_place(body, *init, place, stmt_data.span.clone())?;
                }
            }
            ThirStmtKind::Expr(expr) | ThirStmtKind::Semi(expr) => {
                self.lower_expr(body, *expr)?;
            }
            ThirStmtKind::Empty => {}
        }

        Ok(())
    }

    fn lower_expr(&mut self, body: &ThirBody, expr: ThirExprId) -> IrLowerResult<IrOperand> {
        let expr_data = body.expr(expr).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingExpr { id: expr.index() },
                Span::default(),
            )
        })?;
        let span = expr_data.span.clone();

        match &expr_data.kind {
            ThirExprKind::Int(value) => Ok(IrOperand::ConstInt(*value)),
            ThirExprKind::Use(place) => self.load_place(body, place, span),
            ThirExprKind::Binary { op, lhs, rhs } => {
                let lhs = self.lower_expr(body, *lhs)?;
                let rhs = self.lower_expr(body, *rhs)?;
                let temp = self.alloc_temp(expr_data.ty)?;
                self.emit_quad(Quad::new(
                    QuadOp::from_binary(op.clone()),
                    Some(lhs),
                    Some(rhs),
                    Some(IrPlace::Temp(temp)),
                    span,
                ))?;
                Ok(IrOperand::Temp(temp))
            }
            ThirExprKind::Call { callee, args } => {
                for (index, arg) in args.iter().enumerate() {
                    let value = self.lower_expr(body, *arg)?;
                    self.emit_quad(Quad::new(
                        QuadOp::Arg,
                        Some(value),
                        Some(IrOperand::ConstInt(index as i32)),
                        None,
                        span.clone(),
                    ))?;
                }

                let result = if self.is_value_ty(expr_data.ty) {
                    Some(self.alloc_temp(expr_data.ty)?)
                } else {
                    None
                };
                self.emit_quad(Quad::new(
                    QuadOp::Call(*callee),
                    Some(IrOperand::ConstInt(args.len() as i32)),
                    None,
                    result.map(IrPlace::Temp),
                    span,
                ))?;
                Ok(result
                    .map(IrOperand::Temp)
                    .unwrap_or(self.unit_temp(expr_data.ty)?))
            }
            ThirExprKind::Assign { target, value } => {
                let target = self.lower_place(body, target)?;
                self.lower_expr_into_place(body, *value, target, span)?;
                Ok(self.unit_temp(expr_data.ty)?)
            }
            ThirExprKind::Block(block) => {
                let value = self.lower_block(body, block)?;
                Ok(value.unwrap_or(self.unit_temp(expr_data.ty)?))
            }
            ThirExprKind::If {
                cond,
                then_expr,
                else_expr,
            } => self.lower_if_expr(body, *cond, *then_expr, *else_expr, expr_data.ty, span),
            ThirExprKind::While {
                cond,
                body: loop_body,
            } => self.lower_while_expr(body, *cond, loop_body, expr_data.ty, span),
            ThirExprKind::Loop { body: loop_body } => {
                self.lower_loop_expr(body, loop_body, expr_data.ty, span)
            }
            ThirExprKind::ForRange {
                local,
                start,
                end,
                body: loop_body,
            } => {
                self.lower_for_range_expr(body, *local, *start, *end, loop_body, expr_data.ty, span)
            }
            ThirExprKind::Return(value) => {
                let value = match value {
                    Some(value) => Some(self.lower_expr(body, *value)?),
                    None => None,
                };
                self.set_terminator(Terminator::Return(value))?;
                Ok(self.unit_temp(expr_data.ty)?)
            }
            ThirExprKind::Break(value) => {
                let context =
                    self.loop_stack.last().cloned().ok_or_else(|| {
                        self.error(IrLowerErrorKind::BreakOutsideLoop, span.clone())
                    })?;
                if let Some(value) = value {
                    let value = self.lower_expr(body, *value)?;
                    if let Some(temp) = context.break_value {
                        self.emit_quad(Quad::new(
                            QuadOp::Store,
                            Some(value),
                            None,
                            Some(IrPlace::Temp(temp)),
                            span.clone(),
                        ))?;
                    }
                }
                self.set_terminator(Terminator::Goto(context.break_bb))?;
                Ok(self.unit_temp(expr_data.ty)?)
            }
            ThirExprKind::Continue => {
                let context = self.loop_stack.last().cloned().ok_or_else(|| {
                    self.error(IrLowerErrorKind::ContinueOutsideLoop, span.clone())
                })?;
                self.set_terminator(Terminator::Goto(context.continue_bb))?;
                Ok(self.unit_temp(expr_data.ty)?)
            }
            ThirExprKind::Borrow { mutable, expr } => {
                self.lower_borrow_expr(body, *expr, *mutable, expr_data.ty, span)
            }
            ThirExprKind::DerefValue(base) => {
                let base = self.lower_expr(body, *base)?;
                let addr = self.operand_to_place(base, &span)?;
                if self.is_aggregate_ty(expr_data.ty) {
                    Ok(self.place_to_operand(addr))
                } else {
                    self.emit_load(addr, expr_data.ty, span)
                }
            }
            ThirExprKind::IndexValue { base, index } => {
                let base = self.lower_expr(body, *base)?;
                let index = self.lower_expr(body, *index)?;
                let base = self.operand_to_place(base, &span)?;
                let addr = self.emit_gep(base, index, expr_data.ty, span.clone())?;
                if self.is_aggregate_ty(expr_data.ty) {
                    Ok(self.place_to_operand(addr))
                } else {
                    self.emit_load(addr, expr_data.ty, span)
                }
            }
            ThirExprKind::FieldValue { base, index } => {
                let base = self.lower_expr(body, *base)?;
                let base = self.operand_to_place(base, &span)?;
                let addr = self.emit_gep(
                    base,
                    IrOperand::ConstInt(*index as i32),
                    expr_data.ty,
                    span.clone(),
                )?;
                if self.is_aggregate_ty(expr_data.ty) {
                    Ok(self.place_to_operand(addr))
                } else {
                    self.emit_load(addr, expr_data.ty, span)
                }
            }
            ThirExprKind::Array(elems) => self.lower_aggregate(body, elems, expr_data.ty, span),
            ThirExprKind::Tuple(elems) => self.lower_aggregate(body, elems, expr_data.ty, span),
            ThirExprKind::Range { start, end } => {
                let elems = [*start, *end];
                self.lower_aggregate(body, &elems, expr_data.ty, span)
            }
        }
    }

    fn lower_if_expr(
        &mut self,
        body: &ThirBody,
        cond: ThirExprId,
        then_expr: ThirExprId,
        else_expr: Option<ThirExprId>,
        ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrOperand> {
        let cond = self.lower_expr(body, cond)?;
        let then_bb = self.alloc_block()?;
        let else_bb = self.alloc_block()?;
        let join_bb = self.alloc_block()?;
        let result = if self.is_value_ty(ty) {
            let temp = self.alloc_temp(ty)?;
            self.emit_quad(Quad::new(
                QuadOp::Alloca,
                None,
                None,
                Some(IrPlace::Temp(temp)),
                span.clone(),
            ))?;
            Some(temp)
        } else {
            None
        };

        self.set_terminator(Terminator::If {
            cond,
            then_bb,
            else_bb,
        })?;

        self.current_block = Some(then_bb);
        if let Some(value) = self.lower_expr_to_optional_operand(body, then_expr)? {
            if let Some(result) = result {
                self.emit_quad(Quad::new(
                    QuadOp::Store,
                    Some(value),
                    None,
                    Some(IrPlace::Temp(result)),
                    span.clone(),
                ))?;
            }
        }
        if self.can_emit()? {
            self.set_terminator(Terminator::Goto(join_bb))?;
        }

        self.current_block = Some(else_bb);
        if let Some(else_expr) = else_expr {
            if let Some(value) = self.lower_expr_to_optional_operand(body, else_expr)? {
                if let Some(result) = result {
                    self.emit_quad(Quad::new(
                        QuadOp::Store,
                        Some(value),
                        None,
                        Some(IrPlace::Temp(result)),
                        span.clone(),
                    ))?;
                }
            }
        }
        if self.can_emit()? {
            self.set_terminator(Terminator::Goto(join_bb))?;
        }

        self.current_block = Some(join_bb);
        match result {
            Some(result) => self.emit_load(IrPlace::Temp(result), ty, span),
            None => Ok(self.unit_temp(ty)?),
        }
    }

    fn lower_while_expr(
        &mut self,
        body: &ThirBody,
        cond: ThirExprId,
        loop_body: &ThirBlock,
        ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrOperand> {
        let cond_bb = self.alloc_block()?;
        let body_bb = self.alloc_block()?;
        let exit_bb = self.alloc_block()?;

        self.set_terminator(Terminator::Goto(cond_bb))?;

        self.current_block = Some(cond_bb);
        let cond = self.lower_expr(body, cond)?;
        self.set_terminator(Terminator::If {
            cond,
            then_bb: body_bb,
            else_bb: exit_bb,
        })?;

        self.current_block = Some(body_bb);
        self.loop_stack.push(LoopContext {
            break_bb: exit_bb,
            continue_bb: cond_bb,
            break_value: None,
        });
        self.lower_block(body, loop_body)?;
        self.loop_stack.pop();
        if self.can_emit()? {
            self.set_terminator(Terminator::Goto(cond_bb))?;
        }

        self.current_block = Some(exit_bb);
        Ok(self.unit_temp_with_span(ty, span)?)
    }

    fn lower_loop_expr(
        &mut self,
        body: &ThirBody,
        loop_body: &ThirBlock,
        ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrOperand> {
        let body_bb = self.alloc_block()?;
        let exit_bb = self.alloc_block()?;
        let result = if self.is_value_ty(ty) {
            let temp = self.alloc_temp(ty)?;
            self.emit_quad(Quad::new(
                QuadOp::Alloca,
                None,
                None,
                Some(IrPlace::Temp(temp)),
                span.clone(),
            ))?;
            Some(temp)
        } else {
            None
        };

        self.set_terminator(Terminator::Goto(body_bb))?;
        self.current_block = Some(body_bb);
        self.loop_stack.push(LoopContext {
            break_bb: exit_bb,
            continue_bb: body_bb,
            break_value: result,
        });
        self.lower_block(body, loop_body)?;
        self.loop_stack.pop();
        if self.can_emit()? {
            self.set_terminator(Terminator::Goto(body_bb))?;
        }

        self.current_block = Some(exit_bb);
        match result {
            Some(result) => self.emit_load(IrPlace::Temp(result), ty, span),
            None => Ok(self.unit_temp_with_span(ty, span)?),
        }
    }

    fn lower_for_range_expr(
        &mut self,
        body: &ThirBody,
        local: ThirLocalId,
        start: ThirExprId,
        end: ThirExprId,
        loop_body: &ThirBlock,
        ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrOperand> {
        let local_place = IrPlace::Local(self.ir_local(local, &span)?);
        let local_ty = body.local(local).map(|local| local.ty).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingLocal { id: local.index() },
                span.clone(),
            )
        })?;
        let start = self.lower_expr(body, start)?;
        let end = self.lower_expr(body, end)?;
        self.emit_quad(Quad::new(
            QuadOp::Store,
            Some(start),
            None,
            Some(local_place.clone()),
            span.clone(),
        ))?;

        let cond_bb = self.alloc_block()?;
        let body_bb = self.alloc_block()?;
        let exit_bb = self.alloc_block()?;
        self.set_terminator(Terminator::Goto(cond_bb))?;

        self.current_block = Some(cond_bb);
        let current = self.emit_load(local_place.clone(), local_ty, span.clone())?;
        let cond_temp = self.alloc_temp(local_ty)?;
        self.emit_quad(Quad::new(
            QuadOp::Lt,
            Some(current),
            Some(end),
            Some(IrPlace::Temp(cond_temp)),
            span.clone(),
        ))?;
        self.set_terminator(Terminator::If {
            cond: IrOperand::Temp(cond_temp),
            then_bb: body_bb,
            else_bb: exit_bb,
        })?;

        self.current_block = Some(body_bb);
        self.loop_stack.push(LoopContext {
            break_bb: exit_bb,
            continue_bb: cond_bb,
            break_value: None,
        });
        self.lower_block(body, loop_body)?;
        self.loop_stack.pop();
        if self.can_emit()? {
            let current = self.emit_load(local_place.clone(), local_ty, span.clone())?;
            let next = self.alloc_temp(local_ty)?;
            self.emit_quad(Quad::new(
                QuadOp::Add,
                Some(current),
                Some(IrOperand::ConstInt(1)),
                Some(IrPlace::Temp(next)),
                span.clone(),
            ))?;
            self.emit_quad(Quad::new(
                QuadOp::Store,
                Some(IrOperand::Temp(next)),
                None,
                Some(local_place),
                span.clone(),
            ))?;
            self.set_terminator(Terminator::Goto(cond_bb))?;
        }

        self.current_block = Some(exit_bb);
        Ok(self.unit_temp_with_span(ty, span)?)
    }

    fn lower_expr_to_optional_operand(
        &mut self,
        body: &ThirBody,
        expr: ThirExprId,
    ) -> IrLowerResult<Option<IrOperand>> {
        if !self.can_emit()? {
            return Ok(None);
        }
        let value = self.lower_expr(body, expr)?;
        if self.can_emit()? {
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    fn lower_aggregate(
        &mut self,
        body: &ThirBody,
        elems: &[ThirExprId],
        ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrOperand> {
        let temp = self.alloc_temp(ty)?;
        self.emit_quad(Quad::new(
            QuadOp::Alloca,
            None,
            None,
            Some(IrPlace::Temp(temp)),
            span.clone(),
        ))?;
        self.lower_aggregate_elems_into_place(body, elems, IrPlace::Temp(temp), span)?;
        Ok(IrOperand::Temp(temp))
    }

    fn lower_expr_into_place(
        &mut self,
        body: &ThirBody,
        expr: ThirExprId,
        place: IrPlace,
        span: Span,
    ) -> IrLowerResult<()> {
        if self.lower_aggregate_expr_into_place(body, expr, place.clone(), span.clone())? {
            return Ok(());
        }

        let value = self.lower_expr(body, expr)?;
        self.emit_store(value, place, span)
    }

    fn lower_aggregate_expr_into_place(
        &mut self,
        body: &ThirBody,
        expr: ThirExprId,
        place: IrPlace,
        span: Span,
    ) -> IrLowerResult<bool> {
        let expr_data = body.expr(expr).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingExpr { id: expr.index() },
                Span::default(),
            )
        })?;

        match &expr_data.kind {
            ThirExprKind::Array(elems) | ThirExprKind::Tuple(elems) => {
                self.lower_aggregate_elems_into_place(body, elems, place, span)?;
                Ok(true)
            }
            ThirExprKind::Range { start, end } => {
                let elems = [*start, *end];
                self.lower_aggregate_elems_into_place(body, &elems, place, span)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn lower_aggregate_elems_into_place(
        &mut self,
        body: &ThirBody,
        elems: &[ThirExprId],
        place: IrPlace,
        span: Span,
    ) -> IrLowerResult<()> {
        for (index, elem) in elems.iter().enumerate() {
            let elem_data = body.expr(*elem).ok_or_else(|| {
                self.error(
                    IrLowerErrorKind::MissingExpr { id: elem.index() },
                    Span::default(),
                )
            })?;
            let elem_addr = self.emit_gep(
                place.clone(),
                IrOperand::ConstInt(index as i32),
                elem_data.ty,
                elem_data.span.clone(),
            )?;
            self.lower_expr_into_place(body, *elem, elem_addr, span.clone())?;
        }

        Ok(())
    }

    fn load_place(
        &mut self,
        body: &ThirBody,
        place: &ThirPlace,
        span: Span,
    ) -> IrLowerResult<IrOperand> {
        let addr = self.lower_place(body, place)?;
        if self.is_aggregate_ty(place.ty) {
            Ok(self.place_to_operand(addr))
        } else {
            self.emit_load(addr, place.ty, span)
        }
    }

    fn lower_place(&mut self, body: &ThirBody, place: &ThirPlace) -> IrLowerResult<IrPlace> {
        match &place.kind {
            ThirPlaceKind::Local(local) => Ok(IrPlace::Local(self.ir_local(*local, &place.span)?)),
            ThirPlaceKind::Deref { base } => {
                let base = self.lower_expr(body, *base)?;
                self.operand_to_place(base, &place.span)
            }
            ThirPlaceKind::Index { base, index } => {
                let base = self.lower_place(body, base)?;
                let index = self.lower_expr(body, *index)?;
                self.emit_gep(base, index, place.ty, place.span.clone())
            }
            ThirPlaceKind::Field { base, index } => {
                let base = self.lower_place(body, base)?;
                self.emit_gep(
                    base,
                    IrOperand::ConstInt(*index as i32),
                    place.ty,
                    place.span.clone(),
                )
            }
        }
    }

    fn lower_borrow_expr(
        &mut self,
        body: &ThirBody,
        expr: ThirExprId,
        _mutable: bool,
        ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrOperand> {
        let expr_data = body.expr(expr).ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingExpr { id: expr.index() },
                Span::default(),
            )
        })?;

        match &expr_data.kind {
            ThirExprKind::Use(place) => {
                let place = self.lower_place(body, place)?;
                Ok(self.place_to_operand(place))
            }
            ThirExprKind::DerefValue(base) => self.lower_expr(body, *base),
            _ => {
                let value = self.lower_expr(body, expr)?;
                let temp = self.alloc_temp(ty)?;
                let place = IrPlace::Temp(temp);
                self.emit_quad(Quad::new(
                    QuadOp::Alloca,
                    None,
                    None,
                    Some(place.clone()),
                    span.clone(),
                ))?;
                self.emit_store(value, place, span)?;
                Ok(IrOperand::Temp(temp))
            }
        }
    }

    fn emit_load(&mut self, addr: IrPlace, ty: TyId, span: Span) -> IrLowerResult<IrOperand> {
        let temp = self.alloc_temp(ty)?;
        self.emit_quad(Quad::new(
            QuadOp::Load,
            Some(self.place_to_operand(addr)),
            None,
            Some(IrPlace::Temp(temp)),
            span,
        ))?;
        Ok(IrOperand::Temp(temp))
    }

    fn emit_store(&mut self, value: IrOperand, place: IrPlace, span: Span) -> IrLowerResult<()> {
        self.emit_quad(Quad::new(
            QuadOp::Store,
            Some(value),
            None,
            Some(place),
            span,
        ))
    }

    fn emit_gep(
        &mut self,
        base: IrPlace,
        index: IrOperand,
        ty: TyId,
        span: Span,
    ) -> IrLowerResult<IrPlace> {
        let temp = self.alloc_temp(ty)?;
        self.emit_quad(Quad::new(
            QuadOp::Gep,
            Some(self.place_to_operand(base)),
            Some(index),
            Some(IrPlace::Temp(temp)),
            span,
        ))?;
        Ok(IrPlace::Temp(temp))
    }

    fn place_to_operand(&self, place: IrPlace) -> IrOperand {
        match place {
            IrPlace::Local(local) => IrOperand::Local(local),
            IrPlace::Temp(temp) => IrOperand::Temp(temp),
        }
    }

    fn operand_to_place(&self, operand: IrOperand, span: &Span) -> IrLowerResult<IrPlace> {
        match operand {
            IrOperand::Local(local) => Ok(IrPlace::Local(local)),
            IrOperand::Temp(temp) => Ok(IrPlace::Temp(temp)),
            IrOperand::ConstInt(_) | IrOperand::Param(_) => Err(self.error(
                IrLowerErrorKind::InvalidPlace {
                    message: "expected an address operand, found a non-address value".to_string(),
                },
                span.clone(),
            )),
        }
    }

    fn ir_local(&self, local: ThirLocalId, span: &Span) -> IrLowerResult<IrLocalId> {
        self.local_map.get(&local).copied().ok_or_else(|| {
            self.error(
                IrLowerErrorKind::MissingLocal { id: local.index() },
                span.clone(),
            )
        })
    }

    fn alloc_block(&mut self) -> IrLowerResult<IrBlockId> {
        Ok(self.function_mut()?.alloc_block())
    }

    fn alloc_temp(&mut self, ty: TyId) -> IrLowerResult<IrTempId> {
        Ok(self.function_mut()?.alloc_temp(IrTemp { ty }))
    }

    fn emit_quad(&mut self, quad: Quad) -> IrLowerResult<()> {
        let block = self.current_block()?;
        let function = self.function_mut()?;
        let block = function.block_mut(block).ok_or_else(|| {
            IrLowerError::new(
                IrLowerErrorKind::MissingBlock { id: block.index() },
                quad.span.clone(),
            )
        })?;
        block.quads.push(quad);
        Ok(())
    }

    fn set_terminator(&mut self, terminator: Terminator) -> IrLowerResult<()> {
        let block = self.current_block()?;
        let function = self.function_mut()?;
        let block_data = function.block_mut(block).ok_or_else(|| {
            IrLowerError::new(
                IrLowerErrorKind::MissingBlock { id: block.index() },
                Span::default(),
            )
        })?;
        block_data.terminator = terminator;
        Ok(())
    }

    fn can_emit(&self) -> IrLowerResult<bool> {
        let block = self.current_block()?;
        let function = self.function_ref()?;
        let Some(block) = function.block(block) else {
            return Err(self.error(
                IrLowerErrorKind::MissingBlock { id: block.index() },
                Span::default(),
            ));
        };
        Ok(matches!(block.terminator, Terminator::Unreachable))
    }

    fn unit_temp(&mut self, ty: TyId) -> IrLowerResult<IrOperand> {
        self.unit_temp_with_span(ty, Span::default())
    }

    fn unit_temp_with_span(&mut self, ty: TyId, _span: Span) -> IrLowerResult<IrOperand> {
        let _ = ty;
        Ok(IrOperand::ConstInt(0))
    }

    fn is_unit_ty(&self, ty: Option<TyId>) -> bool {
        ty.map(|ty| matches!(self.tys.kind(ty), TyKind::Unit))
            .unwrap_or(false)
    }

    fn is_value_ty(&self, ty: TyId) -> bool {
        !matches!(
            self.tys.kind(ty),
            TyKind::Unit | TyKind::Never | TyKind::Error
        )
    }

    fn is_aggregate_ty(&self, ty: TyId) -> bool {
        matches!(self.tys.kind(ty), TyKind::Array { .. } | TyKind::Tuple(_))
    }

    fn current_block(&self) -> IrLowerResult<IrBlockId> {
        self.current_block
            .ok_or_else(|| self.error(IrLowerErrorKind::MissingCurrentBlock, Span::default()))
    }

    fn function_ref(&self) -> IrLowerResult<&IrFunction> {
        self.current_fn
            .as_ref()
            .ok_or_else(|| self.error(IrLowerErrorKind::MissingCurrentFunction, Span::default()))
    }

    fn function_mut(&mut self) -> IrLowerResult<&mut IrFunction> {
        self.current_fn.as_mut().ok_or_else(|| {
            IrLowerError::new(IrLowerErrorKind::MissingCurrentFunction, Span::default())
        })
    }

    fn error(&self, kind: IrLowerErrorKind, span: Span) -> IrLowerError {
        IrLowerError::new(kind, span)
    }

    fn emit_error(&mut self, error: IrLowerError) {
        self.errors.push(error);
    }
}
