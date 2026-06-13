use std::collections::HashMap;

use crate::{
    hir::{
        id::{DefId, HirBodyId, HirExprId, HirItemId, HirStmtId, LocalId},
        node::{
            HirBlock, HirExprKind, HirFnSig, HirItemKind, HirPat, HirPatKind, HirProgram,
            HirStmtKind,
        },
        res::Res,
        table::{DefTable, LocalTable},
    },
    lexer::token::Span,
    thir::{
        error::{ThirLowerError, ThirLowerErrorKind},
        id::{ThirExprId, ThirLocalId, ThirStmtId},
        node::{
            ThirBlock, ThirBody, ThirExpr, ThirExprKind, ThirLocal, ThirPat, ThirPatKind,
            ThirPlace, ThirPlaceKind, ThirProgram, ThirStmt, ThirStmtKind,
        },
        output::ThirOutput,
    },
    typecheck::{
        result::TypeckResults,
        ty::{TyId, TyStore},
    },
};

type ThirLowerResult<T> = Result<T, ThirLowerError>;

/// THIR lowering 上下文。
///
/// 这一阶段读取已经完成名字解析的 HIR 和类型检查结果，把表达式进一步整理成更接近
/// 执行语义的结构。它不重新解析名字，也不重新推导类型。
pub struct ThirLowerCtx<'hir> {
    hir: &'hir HirProgram,
    defs: &'hir DefTable,
    locals: &'hir LocalTable,
    typeck: &'hir TypeckResults,
    tys: &'hir TyStore,

    program: ThirProgram,
    errors: Vec<ThirLowerError>,

    current_body: Option<ThirBody>,
    local_map: HashMap<LocalId, ThirLocalId>,
}

impl<'hir> ThirLowerCtx<'hir> {
    pub fn new(
        hir: &'hir HirProgram,
        defs: &'hir DefTable,
        locals: &'hir LocalTable,
        typeck: &'hir TypeckResults,
        tys: &'hir TyStore,
    ) -> Self {
        Self {
            hir,
            defs,
            locals,
            typeck,
            tys,
            program: ThirProgram::new(),
            errors: vec![],
            current_body: None,
            local_map: HashMap::new(),
        }
    }

    pub fn lower(mut self) -> ThirOutput {
        self.lower_program();
        ThirOutput {
            program: self.program,
            errors: self.errors,
        }
    }

    fn lower_program(&mut self) {
        for &item in &self.hir.root_items {
            if let Err(error) = self.lower_item(item) {
                self.emit_error(error);
            }
        }
    }

    fn lower_item(&mut self, item: HirItemId) -> ThirLowerResult<()> {
        let item_data = self.hir.item(item).cloned().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::MissingItem { id: item.index() },
                Span::default(),
            )
        })?;

        match item_data.kind {
            HirItemKind::Fn(hir_fn) => self.lower_fn(item_data.def_id, hir_fn.body, &hir_fn.sig),
            HirItemKind::ExternFn(_) => Ok(()),
            HirItemKind::Struct(_) => Ok(()),
        }
    }

    fn lower_fn(&mut self, owner: DefId, body: HirBodyId, sig: &HirFnSig) -> ThirLowerResult<()> {
        if self.defs.get(owner).is_none() {
            return Err(self.error(
                ThirLowerErrorKind::Internal {
                    message: format!("definition {owner:?} is missing from DefTable"),
                },
                Span::default(),
            ));
        }

        let hir_body = self.hir.body(body).cloned().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::MissingBody { id: body.index() },
                Span::default(),
            )
        })?;

        self.current_body = Some(ThirBody::new(owner));
        self.local_map.clear();

        for param in &sig.params {
            let local = self.declare_local(
                param.local_id,
                param.name.clone(),
                param.mutable,
                param.span.clone(),
            )?;
            self.body_mut()?.params.push(local);
        }

        let value = self.lower_expr(hir_body.value)?;
        self.body_mut()?.value = value;

        let body = self.current_body.take().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::Internal {
                    message: "current THIR body disappeared while lowering function".to_string(),
                },
                Span::default(),
            )
        })?;
        self.program.alloc_body(owner, body);
        self.local_map.clear();

        Ok(())
    }

    fn lower_block(&mut self, block: &HirBlock) -> ThirLowerResult<ThirBlock> {
        let mut stmts = vec![];
        for &stmt in &block.stmts {
            stmts.push(self.lower_stmt(stmt)?);
        }

        let expr = match block.expr {
            Some(expr) => Some(self.lower_expr(expr)?),
            None => None,
        };

        Ok(ThirBlock { stmts, expr })
    }

    fn lower_stmt(&mut self, stmt: HirStmtId) -> ThirLowerResult<ThirStmtId> {
        let stmt_data = self.hir.stmt(stmt).cloned().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::MissingStmt { id: stmt.index() },
                Span::default(),
            )
        })?;

        let ty = self.stmt_ty(stmt, &stmt_data.span)?;
        let span = stmt_data.span.clone();
        let kind = match stmt_data.kind {
            HirStmtKind::Let { pat, init, .. } => {
                let init = match init {
                    Some(init) => Some(self.lower_expr(init)?),
                    None => None,
                };
                let pat = self.lower_pat(&pat)?;
                ThirStmtKind::Let { pat, init }
            }
            HirStmtKind::Expr(expr) => ThirStmtKind::Expr(self.lower_expr(expr)?),
            HirStmtKind::Semi(expr) => ThirStmtKind::Semi(self.lower_expr(expr)?),
            HirStmtKind::Empty => ThirStmtKind::Empty,
        };

        self.alloc_stmt(ThirStmt {
            kind,
            ty,
            span,
            hir_id: Some(stmt),
        })
    }

    fn lower_expr(&mut self, expr: HirExprId) -> ThirLowerResult<ThirExprId> {
        let expr_data = self.hir.expr(expr).cloned().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::MissingExpr { id: expr.index() },
                Span::default(),
            )
        })?;

        let span = expr_data.span.clone();
        let ty = self.expr_ty(expr, &span)?;
        let kind = match expr_data.kind {
            HirExprKind::Int(value) => ThirExprKind::Int(value),
            HirExprKind::Bool(value) => ThirExprKind::Bool(value),
            HirExprKind::String(value) => ThirExprKind::String(value),
            HirExprKind::StructLit { def_id, fields } => {
                let fields = fields
                    .into_iter()
                    .map(|field| {
                        let index = self.struct_field_index(def_id, &field.name, &field.span)?;
                        let expr = self.lower_expr(field.expr)?;
                        Ok((index, expr))
                    })
                    .collect::<ThirLowerResult<Vec<_>>>()?;

                ThirExprKind::StructLit { def_id, fields }
            }
            HirExprKind::Path(Res::Local(_)) => {
                let place = self.lower_place(expr)?;
                ThirExprKind::Use(place)
            }
            HirExprKind::Path(Res::Def(def_id)) => {
                return Err(self.error(ThirLowerErrorKind::DefAsValue { def_id }, span));
            }
            HirExprKind::Path(Res::Err) | HirExprKind::Err => {
                return Err(self.error(
                    ThirLowerErrorKind::InvalidValue {
                        message: "cannot lower unresolved HIR expression into THIR".to_string(),
                    },
                    span,
                ));
            }
            HirExprKind::Binary { op, lhs, rhs } => {
                let lhs = self.lower_expr(lhs)?;
                let rhs = self.lower_expr(rhs)?;
                ThirExprKind::Binary { op, lhs, rhs }
            }
            HirExprKind::Call { callee, args } => {
                let Res::Def(callee) = callee else {
                    return Err(self.error(
                        ThirLowerErrorKind::InvalidCallCallee {
                            message: format!(
                                "THIR call callee must be a definition, got {callee:?}"
                            ),
                        },
                        span,
                    ));
                };
                let args = args
                    .into_iter()
                    .map(|arg| self.lower_expr(arg))
                    .collect::<ThirLowerResult<Vec<_>>>()?;
                ThirExprKind::Call { callee, args }
            }
            HirExprKind::Assign { lhs, rhs } => {
                let target = self.lower_place(lhs)?;
                let value = self.lower_expr(rhs)?;
                ThirExprKind::Assign { target, value }
            }
            HirExprKind::Block(block) => {
                let block = self.lower_block(&block)?;
                ThirExprKind::Block(block)
            }
            HirExprKind::If {
                cond,
                then_block,
                else_expr,
            } => {
                let cond = self.lower_expr(cond)?;
                let then_ty = self.hir_block_ty(&then_block, ty);
                let then_span = span.clone();
                let then_block = self.lower_block(&then_block)?;
                let then_expr = self.alloc_expr(ThirExpr::new(
                    ThirExprKind::Block(then_block),
                    then_ty,
                    then_span,
                    None,
                ))?;
                let else_expr = match else_expr {
                    Some(else_expr) => Some(self.lower_expr(else_expr)?),
                    None => None,
                };
                ThirExprKind::If {
                    cond,
                    then_expr,
                    else_expr,
                }
            }
            HirExprKind::While { cond, body } => {
                let cond = self.lower_expr(cond)?;
                let body = self.lower_block(&body)?;
                ThirExprKind::While { cond, body }
            }
            HirExprKind::Loop { body } => {
                let body = self.lower_block(&body)?;
                ThirExprKind::Loop { body }
            }
            HirExprKind::ForRange {
                local_id,
                name,
                mutable,
                start,
                end,
                body,
                ..
            } => {
                let start = self.lower_expr(start)?;
                let end = self.lower_expr(end)?;
                let local = self.declare_local(local_id, name, mutable, span.clone())?;
                let body = self.lower_block(&body)?;
                ThirExprKind::ForRange {
                    local,
                    start,
                    end,
                    body,
                }
            }
            HirExprKind::Return(value) => {
                let value = match value {
                    Some(value) => Some(self.lower_expr(value)?),
                    None => None,
                };
                ThirExprKind::Return(value)
            }
            HirExprKind::Break(value) => {
                let value = match value {
                    Some(value) => Some(self.lower_expr(value)?),
                    None => None,
                };
                ThirExprKind::Break(value)
            }
            HirExprKind::Continue => ThirExprKind::Continue,
            HirExprKind::Borrow { mutable, expr } => {
                let expr = self.lower_expr(expr)?;
                ThirExprKind::Borrow { mutable, expr }
            }
            HirExprKind::Deref(base) => match self.lower_place(expr) {
                Ok(place) => ThirExprKind::Use(place),
                Err(error) if error.is_invalid_place() => {
                    let base = self.lower_expr(base)?;
                    ThirExprKind::DerefValue(base)
                }
                Err(error) => return Err(error),
            },
            HirExprKind::Index { base, index } => match self.lower_place(expr) {
                Ok(place) => ThirExprKind::Use(place),
                Err(error) if error.is_invalid_place() => {
                    let base = self.lower_expr(base)?;
                    let index = self.lower_expr(index)?;
                    ThirExprKind::IndexValue { base, index }
                }
                Err(error) => return Err(error),
            },
            HirExprKind::Field { base, index } => match self.lower_place(expr) {
                Ok(place) => ThirExprKind::Use(place),
                Err(error) if error.is_invalid_place() => {
                    let base = self.lower_expr(base)?;
                    ThirExprKind::FieldValue { base, index }
                }
                Err(error) => return Err(error),
            },
            HirExprKind::NamedField { base, .. } => match self.lower_place(expr) {
                Ok(place) => ThirExprKind::Use(place),
                Err(error) if error.is_invalid_place() => {
                    let index = self.field_index(expr, &span)?;
                    let base = self.lower_expr(base)?;
                    ThirExprKind::FieldValue { base, index }
                }
                Err(error) => return Err(error),
            },
            HirExprKind::Array(elems) => {
                let elems = elems
                    .into_iter()
                    .map(|elem| self.lower_expr(elem))
                    .collect::<ThirLowerResult<Vec<_>>>()?;
                ThirExprKind::Array(elems)
            }
            HirExprKind::Tuple(elems) => {
                let elems = elems
                    .into_iter()
                    .map(|elem| self.lower_expr(elem))
                    .collect::<ThirLowerResult<Vec<_>>>()?;
                ThirExprKind::Tuple(elems)
            }
            HirExprKind::Range { start, end } => {
                let start = self.lower_expr(start)?;
                let end = self.lower_expr(end)?;
                ThirExprKind::Range { start, end }
            }
        };

        self.alloc_expr(ThirExpr::new(kind, ty, span, Some(expr)))
    }

    fn lower_pat(&mut self, pat: &HirPat) -> ThirLowerResult<ThirPat> {
        let kind = match &pat.kind {
            HirPatKind::Wildcard => ThirPatKind::Wildcard,
            HirPatKind::Binding {
                local_id,
                name,
                mutable,
            } => {
                let local =
                    self.declare_local(*local_id, name.clone(), *mutable, pat.span.clone())?;
                ThirPatKind::Binding(local)
            }
            HirPatKind::Tuple(elems) => {
                let elems = elems
                    .iter()
                    .map(|elem| self.lower_pat(elem))
                    .collect::<ThirLowerResult<Vec<_>>>()?;
                ThirPatKind::Tuple(elems)
            }
            HirPatKind::Struct { def_id, fields } => {
                let fields = fields
                    .iter()
                    .map(|field| Ok((field.index, self.lower_pat(&field.pat)?)))
                    .collect::<ThirLowerResult<Vec<_>>>()?;
                ThirPatKind::Struct {
                    def_id: *def_id,
                    fields,
                }
            }
        };

        Ok(ThirPat {
            kind,
            span: pat.span.clone(),
        })
    }

    fn lower_place(&mut self, expr: HirExprId) -> ThirLowerResult<ThirPlace> {
        let expr_data = self.hir.expr(expr).cloned().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::MissingExpr { id: expr.index() },
                Span::default(),
            )
        })?;

        let span = expr_data.span.clone();
        let ty = self.expr_ty(expr, &span)?;
        let kind = match expr_data.kind {
            HirExprKind::Path(Res::Local(local)) => {
                let local = self.thir_local(local, &span)?;
                ThirPlaceKind::Local(local)
            }
            HirExprKind::Path(Res::Def(def_id)) => {
                return Err(self.error(ThirLowerErrorKind::DefAsValue { def_id }, span));
            }
            HirExprKind::Path(Res::Err) | HirExprKind::Err => {
                return Err(self.error(
                    ThirLowerErrorKind::InvalidPlace {
                        message: "unresolved expression cannot become a THIR place".to_string(),
                    },
                    span,
                ));
            }
            HirExprKind::Deref(base) => {
                let base = self.lower_expr(base)?;
                ThirPlaceKind::Deref { base }
            }
            HirExprKind::Index { base, index } => {
                let base = self.lower_place(base)?;
                let index = self.lower_expr(index)?;
                ThirPlaceKind::Index {
                    base: Box::new(base),
                    index,
                }
            }
            HirExprKind::Field { base, index } => {
                let base = self.lower_place(base)?;
                ThirPlaceKind::Field {
                    base: Box::new(base),
                    index,
                }
            }
            HirExprKind::NamedField { base, .. } => {
                let index = self.field_index(expr, &span)?;
                let base = self.lower_place(base)?;
                ThirPlaceKind::Field {
                    base: Box::new(base),
                    index,
                }
            }
            _ => {
                return Err(self.error(
                    ThirLowerErrorKind::InvalidPlace {
                        message: format!("HIR expression {expr:?} cannot become a THIR place"),
                    },
                    span,
                ));
            }
        };

        Ok(ThirPlace::new(kind, ty, span, Some(expr)))
    }

    fn declare_local(
        &mut self,
        hir_local: LocalId,
        name: String,
        mutable: bool,
        span: Span,
    ) -> ThirLowerResult<ThirLocalId> {
        if let Some(local) = self.local_map.get(&hir_local).copied() {
            return Ok(local);
        }

        if self.locals.get(hir_local).is_none() {
            return Err(self.error(
                ThirLowerErrorKind::MissingLocal {
                    id: hir_local.index(),
                },
                span,
            ));
        }

        let ty = self.local_ty(hir_local, &span)?;
        let local = ThirLocal {
            hir_local: Some(hir_local),
            name,
            mutable,
            ty,
            span,
        };
        let id = self.body_mut()?.alloc_local(local);
        self.local_map.insert(hir_local, id);
        Ok(id)
    }

    fn thir_local(&self, hir_local: LocalId, span: &Span) -> ThirLowerResult<ThirLocalId> {
        self.local_map.get(&hir_local).copied().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::MissingLocal {
                    id: hir_local.index(),
                },
                span.clone(),
            )
        })
    }

    fn expr_ty(&self, expr: HirExprId, span: &Span) -> ThirLowerResult<TyId> {
        let ty = self.typeck.get_expr_ty(expr).copied().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::MissingType {
                    node: format!("expression {expr:?}"),
                },
                span.clone(),
            )
        })?;
        let _ = self.tys.kind(ty);
        Ok(ty)
    }

    fn stmt_ty(&self, stmt: HirStmtId, span: &Span) -> ThirLowerResult<TyId> {
        let ty = self.typeck.get_stmt_ty(stmt).copied().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::MissingType {
                    node: format!("statement {stmt:?}"),
                },
                span.clone(),
            )
        })?;
        let _ = self.tys.kind(ty);
        Ok(ty)
    }

    fn local_ty(&self, local: LocalId, span: &Span) -> ThirLowerResult<TyId> {
        let ty = self.typeck.get_local_ty(local).copied().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::MissingType {
                    node: format!("local {local:?}"),
                },
                span.clone(),
            )
        })?;
        let _ = self.tys.kind(ty);
        Ok(ty)
    }

    fn hir_block_ty(&self, block: &HirBlock, default_ty: TyId) -> TyId {
        if let Some(expr) = block.expr {
            return self.typeck.get_expr_ty(expr).copied().unwrap_or(default_ty);
        }

        if let Some(stmt) = block.stmts.last() {
            return self
                .typeck
                .get_stmt_ty(*stmt)
                .copied()
                .unwrap_or(default_ty);
        }

        default_ty
    }

    fn field_index(&self, expr: HirExprId, span: &Span) -> ThirLowerResult<usize> {
        self.typeck.get_field_index(expr).copied().ok_or_else(|| {
            self.error(
                ThirLowerErrorKind::Internal {
                    message: format!("missing resolved field index for {expr:?}"),
                },
                span.clone(),
            )
        })
    }

    fn struct_field_index(&self, def_id: DefId, name: &str, span: &Span) -> ThirLowerResult<usize> {
        self.defs
            .get(def_id)
            .and_then(|def| {
                def.struct_fields
                    .iter()
                    .position(|field| field.name == name)
            })
            .ok_or_else(|| {
                self.error(
                    ThirLowerErrorKind::Internal {
                        message: format!("missing struct field `{name}` on {def_id:?}"),
                    },
                    span.clone(),
                )
            })
    }

    fn body_mut(&mut self) -> ThirLowerResult<&mut ThirBody> {
        if self.current_body.is_some() {
            Ok(self.current_body.as_mut().unwrap())
        } else {
            Err(ThirLowerError::new(
                ThirLowerErrorKind::Internal {
                    message: "THIR lowering attempted to allocate outside a body".to_string(),
                },
                Span::default(),
            ))
        }
    }

    fn alloc_stmt(&mut self, stmt: ThirStmt) -> ThirLowerResult<ThirStmtId> {
        Ok(self.body_mut()?.alloc_stmt(stmt))
    }

    fn alloc_expr(&mut self, expr: ThirExpr) -> ThirLowerResult<ThirExprId> {
        Ok(self.body_mut()?.alloc_expr(expr))
    }

    fn error(&self, kind: ThirLowerErrorKind, span: Span) -> ThirLowerError {
        ThirLowerError::new(kind, span)
    }

    fn emit_error(&mut self, error: ThirLowerError) {
        self.errors.push(error);
    }
}
