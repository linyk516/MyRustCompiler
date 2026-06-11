use crate::{
    compiler::{Compiler, source::SourceFile},
    ir::{
        node::{IrFunction, QuadOp, Terminator},
        output::IrOutput,
        pretty::IrDump,
    },
};

fn compile_ir(source: &str) -> IrOutput {
    let compiler = Compiler::build(false).expect("compiler should build");
    let outcome = compiler.compile(SourceFile::new(source));

    assert!(
        !outcome.has_errors(),
        "expected source to compile without diagnostics, got {:?}",
        outcome.diagnostics
    );

    outcome
        .output
        .expect("parse output should exist")
        .ir
        .expect("IR output should exist")
}

fn only_function(ir: &IrOutput) -> &IrFunction {
    assert_eq!(ir.program.functions.len(), 1);
    &ir.program.functions[0]
}

fn has_quad(function: &IrFunction, check: impl Fn(&QuadOp) -> bool) -> bool {
    function
        .blocks
        .iter()
        .flat_map(|block| block.quads.iter())
        .any(|quad| check(&quad.op))
}

#[test]
fn ir_lowering_creates_entry_block_for_empty_function() {
    let ir = compile_ir("fn main() {}");
    let function = only_function(&ir);

    assert_eq!(function.entry.index(), 0);
    assert!(!function.blocks.is_empty());
    assert!(matches!(
        function.blocks[function.entry.index()].terminator,
        Terminator::Return(None)
    ));
}

#[test]
fn ir_lowering_generates_alloca_and_store_for_let_initializer() {
    let ir = compile_ir("fn main() { let x:i32 = 1; }");
    let function = only_function(&ir);

    assert!(has_quad(function, |op| matches!(op, QuadOp::Alloca)));
    assert!(has_quad(function, |op| matches!(op, QuadOp::Store)));
}

#[test]
fn ir_lowering_generates_binary_temp_and_store_for_assignment() {
    let ir = compile_ir(
        "
        fn main() {
            let mut a:i32 = 1;
            a = a + 1;
        }
        ",
    );
    let function = only_function(&ir);

    assert!(has_quad(function, |op| matches!(op, QuadOp::Add)));
    assert!(has_quad(function, |op| matches!(op, QuadOp::Store)));
}

#[test]
fn ir_lowering_generates_load_and_store_for_deref_assignment() {
    let ir = compile_ir(
        "
        fn inc_ref(p:&mut i32) -> i32 {
            *p = *p + 1;
            return *p;
        }
        ",
    );
    let function = only_function(&ir);

    assert!(has_quad(function, |op| matches!(op, QuadOp::Load)));
    assert!(has_quad(function, |op| matches!(op, QuadOp::Store)));
}

#[test]
fn ir_lowering_generates_call_quad() {
    let ir = compile_ir(
        "
        fn id(x:i32) -> i32 { return x; }
        fn main() {
            let y:i32 = id(1);
        }
        ",
    );

    assert!(
        ir.program
            .functions
            .iter()
            .any(|function| has_quad(function, |op| matches!(op, QuadOp::Call(_))))
    );
}

#[test]
fn ir_lowering_lowers_aggregate_field_and_index_to_gep_load_store() {
    let ir = compile_ir(
        "
        fn main() -> i32 {
            let arr:[i32; 3] = [1, 2, 3];
            let pair:(i32, i32) = (arr.0, arr[1]);
            return pair.0;
        }
        ",
    );
    let function = only_function(&ir);

    assert!(has_quad(function, |op| matches!(op, QuadOp::Alloca)));
    assert!(has_quad(function, |op| matches!(op, QuadOp::Gep)));
    assert!(has_quad(function, |op| matches!(op, QuadOp::Load)));
    assert!(has_quad(function, |op| matches!(op, QuadOp::Store)));
}

#[test]
fn ir_lowering_generates_blocks_for_control_flow() {
    let ir = compile_ir(
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
    let function = only_function(&ir);

    assert!(function.blocks.len() >= 8);
    assert!(
        function
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::If { .. }))
    );
    assert!(
        function
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::Goto(_)))
    );
}

#[test]
fn ir_dump_prints_standard_quadruple_rows() {
    let ir = compile_ir("fn main() { let mut x:i32 = 1; x = x + 1; }");
    let dump = IrDump::new(&ir.program).dump();

    assert!(dump.contains("IR Program"));
    assert!(dump.contains("Function DefId"));
    assert!(dump.contains("bb0:"));
    assert!(dump.contains("(alloca,"));
    assert!(dump.contains("(add,"));
    assert!(dump.contains("(store,"));
    assert!(!dump.contains("make_array"));
    assert!(!dump.contains("make_tuple"));
    assert!(!dump.contains("get_field"));
    assert!(!dump.contains("get_index"));
}
