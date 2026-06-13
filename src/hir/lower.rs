use std::collections::HashMap;

use crate::{
    ast::{
        node::NodeID,
        ty::{
            Block, ElseBranch, Expr, ExprKind, FnDecl, Item, ItemKind, Param, Place, PlaceKind,
            Program, Stmt, StmtKind, StructDecl, TyKind,
        },
    },
    hir::{
        error::{HirLowerError, HirLowerErrorKind},
        id::{DefId, LocalId},
        node::{
            HirBlock, HirBody, HirExpr, HirExprKind, HirFn, HirFnSig, HirItem, HirItemKind,
            HirParam, HirPat, HirPatKind, HirProgram, HirStmt, HirStmtKind, HirStruct,
            HirStructField, HirStructLitField, HirStructPatField,
        },
        res::Res,
        scope::{ScopeDeclareError, ScopeKind, ScopeStack},
        table::{DefKind, DefTable, LocalKind, LocalTable, StructFieldData},
        ty::{HirTy, HirTyKind},
    },
    lexer::token::Span,
};

pub struct HirLoweringResult {
    pub hir: HirProgram,
    pub defs: DefTable,
    pub locals: LocalTable,
    pub errors: Vec<HirLowerError>,
}

pub struct HirLowerer<'a> {
    ast: &'a Program,

    pub hir: HirProgram,
    pub defs: DefTable,
    pub locals: LocalTable,

    scopes: ScopeStack,
    current_owner: Option<DefId>,
    item_defs: HashMap<NodeID, DefId>,

    errors: Vec<HirLowerError>,
}

impl<'a> HirLowerer<'a> {
    pub fn new(ast: &'a Program) -> Self {
        Self {
            ast,
            hir: HirProgram::new(),
            defs: DefTable::new(),
            locals: LocalTable::new(),
            scopes: ScopeStack::new(),
            current_owner: None,
            item_defs: HashMap::new(),
            errors: vec![],
        }
    }
}

impl HirLowerer<'_> {
    pub fn lower(mut self) -> HirLoweringResult {
        self.collect_defs();
        self.lower_program();
        HirLoweringResult {
            hir: self.hir,
            defs: self.defs,
            locals: self.locals,
            errors: self.errors,
        }
    }

    fn collect_defs(&mut self) {
        for item in &self.ast.items {
            match &item.kind {
                ItemKind::Fn(fn_decl) => {
                    let name = fn_decl.sig.name.kind.clone();
                    let span = item.span.clone();

                    // 若已经存在，则抛出错误，忽略当前的定义继续处理
                    if let Some((_, data)) = self.defs.get_by_names(&name) {
                        self.emit_error(
                            HirLowerErrorKind::DuplicateDef {
                                name,
                                previous: data.span.clone(),
                            },
                            span,
                        );
                        continue;
                    }

                    // 若之前未定义过，则分配一个定义
                    let id = self.defs.alloc(name, DefKind::Fn, span);
                    self.item_defs.insert(item.id, id);
                }
                ItemKind::ExternFn(fn_decl) => {
                    let name = fn_decl.sig.name.kind.clone();
                    let span = item.span.clone();

                    if let Some((_, data)) = self.defs.get_by_names(&name) {
                        self.emit_error(
                            HirLowerErrorKind::DuplicateDef {
                                name,
                                previous: data.span.clone(),
                            },
                            span,
                        );
                        continue;
                    }

                    let id = self.defs.alloc(name, DefKind::ExternFn, span);
                    self.item_defs.insert(item.id, id);
                }
                ItemKind::Struct(struct_decl) => {
                    let name = struct_decl.name.kind.clone();
                    let span = item.span.clone();

                    if let Some((_, data)) = self.defs.get_by_names(&name) {
                        self.emit_error(
                            HirLowerErrorKind::DuplicateDef {
                                name,
                                previous: data.span.clone(),
                            },
                            span,
                        );
                        continue;
                    }

                    let id = self.defs.alloc(name, DefKind::Struct, span);
                    self.item_defs.insert(item.id, id);
                }
            }
        }
    }

    fn enter_scope(&mut self, kind: ScopeKind) {
        self.scopes.enter(kind);
    }

    fn exit_scope(&mut self) {
        self.scopes.exit();
    }

    fn decalre_local(
        &mut self,
        name: String,
        mutable: bool,
        kind: LocalKind,
        span: Span,
    ) -> Result<LocalId, HirLowerErrorKind> {
        let owner = self.current_owner.ok_or(HirLowerErrorKind::Internal {
            message: format!("No current owner when declaring!"),
        })?;

        let id = self.locals.alloc(name.clone(), mutable, kind, owner, span);

        self.scopes.declare(name, id).map_err(|err| match err {
            ScopeDeclareError::NoScope => HirLowerErrorKind::Internal {
                message: format!("Working on empty scope stack!"),
            },
        })?;
        Ok(id)
    }

    fn resolve_name(&mut self, name: &str, span: Span) -> Res {
        if let Some(id) = self.scopes.resolve_local(name) {
            return Res::Local(id);
        }
        if let Some((id, _)) = self.defs.get_by_names(name) {
            return Res::Def(id);
        }

        self.emit_error(
            HirLowerErrorKind::UndefinedName {
                name: name.to_string(),
            },
            span,
        );
        Res::Err
    }

    fn resolve_callee(&mut self, name: &str, span: Span) -> Res {
        if let Some((id, _)) = self.defs.get_by_names(name) {
            return Res::Def(id);
        }
        self.emit_error(
            HirLowerErrorKind::UndefinedName {
                name: name.to_string(),
            },
            span,
        );
        Res::Err
    }

    fn emit_error(&mut self, kind: HirLowerErrorKind, span: Span) {
        self.errors.push(HirLowerError { kind, span });
    }
}

impl HirLowerer<'_> {
    fn lower_program(&mut self) {
        let items = self.ast.items.clone();
        for item in items {
            if let Some(hir_item) = self.lower_item(&item) {
                let id = self.hir.alloc_item(hir_item);
                self.hir.root_items.push(id);
            }
        }
    }

    fn lower_item(&mut self, item: &Item) -> Option<HirItem> {
        let def_id = *self.item_defs.get(&item.id)?;
        let span = item.span.clone();
        match &item.kind {
            ItemKind::Fn(fn_decl) => {
                let hir_fn = self.lower_fn(def_id, fn_decl, span.clone());
                Some(HirItem {
                    def_id,
                    span,
                    kind: HirItemKind::Fn(hir_fn),
                })
            }
            ItemKind::ExternFn(fn_decl) => {
                let name = fn_decl.sig.name.kind.clone();
                let params =
                    self.lower_extern_param_list(def_id, &fn_decl.sig.params, span.clone());
                let ret_ty = match &fn_decl.sig.ret_ty {
                    Some(ty) => self.lower_ty(ty.kind.clone(), ty.span.clone()),
                    None => HirTy::unit(span.clone()),
                };
                Some(HirItem {
                    def_id,
                    span,
                    kind: HirItemKind::ExternFn(crate::hir::node::HirExternFn {
                        name,
                        sig: HirFnSig {
                            params,
                            ret_ty,
                            variadic: fn_decl.sig.variadic,
                        },
                    }),
                })
            }
            ItemKind::Struct(struct_decl) => {
                let hir_struct = self.lower_struct(def_id, struct_decl);
                Some(HirItem {
                    def_id,
                    span,
                    kind: HirItemKind::Struct(hir_struct),
                })
            }
        }
    }

    fn lower_struct(&mut self, def_id: DefId, struct_decl: &StructDecl) -> HirStruct {
        let name = struct_decl.name.kind.clone();
        let fields = struct_decl
            .fields
            .iter()
            .map(|field| HirStructField {
                name: field.name.kind.clone(),
                ty: self.lower_ty(field.ty.kind.clone(), field.ty.span.clone()),
                span: field.name.span.clone(),
            })
            .collect::<Vec<_>>();

        let table_fields = fields
            .iter()
            .map(|field| StructFieldData {
                name: field.name.clone(),
                ty: field.ty.clone(),
                span: field.span.clone(),
            })
            .collect();
        self.defs.set_struct_fields(def_id, table_fields);

        HirStruct { name, fields }
    }

    fn lower_fn(&mut self, def_id: DefId, fn_decl: &FnDecl, span: Span) -> HirFn {
        self.current_owner = Some(def_id);
        // 进入函数作用域
        self.enter_scope(ScopeKind::Function);

        let name = fn_decl.sig.name.kind.clone();

        // 解析参数表
        let param_list = self.lower_param_list(&fn_decl.sig.params, span.clone());
        let params: Vec<_> = param_list.iter().map(|p| p.local_id).collect();

        // 解析返回类型
        let ret_ty = match &fn_decl.sig.ret_ty {
            Some(ty) => self.lower_ty(ty.kind.clone(), ty.span.clone()),
            None => HirTy::unit(span.clone()),
        };

        let sig = HirFnSig {
            params: param_list,
            ret_ty,
            variadic: fn_decl.sig.variadic,
        };

        // 解析函数体
        let block_expr = self.lower_block_expr(&fn_decl.body);
        let value = self.hir.alloc_expr(block_expr);
        let body = self.hir.alloc_body(HirBody {
            owner: self.current_owner.unwrap(),
            params,
            value,
        });

        // 退出作用域
        self.exit_scope();
        self.current_owner = None;

        HirFn { name, sig, body }
    }

    fn lower_param_list(&mut self, params: &[Param], span: Span) -> Vec<HirParam> {
        let mut param_list = vec![];
        let mut seen_params: HashMap<String, Span> = HashMap::new();
        for param in params {
            let name = param.name.kind.clone();
            if let Some(previous) = seen_params.get(&name) {
                self.emit_error(
                    HirLowerErrorKind::DuplicateParam {
                        name: name.clone(),
                        previous: previous.clone(),
                    },
                    param.name.span.clone(),
                );
            } else {
                seen_params.insert(name.clone(), param.name.span.clone());
            }

            // 添加参数到符号表中
            let local_id = match self.decalre_local(
                name.clone(),
                param.mutable,
                LocalKind::Param,
                param.name.span.clone(),
            ) {
                Ok(id) => id,
                Err(err_kind) => {
                    self.emit_error(err_kind, span.clone());
                    continue;
                }
            };

            param_list.push(HirParam {
                local_id,
                name,
                mutable: param.mutable,
                ty: self.lower_ty(param.ty.kind.clone(), param.ty.span.clone()),
                span: param.name.span.clone(),
            });
        }

        param_list
    }

    fn lower_extern_param_list(
        &mut self,
        owner: DefId,
        params: &[Param],
        span: Span,
    ) -> Vec<HirParam> {
        let mut param_list = vec![];
        let mut seen_params: HashMap<String, Span> = HashMap::new();

        for param in params {
            let name = param.name.kind.clone();
            if let Some(previous) = seen_params.get(&name) {
                self.emit_error(
                    HirLowerErrorKind::DuplicateParam {
                        name: name.clone(),
                        previous: previous.clone(),
                    },
                    param.name.span.clone(),
                );
            } else {
                seen_params.insert(name.clone(), param.name.span.clone());
            }

            let local_id = self.locals.alloc(
                name.clone(),
                param.mutable,
                LocalKind::Param,
                owner,
                param.name.span.clone(),
            );

            param_list.push(HirParam {
                local_id,
                name,
                mutable: param.mutable,
                ty: self.lower_ty(param.ty.kind.clone(), param.ty.span.clone()),
                span: span.clone(),
            });
        }

        param_list
    }

    fn lower_ty(&mut self, ast_ty_kind: TyKind, span: Span) -> HirTy {
        let hir_ty_kind = match ast_ty_kind {
            TyKind::Int(kind) => HirTyKind::Int(kind),
            TyKind::Bool => HirTyKind::Bool,
            TyKind::Str => HirTyKind::Str,
            TyKind::Adt(name) => match self.defs.get_by_names(&name.kind) {
                Some((def_id, data)) if data.kind == DefKind::Struct => HirTyKind::Adt(def_id),
                _ => {
                    self.emit_error(
                        HirLowerErrorKind::UndefinedName {
                            name: name.kind.clone(),
                        },
                        name.span.clone(),
                    );
                    HirTyKind::Err
                }
            },
            TyKind::Ref { mutable, inner } => {
                let inner_ty = self.lower_ty(inner.kind, span.clone());
                HirTyKind::Ref {
                    mutable,
                    inner: Box::new(inner_ty),
                }
            }
            TyKind::Array { elem, len } => {
                let elem_ty = self.lower_ty(elem.kind, span.clone());

                HirTyKind::Array {
                    elem: Box::new(elem_ty),
                    len,
                }
            }
            TyKind::Tuple(ty_list) => {
                let hir_ty_list = ty_list
                    .iter()
                    .map(|ty_node| self.lower_ty(ty_node.kind.clone(), span.clone()))
                    .collect();

                HirTyKind::Tuple(hir_ty_list)
            }
        };

        HirTy {
            span,
            kind: hir_ty_kind,
        }
    }

    fn lower_block(&mut self, block: &Block) -> HirBlock {
        self.enter_scope(ScopeKind::Block);

        let stmts = block
            .kind
            .stmts
            .iter()
            .map(|stmt| {
                let hir_stmt = self.lower_stmt(stmt);
                self.hir.alloc_stmt(hir_stmt)
            })
            .collect();

        let expr = block.kind.tail_expr.as_ref().map_or(None, |expr| {
            let hir_expr = self.lower_expr(expr.as_ref());
            Some(self.hir.alloc_expr(hir_expr))
        });

        self.exit_scope();
        HirBlock { stmts, expr }
    }

    fn lower_block_expr(&mut self, block: &Block) -> HirExpr {
        let hir_block = self.lower_block(block);
        let kind = HirExprKind::Block(hir_block);
        HirExpr {
            span: block.span.clone(),
            kind,
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> HirStmt {
        let kind = match &stmt.kind {
            StmtKind::Let { pat, ty, init } => {
                let hir_ty = ty
                    .as_ref()
                    .map(|ty| self.lower_ty(ty.kind.clone(), ty.span.clone()));
                let hir_expr = init.as_ref().map(|expr| self.lower_expr(expr));
                let init = hir_expr.map_or(None, |hir_expr| Some(self.hir.alloc_expr(hir_expr)));

                let hir_pat = self.lower_pat(pat);
                HirStmtKind::Let {
                    pat: hir_pat,
                    ty: hir_ty,
                    init,
                }
            }

            StmtKind::Assign { target, value } => {
                let lhs_expr = self.lower_place_as_expr(&target);
                let rhs_expr = self.lower_expr(&value);

                let lhs = self.hir.alloc_expr(lhs_expr);
                let rhs = self.hir.alloc_expr(rhs_expr);

                let assign_expr = HirExpr {
                    span: stmt.span.clone(),
                    kind: HirExprKind::Assign { lhs, rhs },
                };

                let assign_expr_id = self.hir.alloc_expr(assign_expr);

                HirStmtKind::Semi(assign_expr_id)
            }

            StmtKind::Expr(expr) => {
                let hir_expr = self.lower_expr(&expr);

                let hir_expr_id = self.hir.alloc_expr(hir_expr);

                HirStmtKind::Expr(hir_expr_id)
            }

            StmtKind::Semi(expr) => {
                let hir_expr = self.lower_expr(&expr);

                let hir_expr_id = self.hir.alloc_expr(hir_expr);

                HirStmtKind::Semi(hir_expr_id)
            }

            StmtKind::Return(return_expr) => {
                let value = return_expr.as_ref().map(|expr| {
                    let hir_expr = self.lower_expr(expr);
                    self.hir.alloc_expr(hir_expr)
                });

                let ret_expr = HirExpr {
                    span: stmt.span.clone(),
                    kind: HirExprKind::Return(value),
                };

                let ret_expr_id = self.hir.alloc_expr(ret_expr);

                HirStmtKind::Semi(ret_expr_id)
            }

            StmtKind::Break(break_expr) => {
                let value = break_expr.as_ref().map(|expr| {
                    let hir_expr = self.lower_expr(expr);
                    self.hir.alloc_expr(hir_expr)
                });

                let brk_expr = HirExpr {
                    span: stmt.span.clone(),
                    kind: HirExprKind::Break(value),
                };

                let brk_expr_id = self.hir.alloc_expr(brk_expr);

                HirStmtKind::Semi(brk_expr_id)
            }

            StmtKind::Continue => {
                let continue_expr = HirExpr {
                    span: stmt.span.clone(),
                    kind: HirExprKind::Continue,
                };

                let continue_expr_id = self.hir.alloc_expr(continue_expr);

                HirStmtKind::Semi(continue_expr_id)
            }

            StmtKind::While { cond, body } => {
                let cond_expr = self.lower_expr(&cond);
                let cond_expr_id = self.hir.alloc_expr(cond_expr);

                let body = self.lower_block(body);

                let while_expr = HirExpr {
                    span: stmt.span.clone(),
                    kind: HirExprKind::While {
                        cond: cond_expr_id,
                        body,
                    },
                };

                let while_expr_id = self.hir.alloc_expr(while_expr);

                HirStmtKind::Semi(while_expr_id)
            }

            StmtKind::For {
                mutable,
                var,
                ty,
                iter,
                body,
            } => {
                let for_expr =
                    self.lower_for_expr(*mutable, var, ty.as_ref(), iter, body, stmt.span.clone());
                let for_expr_id = self.hir.alloc_expr(for_expr);

                HirStmtKind::Semi(for_expr_id)
            }

            StmtKind::Loop { body } => {
                let body = self.lower_block(body);

                let loop_expr = HirExpr {
                    span: stmt.span.clone(),
                    kind: HirExprKind::Loop { body },
                };

                let loop_expr_id = self.hir.alloc_expr(loop_expr);

                HirStmtKind::Semi(loop_expr_id)
            }

            StmtKind::If {
                cond,
                then_block,
                else_branch,
            } => {
                let if_expr =
                    self.lower_if_expr(cond, then_block, else_branch.as_ref(), stmt.span.clone());
                let if_expr_id = self.hir.alloc_expr(if_expr);

                HirStmtKind::Semi(if_expr_id)
            }

            StmtKind::Empty => HirStmtKind::Empty,
        };

        HirStmt {
            span: stmt.span.clone(),
            kind,
        }
    }

    fn lower_expr(&mut self, expr: &Expr) -> HirExpr {
        let kind = match &expr.kind {
            ExprKind::Int(value) => HirExprKind::Int(*value),

            ExprKind::Bool(value) => HirExprKind::Bool(*value),

            ExprKind::String(value) => HirExprKind::String(value.clone()),

            ExprKind::Place(place) => return self.lower_place_as_expr(place),

            ExprKind::StructLit { name, fields } => {
                let def_id = match self.defs.get_by_names(&name.kind) {
                    Some((def_id, data)) if data.kind == DefKind::Struct => def_id,
                    _ => {
                        self.emit_error(
                            HirLowerErrorKind::UndefinedName {
                                name: name.kind.clone(),
                            },
                            name.span.clone(),
                        );
                        return HirExpr::err(expr.span.clone());
                    }
                };
                let fields = fields
                    .iter()
                    .map(|field| {
                        let hir_expr = self.lower_expr(&field.expr);
                        let expr = self.hir.alloc_expr(hir_expr);
                        HirStructLitField {
                            name: field.name.kind.clone(),
                            expr,
                            span: field.name.span.clone(),
                        }
                    })
                    .collect();

                HirExprKind::StructLit { def_id, fields }
            }

            ExprKind::Binary { op, lhs, rhs } => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                let lhs = self.hir.alloc_expr(lhs);
                let rhs = self.hir.alloc_expr(rhs);

                HirExprKind::Binary {
                    op: op.clone(),
                    lhs,
                    rhs,
                }
            }

            ExprKind::Call { callee, args } => {
                let callee = self.resolve_callee(&callee.kind, callee.span.clone());
                let args = args
                    .iter()
                    .map(|arg| {
                        let hir_arg = self.lower_expr(arg);
                        self.hir.alloc_expr(hir_arg)
                    })
                    .collect();

                HirExprKind::Call { callee, args }
            }

            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                let cond = self.lower_expr(cond);
                let cond = self.hir.alloc_expr(cond);
                let then_block = self.lower_block(then_block);
                let else_expr = self.lower_block_expr(else_block);
                let else_expr = self.hir.alloc_expr(else_expr);

                HirExprKind::If {
                    cond,
                    then_block,
                    else_expr: Some(else_expr),
                }
            }

            ExprKind::Loop { body } => {
                let body = self.lower_block(body);
                HirExprKind::Loop { body }
            }

            ExprKind::Block(block) => {
                let block = self.lower_block(block);
                HirExprKind::Block(block)
            }

            ExprKind::Array(elems) => {
                let elems = elems
                    .iter()
                    .map(|elem| {
                        let hir_elem = self.lower_expr(elem);
                        self.hir.alloc_expr(hir_elem)
                    })
                    .collect();

                HirExprKind::Array(elems)
            }

            ExprKind::Tuple(elems) => {
                let elems = elems
                    .iter()
                    .map(|elem| {
                        let hir_elem = self.lower_expr(elem);
                        self.hir.alloc_expr(hir_elem)
                    })
                    .collect();

                HirExprKind::Tuple(elems)
            }

            ExprKind::Index { base, index } => {
                let base = self.lower_expr(base);
                let index = self.lower_expr(index);
                let base = self.hir.alloc_expr(base);
                let index = self.hir.alloc_expr(index);

                HirExprKind::Index { base, index }
            }

            ExprKind::Range { start, end } => {
                let start = self.lower_expr(start);
                let end = self.lower_expr(end);
                let start = self.hir.alloc_expr(start);
                let end = self.hir.alloc_expr(end);

                HirExprKind::Range { start, end }
            }

            ExprKind::Borrow { mutable, expr } => {
                let expr = self.lower_expr(expr);
                let expr = self.hir.alloc_expr(expr);

                HirExprKind::Borrow {
                    mutable: *mutable,
                    expr,
                }
            }
        };

        HirExpr {
            span: expr.span.clone(),
            kind,
        }
    }

    fn lower_pat(&mut self, pat: &crate::ast::ty::Pat) -> HirPat {
        let kind = match &pat.kind {
            crate::ast::ty::PatKind::Wildcard => HirPatKind::Wildcard,
            crate::ast::ty::PatKind::Binding { mutable, name } => {
                match self.decalre_local(
                    name.kind.clone(),
                    *mutable,
                    LocalKind::Let,
                    name.span.clone(),
                ) {
                    Ok(local_id) => HirPatKind::Binding {
                        local_id,
                        name: name.kind.clone(),
                        mutable: *mutable,
                    },
                    Err(err) => {
                        self.emit_error(err, name.span.clone());
                        HirPatKind::Wildcard
                    }
                }
            }
            crate::ast::ty::PatKind::Tuple(elems) => {
                HirPatKind::Tuple(elems.iter().map(|elem| self.lower_pat(elem)).collect())
            }
            crate::ast::ty::PatKind::Struct { name, fields } => {
                let def_id = match self.defs.get_by_names(&name.kind) {
                    Some((def_id, data)) if data.kind == DefKind::Struct => def_id,
                    _ => {
                        self.emit_error(
                            HirLowerErrorKind::UndefinedName {
                                name: name.kind.clone(),
                            },
                            name.span.clone(),
                        );
                        return HirPat {
                            span: pat.span.clone(),
                            kind: HirPatKind::Wildcard,
                        };
                    }
                };
                let lowered_fields = fields
                    .iter()
                    .filter_map(|field| {
                        let index = match self.defs.get(def_id).and_then(|data| {
                            data.struct_fields
                                .iter()
                                .position(|decl_field| decl_field.name == field.name.kind)
                        }) {
                            Some(index) => index,
                            None => {
                                self.emit_error(
                                    HirLowerErrorKind::UndefinedName {
                                        name: field.name.kind.clone(),
                                    },
                                    field.name.span.clone(),
                                );
                                return None;
                            }
                        };
                        Some(HirStructPatField {
                            name: field.name.kind.clone(),
                            index,
                            pat: self.lower_pat(&field.pat),
                            span: field.name.span.clone(),
                        })
                    })
                    .collect();

                HirPatKind::Struct {
                    def_id,
                    fields: lowered_fields,
                }
            }
        };

        HirPat {
            span: pat.span.clone(),
            kind,
        }
    }

    fn lower_place_as_expr(&mut self, place: &Place) -> HirExpr {
        let kind = match &place.kind {
            PlaceKind::Local(name) => {
                let res = self.resolve_name(&name.kind, name.span.clone());
                HirExprKind::Path(res)
            }

            PlaceKind::Deref(expr) => {
                let expr = self.lower_expr(expr);
                let expr = self.hir.alloc_expr(expr);
                HirExprKind::Deref(expr)
            }

            PlaceKind::Index { base, index } => {
                let base = self.lower_place_as_expr(base);
                let index = self.lower_expr(index);
                let base = self.hir.alloc_expr(base);
                let index = self.hir.alloc_expr(index);

                HirExprKind::Index { base, index }
            }

            PlaceKind::Field { base, index } => {
                let base = self.lower_place_as_expr(base);
                let base = self.hir.alloc_expr(base);

                HirExprKind::Field {
                    base,
                    index: *index,
                }
            }

            PlaceKind::NamedField { base, name } => {
                let base = self.lower_place_as_expr(base);
                let base = self.hir.alloc_expr(base);

                HirExprKind::NamedField {
                    base,
                    name: name.kind.clone(),
                }
            }
        };

        HirExpr {
            span: place.span.clone(),
            kind,
        }
    }

    fn lower_if_expr(
        &mut self,
        cond: &Expr,
        then_block: &Block,
        else_branch: Option<&ElseBranch>,
        span: Span,
    ) -> HirExpr {
        let cond = self.lower_expr(cond);
        let cond = self.hir.alloc_expr(cond);
        let then_block = self.lower_block(then_block);
        let else_expr = else_branch.map(|branch| {
            let expr = self.lower_else_branch_as_expr(branch);
            self.hir.alloc_expr(expr)
        });

        HirExpr {
            span,
            kind: HirExprKind::If {
                cond,
                then_block,
                else_expr,
            },
        }
    }

    fn lower_else_branch_as_expr(&mut self, branch: &ElseBranch) -> HirExpr {
        match branch {
            ElseBranch::Block(block) => self.lower_block_expr(block),
            ElseBranch::If {
                cond,
                then_block,
                else_branch,
            } => self.lower_if_expr(cond, then_block, else_branch.as_deref(), cond.span.clone()),
        }
    }

    fn lower_for_expr(
        &mut self,
        mutable: bool,
        var: &crate::ast::ty::Ident,
        ty: Option<&crate::ast::ty::Ty>,
        iter: &Expr,
        body: &Block,
        span: Span,
    ) -> HirExpr {
        let hir_ty = ty.map(|ty| self.lower_ty(ty.kind.clone(), ty.span.clone()));

        self.enter_scope(ScopeKind::For);
        let local_id =
            match self.decalre_local(var.kind.clone(), mutable, LocalKind::For, var.span.clone()) {
                Ok(id) => id,
                Err(err) => {
                    self.emit_error(err, var.span.clone());
                    LocalId(usize::MAX)
                }
            };
        let body = self.lower_block(body);
        self.exit_scope();

        let kind = match &iter.kind {
            ExprKind::Range { start, end } => {
                let start = self.lower_expr(start);
                let end = self.lower_expr(end);
                let start = self.hir.alloc_expr(start);
                let end = self.hir.alloc_expr(end);
                HirExprKind::ForRange {
                    local_id,
                    name: var.kind.clone(),
                    mutable,
                    ty: hir_ty,
                    start,
                    end,
                    body,
                }
            }
            _ => {
                let iter = self.lower_expr(iter);
                let iter = self.hir.alloc_expr(iter);
                HirExprKind::ForIter {
                    local_id,
                    name: var.kind.clone(),
                    mutable,
                    ty: hir_ty,
                    iter,
                    body,
                }
            }
        };

        HirExpr { span, kind }
    }
}
