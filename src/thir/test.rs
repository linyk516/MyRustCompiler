use crate::{
    compiler::{Compiler, source::SourceFile},
    thir::{
        node::{ThirBody, ThirExprKind, ThirPlaceKind},
        output::ThirOutput,
        pretty::ThirDump,
    },
    typecheck::result::TypeckOutput,
};

fn compile_thir(source: &str) -> (ThirOutput, TypeckOutput) {
    let compiler = Compiler::build(false).expect("compiler should build");
    let outcome = compiler.compile(SourceFile::new(source));

    assert!(
        !outcome.has_errors(),
        "expected source to compile without diagnostics, got {:?}",
        outcome.diagnostics
    );

    let output = outcome.output.expect("parse output should exist");
    (
        output.thir.expect("THIR output should exist"),
        output.typeck.expect("typecheck output should exist"),
    )
}

fn only_body(thir: &ThirOutput) -> &ThirBody {
    assert_eq!(thir.program.bodies.len(), 1);
    &thir.program.bodies[0]
}

#[test]
fn thir_lowering_creates_body_for_empty_function() {
    let (thir, _) = compile_thir("fn main() {}");
    let body = only_body(&thir);

    assert_eq!(body.params.len(), 0);
    assert!(body.expr(body.value).is_some());
}

#[test]
fn thir_lowering_maps_params_and_let_locals() {
    let (thir, _) = compile_thir(
        "
        fn main(x:i32) -> i32 {
            let mut y:i32 = x + 1;
            return y;
        }
        ",
    );
    let body = only_body(&thir);

    assert_eq!(body.params.len(), 1);
    assert!(body.locals.iter().any(|local| local.name == "x"));
    assert!(
        body.locals
            .iter()
            .any(|local| local.name == "y" && local.mutable)
    );
    assert!(
        body.exprs
            .iter()
            .any(|expr| matches!(expr.kind, ThirExprKind::Use(_)))
    );
}

#[test]
fn thir_lowering_turns_assignment_lhs_into_place() {
    let (thir, _) = compile_thir(
        "
        fn main() {
            let mut a:i32 = 1;
            a = a + 1;
        }
        ",
    );
    let body = only_body(&thir);

    assert!(body.exprs.iter().any(|expr| {
        matches!(
            &expr.kind,
            ThirExprKind::Assign {
                target,
                ..
            } if matches!(target.kind, ThirPlaceKind::Local(_))
        )
    }));
}

#[test]
fn thir_lowering_keeps_deref_assignment_as_place() {
    let (thir, _) = compile_thir(
        "
        fn inc_ref(p:&mut i32) -> i32 {
            *p = *p + 1;
            return *p;
        }
        ",
    );
    let body = only_body(&thir);

    assert!(body.exprs.iter().any(|expr| {
        matches!(
            &expr.kind,
            ThirExprKind::Assign {
                target,
                ..
            } if matches!(target.kind, ThirPlaceKind::Deref { .. })
        )
    }));
}

#[test]
fn thir_lowering_supports_index_and_field_places() {
    let (thir, _) = compile_thir(
        "
        fn main() -> i32 {
            let mut arr:[i32; 4] = [1, 2, 3, 4];
            arr[0] = arr.3;
            return arr[0];
        }
        ",
    );
    let body = only_body(&thir);

    assert!(body.exprs.iter().any(|expr| {
        matches!(
            &expr.kind,
            ThirExprKind::Assign {
                target,
                ..
            } if matches!(target.kind, ThirPlaceKind::Index { .. })
        )
    }));
    assert!(body.exprs.iter().any(|expr| {
        matches!(
            &expr.kind,
            ThirExprKind::Use(place) if matches!(place.kind, ThirPlaceKind::Field { .. })
        )
    }));
}

#[test]
fn thir_lowering_keeps_non_place_index_as_value_access() {
    let (thir, _) = compile_thir("fn main() -> i32 { [1, 2, 3][0] }");
    let body = only_body(&thir);

    assert!(
        body.exprs
            .iter()
            .any(|expr| matches!(expr.kind, ThirExprKind::IndexValue { .. }))
    );
}

#[test]
fn thir_lowering_preserves_control_expressions() {
    let (thir, _) = compile_thir(
        "
        fn main() -> i32 {
            let mut x:i32 = 0;
            while x < 3 {
                x = x + 1;
            }
            for i:i32 in 0..3 {
                x = x + i;
            }
            let y:i32 = if x { 1 } else { 2 };
            let z:i32 = loop {
                if y {
                    break y;
                } else {
                    break y + 1;
                }
                0
            };
            return z;
        }
        ",
    );
    let body = only_body(&thir);

    assert!(
        body.exprs
            .iter()
            .any(|expr| matches!(expr.kind, ThirExprKind::While { .. }))
    );
    assert!(
        body.exprs
            .iter()
            .any(|expr| matches!(expr.kind, ThirExprKind::ForRange { .. }))
    );
    assert!(
        body.exprs
            .iter()
            .any(|expr| matches!(expr.kind, ThirExprKind::If { .. }))
    );
    assert!(
        body.exprs
            .iter()
            .any(|expr| matches!(expr.kind, ThirExprKind::Loop { .. }))
    );
    assert!(
        body.exprs
            .iter()
            .any(|expr| matches!(expr.kind, ThirExprKind::Break(Some(_))))
    );
    assert!(
        body.exprs
            .iter()
            .any(|expr| matches!(expr.kind, ThirExprKind::Return(Some(_))))
    );
}

#[test]
fn thir_lowering_allows_borrowing_block_expression() {
    let (thir, _) = compile_thir(
        "
        fn main() -> i32 {
            let mut a:i32 = 1;
            let p:&mut i32 = &mut {
                a = a + 1;
                a
            };
            return *p;
        }
        ",
    );
    let body = only_body(&thir);

    assert!(
        body.exprs
            .iter()
            .any(|expr| { matches!(expr.kind, ThirExprKind::Borrow { mutable: true, .. }) })
    );
}

#[test]
fn thir_dump_prints_program_shape() {
    let (thir, typeck) = compile_thir("fn main() { let x:i32 = 1; }");
    let dump = ThirDump::new(&thir.program, &typeck.tys).dump();

    assert!(dump.contains("THIR Program"));
    assert!(dump.contains("Bodies"));
    assert!(dump.contains("Locals"));
    assert!(dump.contains("Exprs"));
    assert!(dump.contains("x: i32"));
}
