use crate::{
    ast::ty::BinaryOp,
    hir::{
        id::{DefId, HirExprId, HirStmtId, LocalId},
        node::{
            HirBlock, HirBody, HirExpr, HirExprKind, HirFn, HirFnSig, HirItem, HirItemKind,
            HirParam, HirProgram, HirStmt, HirStmtKind,
        },
        res::Res,
        table::{DefKind, DefTable, LocalKind, LocalTable},
        ty::{HirTy, HirTyKind},
    },
    lexer::token::Span,
    typecheck::{
        check::TypeckCtx,
        error::TypeErrorKind,
        infer::InferCtx,
        result::{TypeckOutput, TypeckResults},
        ty::{TyId, TyKind, TyStore, TyVarId},
    },
};

fn infer_var(tys: &TyStore, ty: TyId) -> TyVarId {
    match tys.kind(ty) {
        TyKind::Infer(var) => *var,
        kind => panic!("expected infer type, got {kind:?}"),
    }
}

fn span() -> Span {
    Span::default()
}

fn hir_i32() -> HirTy {
    HirTy::new(HirTyKind::I32, span())
}

fn hir_unit() -> HirTy {
    HirTy::unit(span())
}

fn empty_block_expr(hir: &mut HirProgram) -> crate::hir::id::HirExprId {
    hir.alloc_expr(HirExpr::new(
        HirExprKind::Block(HirBlock {
            stmts: vec![],
            expr: None,
        }),
        span(),
    ))
}

fn single_fn_hir(
    param_ty: HirTy,
    ret_ty: HirTy,
) -> (HirProgram, DefTable, LocalTable, DefId, LocalId) {
    let mut hir = HirProgram::new();
    let mut defs = DefTable::new();
    let mut locals = LocalTable::new();

    let def_id = defs.alloc("main".to_string(), DefKind::Fn, span());
    let local_id = locals.alloc("x".to_string(), false, LocalKind::Param, def_id, span());

    let body_value = empty_block_expr(&mut hir);
    let body = hir.alloc_body(HirBody {
        owner: def_id,
        params: vec![local_id],
        value: body_value,
    });

    let sig = HirFnSig {
        params: vec![HirParam {
            local_id,
            name: "x".to_string(),
            mutable: false,
            ty: param_ty,
            span: span(),
        }],
        ret_ty,
        variadic: false,
    };

    let item = hir.alloc_item(HirItem {
        def_id,
        span: span(),
        kind: HirItemKind::Fn(HirFn {
            name: "main".to_string(),
            sig,
            body,
        }),
    });
    hir.root_items.push(item);

    (hir, defs, locals, def_id, local_id)
}

fn expr(hir: &mut HirProgram, kind: HirExprKind) -> HirExprId {
    hir.alloc_expr(HirExpr::new(kind, span()))
}

fn int_expr(hir: &mut HirProgram, value: i32) -> HirExprId {
    expr(hir, HirExprKind::Int(value))
}

fn local_expr(hir: &mut HirProgram, local_id: LocalId) -> HirExprId {
    expr(hir, HirExprKind::Path(Res::Local(local_id)))
}

fn block_expr(hir: &mut HirProgram, stmts: Vec<HirStmtId>, tail: Option<HirExprId>) -> HirExprId {
    expr(hir, HirExprKind::Block(HirBlock { stmts, expr: tail }))
}

fn semi_stmt(hir: &mut HirProgram, value: HirExprId) -> HirStmtId {
    hir.alloc_stmt(HirStmt::new(HirStmtKind::Semi(value), span()))
}

fn let_stmt(
    hir: &mut HirProgram,
    local_id: LocalId,
    name: &str,
    mutable: bool,
    ty: Option<HirTy>,
    init: Option<HirExprId>,
) -> HirStmtId {
    hir.alloc_stmt(HirStmt::new(
        HirStmtKind::Let {
            local_id,
            name: name.to_string(),
            mutable,
            ty,
            init,
        },
        span(),
    ))
}

fn single_fn_hir_with_body<F>(
    params: Vec<(&str, bool, HirTy)>,
    ret_ty: HirTy,
    build_body: F,
) -> (HirProgram, DefTable, LocalTable, DefId, Vec<LocalId>)
where
    F: FnOnce(&mut HirProgram, DefId, &[LocalId], &mut LocalTable) -> HirExprId,
{
    let mut hir = HirProgram::new();
    let mut defs = DefTable::new();
    let mut locals = LocalTable::new();

    let def_id = defs.alloc("main".to_string(), DefKind::Fn, span());
    let mut param_ids = vec![];
    let mut hir_params = vec![];

    for (name, mutable, ty) in params {
        let local_id = locals.alloc(name.to_string(), mutable, LocalKind::Param, def_id, span());
        param_ids.push(local_id);
        hir_params.push(HirParam {
            local_id,
            name: name.to_string(),
            mutable,
            ty,
            span: span(),
        });
    }

    let body_value = build_body(&mut hir, def_id, &param_ids, &mut locals);
    let body = hir.alloc_body(HirBody {
        owner: def_id,
        params: param_ids.clone(),
        value: body_value,
    });
    let item = hir.alloc_item(HirItem {
        def_id,
        span: span(),
        kind: HirItemKind::Fn(HirFn {
            name: "main".to_string(),
            sig: HirFnSig {
                params: hir_params,
                ret_ty,
                variadic: false,
            },
            body,
        }),
    });
    hir.root_items.push(item);

    (hir, defs, locals, def_id, param_ids)
}

fn has_error(output: &TypeckOutput, check: impl Fn(&TypeErrorKind) -> bool) -> bool {
    output.errors.iter().any(|error| check(&error.kind))
}

#[test]
fn typeck_results_start_empty() {
    let results = TypeckResults::new();

    assert!(results.expr_tys.is_empty());
    assert!(results.stmt_tys.is_empty());
    assert!(results.local_tys.is_empty());
    assert!(results.def_tys.is_empty());
}

#[test]
fn typeck_results_store_and_fetch_node_types() {
    let mut results = TypeckResults::new();

    results.set_expr_ty(HirExprId(0), 1);
    results.set_stmt_ty(HirStmtId(1), 2);
    results.set_local_ty(LocalId(2), 3);
    results.set_def_ty(DefId(3), 4);

    assert_eq!(results.get_expr_ty(HirExprId(0)), Some(&1));
    assert_eq!(results.get_stmt_ty(HirStmtId(1)), Some(&2));
    assert_eq!(results.get_local_ty(LocalId(2)), Some(&3));
    assert_eq!(results.get_def_ty(DefId(3)), Some(&4));

    assert_eq!(results.get_expr_ty(HirExprId(99)), None);
    assert_eq!(results.get_stmt_ty(HirStmtId(99)), None);
    assert_eq!(results.get_local_ty(LocalId(99)), None);
    assert_eq!(results.get_def_ty(DefId(99)), None);
}

#[test]
fn typeck_ctx_collects_function_signature_and_param_type() {
    let (hir, defs, locals, def_id, local_id) = single_fn_hir(hir_i32(), hir_unit());

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty());
    let def_ty = *output.results.get_def_ty(def_id).unwrap();
    let local_ty = *output.results.get_local_ty(local_id).unwrap();

    assert_eq!(output.tys.kind(local_ty), &TyKind::Int);
    match output.tys.kind(def_ty) {
        TyKind::Fn { params, ret, .. } => {
            assert_eq!(params.len(), 1);
            assert_eq!(output.tys.kind(params[0]), &TyKind::Int);
            assert_eq!(output.tys.kind(*ret), &TyKind::Unit);
        }
        kind => panic!("expected function type, got {kind:?}"),
    }
}

#[test]
fn typeck_ctx_lowers_composite_hir_types_in_function_signature() {
    let param_ty = HirTy::new(
        HirTyKind::Ref {
            mutable: true,
            inner: Box::new(hir_i32()),
        },
        span(),
    );
    let ret_ty = HirTy::new(
        HirTyKind::Tuple(vec![
            hir_i32(),
            HirTy::new(
                HirTyKind::Array {
                    elem: Box::new(hir_i32()),
                    len: 3,
                },
                span(),
            ),
        ]),
        span(),
    );
    let (hir, defs, locals, def_id, param_ids) =
        single_fn_hir_with_body(vec![("x", false, param_ty)], ret_ty, |hir, _, _, _| {
            let first = int_expr(hir, 1);
            let a = int_expr(hir, 1);
            let b = int_expr(hir, 2);
            let c = int_expr(hir, 3);
            let array = expr(hir, HirExprKind::Array(vec![a, b, c]));
            let tuple = expr(hir, HirExprKind::Tuple(vec![first, array]));
            block_expr(hir, vec![], Some(tuple))
        });
    let local_id = param_ids[0];

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty());
    let def_ty = *output.results.get_def_ty(def_id).unwrap();
    let local_ty = *output.results.get_local_ty(local_id).unwrap();

    match output.tys.kind(local_ty) {
        TyKind::Ref { mutable, inner } => {
            assert!(*mutable);
            assert_eq!(output.tys.kind(*inner), &TyKind::Int);
        }
        kind => panic!("expected mutable ref type, got {kind:?}"),
    }

    match output.tys.kind(def_ty) {
        TyKind::Fn { ret, .. } => match output.tys.kind(*ret) {
            TyKind::Tuple(elems) => {
                assert_eq!(elems.len(), 2);
                assert_eq!(output.tys.kind(elems[0]), &TyKind::Int);
                match output.tys.kind(elems[1]) {
                    TyKind::Array { elem, len } => {
                        assert_eq!(*len, 3);
                        assert_eq!(output.tys.kind(*elem), &TyKind::Int);
                    }
                    kind => panic!("expected array type, got {kind:?}"),
                }
            }
            kind => panic!("expected tuple return type, got {kind:?}"),
        },
        kind => panic!("expected function type, got {kind:?}"),
    }
}

#[test]
fn typeck_ctx_checks_block_tail_expr_and_records_expr_type() {
    let mut binary = HirExprId(usize::MAX);
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(vec![], hir_i32(), |hir, _, _, _| {
        let lhs = int_expr(hir, 1);
        let rhs = int_expr(hir, 2);
        binary = expr(
            hir,
            HirExprKind::Binary {
                op: BinaryOp::Add,
                lhs,
                rhs,
            },
        );
        block_expr(hir, vec![], Some(binary))
    });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let ty = *output
        .results
        .get_expr_ty(binary)
        .expect("binary expression should have a type");
    assert_eq!(output.tys.kind(ty), &TyKind::Int);
}

#[test]
fn typeck_ctx_infers_let_initializer_type() {
    let mut local_id = LocalId(usize::MAX);
    let mut init = HirExprId(usize::MAX);
    let mut stmt = HirStmtId(usize::MAX);
    let (hir, defs, locals, _, _) =
        single_fn_hir_with_body(vec![], hir_unit(), |hir, def_id, _, locals| {
            local_id = locals.alloc("x".to_string(), false, LocalKind::Let, def_id, span());
            init = int_expr(hir, 1);
            stmt = let_stmt(hir, local_id, "x", false, None, Some(init));
            block_expr(hir, vec![stmt], None)
        });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let local_ty = *output
        .results
        .get_local_ty(local_id)
        .expect("local initialized with i32 should have inferred type");
    let init_ty = *output
        .results
        .get_expr_ty(init)
        .expect("initializer expression should have a type");
    let stmt_ty = *output
        .results
        .get_stmt_ty(stmt)
        .expect("let statement should have a type");
    assert_eq!(output.tys.kind(local_ty), &TyKind::Int);
    assert_eq!(output.tys.kind(init_ty), &TyKind::Int);
    assert_eq!(output.tys.kind(stmt_ty), &TyKind::Unit);
}

#[test]
fn typeck_ctx_reports_function_body_return_type_mismatch() {
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(vec![], hir_i32(), |hir, _, _, _| {
        block_expr(hir, vec![], None)
    });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(has_error(&output, |kind| matches!(
        kind,
        TypeErrorKind::ReturnTypeMismatch { .. }
    )));
}

#[test]
fn typeck_ctx_accepts_explicit_return_without_tail_expr() {
    let mut return_expr = HirExprId(usize::MAX);
    let mut return_stmt = HirStmtId(usize::MAX);
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(vec![], hir_i32(), |hir, _, _, _| {
        let value = int_expr(hir, 1);
        return_expr = expr(hir, HirExprKind::Return(Some(value)));
        return_stmt = semi_stmt(hir, return_expr);
        block_expr(hir, vec![return_stmt], None)
    });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let return_expr_ty = *output
        .results
        .get_expr_ty(return_expr)
        .expect("return expression should have never type");
    let return_stmt_ty = *output
        .results
        .get_stmt_ty(return_stmt)
        .expect("return statement should preserve never type");
    assert_eq!(output.tys.kind(return_expr_ty), &TyKind::Never);
    assert_eq!(output.tys.kind(return_stmt_ty), &TyKind::Never);
}

#[test]
fn typeck_ctx_checks_call_expression_and_records_return_type() {
    let mut call = HirExprId(usize::MAX);
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(
        vec![("x", false, hir_i32())],
        hir_i32(),
        |hir, def_id, _, _| {
            let arg = int_expr(hir, 1);
            call = expr(
                hir,
                HirExprKind::Call {
                    callee: Res::Def(def_id),
                    args: vec![arg],
                },
            );
            block_expr(hir, vec![], Some(call))
        },
    );

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let call_ty = *output
        .results
        .get_expr_ty(call)
        .expect("call expression should have callee return type");
    assert_eq!(output.tys.kind(call_ty), &TyKind::Int);
}

#[test]
fn typeck_ctx_reports_wrong_call_argument_count() {
    let (hir, defs, locals, _, _) =
        single_fn_hir_with_body(vec![], hir_i32(), |hir, def_id, _, _| {
            let arg = int_expr(hir, 1);
            let call = expr(
                hir,
                HirExprKind::Call {
                    callee: Res::Def(def_id),
                    args: vec![arg],
                },
            );
            block_expr(hir, vec![], Some(call))
        });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(has_error(&output, |kind| matches!(
        kind,
        TypeErrorKind::WrongArgCount {
            expected: 0,
            actual: 1
        }
    )));
}

#[test]
fn typeck_ctx_reports_if_branch_mismatch() {
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(vec![], hir_i32(), |hir, _, _, _| {
        let cond = int_expr(hir, 1);
        let then_tail = int_expr(hir, 1);
        let then_block = HirBlock {
            stmts: vec![],
            expr: Some(then_tail),
        };
        let else_expr = block_expr(hir, vec![], None);
        let if_expr = expr(
            hir,
            HirExprKind::If {
                cond,
                then_block,
                else_expr: Some(else_expr),
            },
        );
        block_expr(hir, vec![], Some(if_expr))
    });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(has_error(&output, |kind| matches!(
        kind,
        TypeErrorKind::IfBranchMismatch { .. }
    )));
}

#[test]
fn typeck_ctx_reports_missing_else_when_if_is_used_as_value() {
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(vec![], hir_i32(), |hir, _, _, _| {
        let cond = int_expr(hir, 1);
        let then_tail = int_expr(hir, 1);
        let then_block = HirBlock {
            stmts: vec![],
            expr: Some(then_tail),
        };
        let if_expr = expr(
            hir,
            HirExprKind::If {
                cond,
                then_block,
                else_expr: None,
            },
        );
        block_expr(hir, vec![], Some(if_expr))
    });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(has_error(&output, |kind| matches!(
        kind,
        TypeErrorKind::MissingElseForValueIf { .. }
    )));
}

#[test]
fn typeck_ctx_infers_loop_expr_type_from_break_values() {
    let mut loop_expr = HirExprId(usize::MAX);
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(vec![], hir_i32(), |hir, _, _, _| {
        let cond = int_expr(hir, 1);
        let then_value = int_expr(hir, 20);
        let then_break = expr(hir, HirExprKind::Break(Some(then_value)));
        let then_stmt = semi_stmt(hir, then_break);
        let else_value = int_expr(hir, 23);
        let else_break = expr(hir, HirExprKind::Break(Some(else_value)));
        let else_stmt = semi_stmt(hir, else_break);
        let else_expr = block_expr(hir, vec![else_stmt], None);
        let if_expr = expr(
            hir,
            HirExprKind::If {
                cond,
                then_block: HirBlock {
                    stmts: vec![then_stmt],
                    expr: None,
                },
                else_expr: Some(else_expr),
            },
        );
        let if_stmt = semi_stmt(hir, if_expr);
        let tail = int_expr(hir, 0);
        loop_expr = expr(
            hir,
            HirExprKind::Loop {
                body: HirBlock {
                    stmts: vec![if_stmt],
                    expr: Some(tail),
                },
            },
        );
        block_expr(hir, vec![], Some(loop_expr))
    });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let loop_ty = *output
        .results
        .get_expr_ty(loop_expr)
        .expect("loop expression should have break value type");
    assert_eq!(output.tys.kind(loop_ty), &TyKind::Int);
}

#[test]
fn typeck_ctx_checks_array_and_index_expression_types() {
    let mut array = HirExprId(usize::MAX);
    let mut index = HirExprId(usize::MAX);
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(vec![], hir_i32(), |hir, _, _, _| {
        let first = int_expr(hir, 1);
        let second = int_expr(hir, 2);
        array = expr(hir, HirExprKind::Array(vec![first, second]));
        let offset = int_expr(hir, 0);
        index = expr(
            hir,
            HirExprKind::Index {
                base: array,
                index: offset,
            },
        );
        block_expr(hir, vec![], Some(index))
    });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let array_ty = *output
        .results
        .get_expr_ty(array)
        .expect("array expression should have a type");
    let index_ty = *output
        .results
        .get_expr_ty(index)
        .expect("index expression should have element type");
    match output.tys.kind(array_ty) {
        TyKind::Array { elem, len } => {
            assert_eq!(*len, 2);
            assert_eq!(output.tys.kind(*elem), &TyKind::Int);
        }
        kind => panic!("expected array type, got {kind:?}"),
    }
    assert_eq!(output.tys.kind(index_ty), &TyKind::Int);
}

#[test]
fn typeck_ctx_checks_array_field_expression_types() {
    let mut first_field = HirExprId(usize::MAX);
    let mut last_field = HirExprId(usize::MAX);
    let (hir, defs, locals, _, _) =
        single_fn_hir_with_body(vec![], hir_unit(), |hir, def_id, _, locals| {
            let arr_local = locals.alloc("arr".to_string(), false, LocalKind::Let, def_id, span());
            let a = int_expr(hir, 1);
            let b = int_expr(hir, 2);
            let c = int_expr(hir, 3);
            let d = int_expr(hir, 4);
            let array = expr(hir, HirExprKind::Array(vec![a, b, c, d]));
            let arr_ty = HirTy::new(
                HirTyKind::Array {
                    elem: Box::new(hir_i32()),
                    len: 4,
                },
                span(),
            );
            let arr_stmt = let_stmt(hir, arr_local, "arr", false, Some(arr_ty), Some(array));
            let arr_ref_a = local_expr(hir, arr_local);
            let arr_ref_b = local_expr(hir, arr_local);
            first_field = expr(
                hir,
                HirExprKind::Field {
                    base: arr_ref_a,
                    index: 0,
                },
            );
            last_field = expr(
                hir,
                HirExprKind::Field {
                    base: arr_ref_b,
                    index: 3,
                },
            );
            let tuple = expr(hir, HirExprKind::Tuple(vec![first_field, last_field]));
            let tuple_stmt = semi_stmt(hir, tuple);
            block_expr(hir, vec![arr_stmt, tuple_stmt], None)
        });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let first_ty = *output
        .results
        .get_expr_ty(first_field)
        .expect("array .0 field should have element type");
    let last_ty = *output
        .results
        .get_expr_ty(last_field)
        .expect("array .3 field should have element type");
    assert_eq!(output.tys.kind(first_ty), &TyKind::Int);
    assert_eq!(output.tys.kind(last_ty), &TyKind::Int);
}

#[test]
fn typeck_ctx_reports_invalid_index_base() {
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(vec![], hir_unit(), |hir, _, _, _| {
        let base = int_expr(hir, 1);
        let offset = int_expr(hir, 0);
        let index = expr(
            hir,
            HirExprKind::Index {
                base,
                index: offset,
            },
        );
        let stmt = semi_stmt(hir, index);
        block_expr(hir, vec![stmt], None)
    });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(has_error(&output, |kind| matches!(
        kind,
        TypeErrorKind::InvalidIndex { .. }
    )));
}

#[test]
fn typeck_ctx_checks_borrow_expression_type() {
    let ret_ty = HirTy::new(
        HirTyKind::Ref {
            mutable: false,
            inner: Box::new(hir_i32()),
        },
        span(),
    );
    let mut borrow = HirExprId(usize::MAX);
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(
        vec![("x", false, hir_i32())],
        ret_ty,
        |hir, _, params, _| {
            let base = local_expr(hir, params[0]);
            borrow = expr(
                hir,
                HirExprKind::Borrow {
                    mutable: false,
                    expr: base,
                },
            );
            block_expr(hir, vec![], Some(borrow))
        },
    );

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let borrow_ty = *output
        .results
        .get_expr_ty(borrow)
        .expect("borrow expression should have ref type");
    match output.tys.kind(borrow_ty) {
        TyKind::Ref { mutable, inner } => {
            assert!(!*mutable);
            assert_eq!(output.tys.kind(*inner), &TyKind::Int);
        }
        kind => panic!("expected ref type, got {kind:?}"),
    }
}

#[test]
fn typeck_ctx_checks_deref_expression_type() {
    let param_ty = HirTy::new(
        HirTyKind::Ref {
            mutable: false,
            inner: Box::new(hir_i32()),
        },
        span(),
    );
    let mut deref = HirExprId(usize::MAX);
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(
        vec![("p", false, param_ty)],
        hir_i32(),
        |hir, _, params, _| {
            let base = local_expr(hir, params[0]);
            deref = expr(hir, HirExprKind::Deref(base));
            block_expr(hir, vec![], Some(deref))
        },
    );

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let deref_ty = *output
        .results
        .get_expr_ty(deref)
        .expect("deref expression should have inner type");
    assert_eq!(output.tys.kind(deref_ty), &TyKind::Int);
}

#[test]
fn typeck_ctx_reports_cannot_deref_non_ref() {
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(vec![], hir_unit(), |hir, _, _, _| {
        let base = int_expr(hir, 1);
        let deref = expr(hir, HirExprKind::Deref(base));
        let stmt = semi_stmt(hir, deref);
        block_expr(hir, vec![stmt], None)
    });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(has_error(&output, |kind| matches!(
        kind,
        TypeErrorKind::CannotDeref { .. }
    )));
}

#[test]
fn typeck_ctx_reports_assignment_to_immutable_local() {
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(
        vec![("x", false, hir_i32())],
        hir_unit(),
        |hir, _, params, _| {
            let lhs = local_expr(hir, params[0]);
            let rhs = int_expr(hir, 1);
            let assign = expr(hir, HirExprKind::Assign { lhs, rhs });
            let stmt = semi_stmt(hir, assign);
            block_expr(hir, vec![stmt], None)
        },
    );

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(has_error(&output, |kind| matches!(
        kind,
        TypeErrorKind::NotAssignable { .. }
    )));
}

#[test]
fn typeck_ctx_accepts_assignment_to_mutable_local() {
    let mut assign = HirExprId(usize::MAX);
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(
        vec![("x", true, hir_i32())],
        hir_unit(),
        |hir, _, params, _| {
            let lhs = local_expr(hir, params[0]);
            let rhs = int_expr(hir, 1);
            assign = expr(hir, HirExprKind::Assign { lhs, rhs });
            let stmt = semi_stmt(hir, assign);
            block_expr(hir, vec![stmt], None)
        },
    );

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let assign_ty = *output
        .results
        .get_expr_ty(assign)
        .expect("assignment expression should have unit type");
    assert_eq!(output.tys.kind(assign_ty), &TyKind::Unit);
}

#[test]
fn typeck_ctx_reports_break_and_continue_outside_loop() {
    let (hir, defs, locals, _, _) = single_fn_hir_with_body(vec![], hir_unit(), |hir, _, _, _| {
        let break_expr = expr(hir, HirExprKind::Break(None));
        let continue_expr = expr(hir, HirExprKind::Continue);
        let break_stmt = semi_stmt(hir, break_expr);
        let continue_stmt = semi_stmt(hir, continue_expr);
        block_expr(hir, vec![break_stmt, continue_stmt], None)
    });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(has_error(&output, |kind| matches!(
        kind,
        TypeErrorKind::BreakOutsideLoop
    )));
    assert!(has_error(&output, |kind| matches!(
        kind,
        TypeErrorKind::ContinueOutsideLoop
    )));
}

#[test]
fn typeck_ctx_checks_for_range_loop_variable_and_bounds() {
    let mut loop_local = LocalId(usize::MAX);
    let (hir, defs, locals, _, _) =
        single_fn_hir_with_body(vec![], hir_unit(), |hir, def_id, _, locals| {
            loop_local = locals.alloc("i".to_string(), false, LocalKind::For, def_id, span());
            let start = int_expr(hir, 0);
            let end = int_expr(hir, 10);
            let for_expr = expr(
                hir,
                HirExprKind::ForRange {
                    local_id: loop_local,
                    name: "i".to_string(),
                    mutable: false,
                    ty: None,
                    start,
                    end,
                    body: HirBlock {
                        stmts: vec![],
                        expr: None,
                    },
                },
            );
            let stmt = semi_stmt(hir, for_expr);
            block_expr(hir, vec![stmt], None)
        });

    let output = TypeckCtx::new(&hir, &defs, &locals).check_program();

    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let loop_local_ty = *output
        .results
        .get_local_ty(loop_local)
        .expect("for range loop variable should have inferred i32 type");
    assert_eq!(output.tys.kind(loop_local_ty), &TyKind::Int);
}

#[test]
fn primitive_types_are_interned() {
    let mut tys = TyStore::new();

    let int_a = tys.int();
    let int_b = tys.int();
    let unit = tys.unit();
    let never = tys.never();
    let error = tys.error();

    assert_eq!(int_a, int_b);
    assert_ne!(int_a, unit);
    assert_ne!(unit, never);
    assert_ne!(never, error);
}

#[test]
fn equivalent_ref_types_reuse_the_same_id() {
    let mut tys = TyStore::new();

    let int = tys.int();
    let ref_a = tys.intern(TyKind::Ref {
        mutable: true,
        inner: int,
    });
    let ref_b = tys.intern(TyKind::Ref {
        mutable: true,
        inner: int,
    });
    let shared_ref = tys.intern(TyKind::Ref {
        mutable: false,
        inner: int,
    });

    assert_eq!(ref_a, ref_b);
    assert_ne!(ref_a, shared_ref);
}

#[test]
fn composite_types_are_interned_by_structure() {
    let mut tys = TyStore::new();

    let int = tys.int();
    let unit = tys.unit();
    let ref_int = tys.intern(TyKind::Ref {
        mutable: false,
        inner: int,
    });

    let tuple_a = tys.intern(TyKind::Tuple(vec![int, ref_int]));
    let tuple_b = tys.intern(TyKind::Tuple(vec![int, ref_int]));
    let array_a = tys.intern(TyKind::Array {
        elem: tuple_a,
        len: 4,
    });
    let array_b = tys.intern(TyKind::Array {
        elem: tuple_b,
        len: 4,
    });
    let fn_a = tys.intern(TyKind::Fn {
        params: vec![array_a],
        ret: unit,
        variadic: false,
    });
    let fn_b = tys.intern(TyKind::Fn {
        params: vec![array_b],
        ret: unit,
        variadic: false,
    });

    assert_eq!(tuple_a, tuple_b);
    assert_eq!(array_a, array_b);
    assert_eq!(fn_a, fn_b);
}

#[test]
fn infer_types_are_distinguished_by_var_id() {
    let mut tys = TyStore::new();

    let infer_0_a = tys.intern(TyKind::Infer(0));
    let infer_0_b = tys.intern(TyKind::Infer(0));
    let infer_1 = tys.intern(TyKind::Infer(1));

    assert_eq!(infer_0_a, infer_0_b);
    assert_ne!(infer_0_a, infer_1);
}

#[test]
fn kind_returns_the_stored_type() {
    let mut tys = TyStore::new();

    let int = tys.int();
    let ref_int = tys.intern(TyKind::Ref {
        mutable: true,
        inner: int,
    });

    match tys.kind(ref_int) {
        TyKind::Ref { mutable, inner } => {
            assert!(*mutable);
            assert_eq!(*inner, int);
        }
        kind => panic!("expected ref type, got {kind:?}"),
    }
}

#[test]
fn new_ty_var_creates_distinct_infer_types() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let a = infer.new_ty_var(&mut tys);
    let b = infer.new_ty_var(&mut tys);

    assert_ne!(a, b);
    assert_eq!(infer_var(&tys, a), 0);
    assert_eq!(infer_var(&tys, b), 1);
}

#[test]
fn bind_var_resolves_to_concrete_type() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let var_ty = infer.new_ty_var(&mut tys);
    let var = infer_var(&tys, var_ty);
    let int = tys.int();

    infer.bind_var(&mut tys, var, int).unwrap();

    assert_eq!(infer.resolve_ty(&tys, var_ty), int);
    assert_eq!(infer.deep_resolve_ty(&mut tys, var_ty), int);
}

#[test]
fn unioned_vars_share_binding() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let a_ty = infer.new_ty_var(&mut tys);
    let b_ty = infer.new_ty_var(&mut tys);
    let a = infer_var(&tys, a_ty);
    let b = infer_var(&tys, b_ty);
    let int = tys.int();

    infer.union(&mut tys, a, b).unwrap();
    infer.bind_var(&mut tys, a, int).unwrap();

    assert_eq!(infer.resolve_ty(&tys, a_ty), int);
    assert_eq!(infer.resolve_ty(&tys, b_ty), int);
}

#[test]
fn union_with_two_compatible_bindings_keeps_resolved_type() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let a_ty = infer.new_ty_var(&mut tys);
    let b_ty = infer.new_ty_var(&mut tys);
    let a = infer_var(&tys, a_ty);
    let b = infer_var(&tys, b_ty);
    let int = tys.int();

    infer.bind_var(&mut tys, a, int).unwrap();
    infer.bind_var(&mut tys, b, int).unwrap();
    infer.union(&mut tys, a, b).unwrap();

    assert_eq!(infer.resolve_ty(&tys, a_ty), int);
    assert_eq!(infer.resolve_ty(&tys, b_ty), int);
}

#[test]
fn union_with_incompatible_bindings_reports_type_mismatch() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let a_ty = infer.new_ty_var(&mut tys);
    let b_ty = infer.new_ty_var(&mut tys);
    let a = infer_var(&tys, a_ty);
    let b = infer_var(&tys, b_ty);
    let int = tys.int();
    let unit = tys.unit();

    infer.bind_var(&mut tys, a, int).unwrap();
    infer.bind_var(&mut tys, b, unit).unwrap();

    let err = infer.union(&mut tys, a, b).unwrap_err();

    assert!(matches!(err.kind, TypeErrorKind::MismatchedTypes { .. }));
}

#[test]
fn unify_infer_with_concrete_type_binds_variable() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let var_ty = infer.new_ty_var(&mut tys);
    let int = tys.int();

    let unified = infer.unify(&mut tys, var_ty, int).unwrap();

    assert_eq!(unified, int);
    assert_eq!(infer.resolve_ty(&tys, var_ty), int);
}

#[test]
fn unify_two_vars_then_bind_one_resolves_both() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let a_ty = infer.new_ty_var(&mut tys);
    let b_ty = infer.new_ty_var(&mut tys);
    let int = tys.int();

    infer.unify(&mut tys, a_ty, b_ty).unwrap();
    infer.unify(&mut tys, b_ty, int).unwrap();

    assert_eq!(infer.resolve_ty(&tys, a_ty), int);
    assert_eq!(infer.resolve_ty(&tys, b_ty), int);
}

#[test]
fn deep_resolve_updates_composite_types() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let var_ty = infer.new_ty_var(&mut tys);
    let var = infer_var(&tys, var_ty);
    let int = tys.int();
    let ref_var = tys.intern(TyKind::Ref {
        mutable: false,
        inner: var_ty,
    });
    let tuple = tys.intern(TyKind::Tuple(vec![ref_var, var_ty]));

    infer.bind_var(&mut tys, var, int).unwrap();
    let resolved = infer.deep_resolve_ty(&mut tys, tuple);
    let ref_int = tys.intern(TyKind::Ref {
        mutable: false,
        inner: int,
    });
    let expected = tys.intern(TyKind::Tuple(vec![ref_int, int]));

    assert_eq!(resolved, expected);
}

#[test]
fn unify_tuple_types_unifies_elements() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let var_ty = infer.new_ty_var(&mut tys);
    let int = tys.int();
    let unit = tys.unit();
    let tuple_var = tys.intern(TyKind::Tuple(vec![var_ty, unit]));
    let tuple_int = tys.intern(TyKind::Tuple(vec![int, unit]));

    let unified = infer.unify(&mut tys, tuple_var, tuple_int).unwrap();

    assert_eq!(infer.deep_resolve_ty(&mut tys, unified), tuple_int);
    assert_eq!(infer.resolve_ty(&tys, var_ty), int);
}

#[test]
fn unify_fn_types_unifies_params_and_return() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let param_var = infer.new_ty_var(&mut tys);
    let ret_var = infer.new_ty_var(&mut tys);
    let int = tys.int();
    let unit = tys.unit();
    let fn_var = tys.intern(TyKind::Fn {
        params: vec![param_var],
        ret: ret_var,
        variadic: false,
    });
    let fn_concrete = tys.intern(TyKind::Fn {
        params: vec![int],
        ret: unit,
        variadic: false,
    });

    let unified = infer.unify(&mut tys, fn_var, fn_concrete).unwrap();

    assert_eq!(infer.deep_resolve_ty(&mut tys, unified), fn_concrete);
    assert_eq!(infer.resolve_ty(&tys, param_var), int);
    assert_eq!(infer.resolve_ty(&tys, ret_var), unit);
}

#[test]
fn unify_ref_types_unifies_inner_types() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let var_ty = infer.new_ty_var(&mut tys);
    let int = tys.int();
    let ref_var = tys.intern(TyKind::Ref {
        mutable: true,
        inner: var_ty,
    });
    let ref_int = tys.intern(TyKind::Ref {
        mutable: true,
        inner: int,
    });

    let unified = infer.unify(&mut tys, ref_var, ref_int).unwrap();

    assert_eq!(infer.deep_resolve_ty(&mut tys, unified), ref_int);
    assert_eq!(infer.resolve_ty(&tys, var_ty), int);
}

#[test]
fn unify_array_length_mismatch_reports_type_mismatch() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let int = tys.int();
    let a = tys.intern(TyKind::Array { elem: int, len: 2 });
    let b = tys.intern(TyKind::Array { elem: int, len: 3 });

    let err = infer.unify(&mut tys, a, b).unwrap_err();

    assert!(matches!(err.kind, TypeErrorKind::MismatchedTypes { .. }));
}

#[test]
fn unify_ref_mutability_mismatch_reports_type_mismatch() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let int = tys.int();
    let shared = tys.intern(TyKind::Ref {
        mutable: false,
        inner: int,
    });
    let mutable = tys.intern(TyKind::Ref {
        mutable: true,
        inner: int,
    });

    let err = infer.unify(&mut tys, shared, mutable).unwrap_err();

    assert!(matches!(err.kind, TypeErrorKind::MismatchedTypes { .. }));
}

#[test]
fn occurs_check_rejects_recursive_type() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let var_ty = infer.new_ty_var(&mut tys);
    let var = infer_var(&tys, var_ty);
    let ref_var = tys.intern(TyKind::Ref {
        mutable: false,
        inner: var_ty,
    });

    let err = infer.bind_var(&mut tys, var, ref_var).unwrap_err();

    assert!(matches!(err.kind, TypeErrorKind::OccursCheckFailed { .. }));
}

#[test]
fn never_unifies_to_the_other_type() {
    let mut tys = TyStore::new();
    let mut infer = InferCtx::new();

    let never = tys.never();
    let int = tys.int();

    assert_eq!(infer.unify(&mut tys, never, int).unwrap(), int);
    assert_eq!(infer.unify(&mut tys, int, never).unwrap(), int);
}
