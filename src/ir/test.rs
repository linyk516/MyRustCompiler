use crate::{
    compiler::{Compiler, source::SourceFile},
    ir::{
        node::{IrFunction, IrInstrKind, IrTerminator, IrTy},
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

fn has_instr(function: &IrFunction, check: impl Fn(&IrInstrKind) -> bool) -> bool {
    function
        .blocks
        .iter()
        .flat_map(|block| block.instrs.iter())
        .any(|instr| check(&instr.kind))
}

#[test]
fn ir_lowering_creates_entry_block_for_empty_function() {
    let ir = compile_ir("fn main() {}");
    let function = only_function(&ir);

    assert_eq!(function.entry.index(), 0);
    assert!(!function.blocks.is_empty());
    assert!(matches!(
        function.blocks[function.entry.index()].terminator,
        Some(IrTerminator::Ret { value: None, .. })
    ));
}

#[test]
fn ir_lowering_generates_alloca_and_store_for_let_initializer() {
    let ir = compile_ir("fn main() { let x:i32 = 1; }");
    let function = only_function(&ir);

    assert!(has_instr(function, |kind| matches!(
        kind,
        IrInstrKind::Alloca { .. }
    )));
    assert!(has_instr(function, |kind| matches!(
        kind,
        IrInstrKind::Store { .. }
    )));
}

#[test]
fn ir_lowering_generates_binary_value_and_store_for_assignment() {
    let ir = compile_ir(
        "
        fn main() {
            let mut a:i32 = 1;
            a = a + 1;
        }
        ",
    );
    let function = only_function(&ir);

    assert!(has_instr(function, |kind| matches!(
        kind,
        IrInstrKind::Binary { .. }
    )));
    assert!(has_instr(function, |kind| matches!(
        kind,
        IrInstrKind::Store { .. }
    )));
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

    assert!(has_instr(function, |kind| matches!(
        kind,
        IrInstrKind::Load { .. }
    )));
    assert!(has_instr(function, |kind| matches!(
        kind,
        IrInstrKind::Store { .. }
    )));
}

#[test]
fn ir_lowering_generates_call_instruction() {
    let ir = compile_ir(
        "
        fn id(x:i32) -> i32 { return x; }
        fn main() {
            let y:i32 = id(1);
        }
        ",
    );

    assert!(
        ir.program.functions.iter().any(|function| {
            has_instr(function, |kind| matches!(kind, IrInstrKind::Call { .. }))
        })
    );
}

#[test]
fn ir_dump_exports_main_as_llvm_entry_symbol() {
    let ir = compile_ir("fn main() -> i32 { return 0; }");
    let dump = IrDump::new(&ir.program).dump();

    assert!(dump.contains("define i32 @main()"));
    assert!(!dump.contains("define i32 @fn"));
}

#[test]
fn ir_dump_exports_library_functions_with_source_names() {
    let ir = compile_ir("fn add(a:i32, b:i32) -> i32 { return a + b; }");
    let dump = IrDump::new(&ir.program).dump();

    assert!(dump.contains("define i32 @add(i32 %arg0, i32 %arg1)"));
    assert!(!dump.contains("@main"));
}

#[test]
fn ir_dump_uses_source_names_for_calls() {
    let ir = compile_ir(
        "
        fn id(x:i32) -> i32 { return x; }
        fn main() -> i32 {
            return id(1);
        }
        ",
    );
    let dump = IrDump::new(&ir.program).dump();

    assert!(dump.contains("call i32 @id(i32 1)"));
}

#[test]
fn ir_dump_declares_extern_variadic_function_and_global_string() {
    let ir = compile_ir(
        r#"
        extern fn printf(fmt:str, ...) -> i32;

        fn main() -> i32 {
            printf("answer = %d\n", 42);
            return 0;
        }
        "#,
    );
    let dump = IrDump::new(&ir.program).dump();

    assert!(dump.contains("@.str.0 = private unnamed_addr constant"));
    assert!(dump.contains("c\"answer = %d\\0A\\00\""));
    assert!(dump.contains("declare i32 @printf(ptr, ...)"));
    assert!(dump.contains("call i32 (ptr, ...) @printf(ptr"));
    assert!(dump.contains(", i32 42)"));
    assert!(!dump.contains("define i32 @printf"));
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

    assert!(has_instr(function, |kind| matches!(
        kind,
        IrInstrKind::Alloca { .. }
    )));
    assert!(has_instr(function, |kind| matches!(
        kind,
        IrInstrKind::Gep { .. }
    )));
    assert!(has_instr(function, |kind| matches!(
        kind,
        IrInstrKind::Load { .. }
    )));
    assert!(has_instr(function, |kind| matches!(
        kind,
        IrInstrKind::Store { .. }
    )));
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
            let y:i32 = if x > 0 { 1 } else { 2 };
            let z:i32 = loop {
                if y > 0 {
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
            .any(|block| matches!(block.terminator, Some(IrTerminator::CondBr { .. })))
    );
    assert!(
        function
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Some(IrTerminator::Br { .. })))
    );
}

#[test]
fn ir_lowering_does_not_emit_void_return_for_if_branches_that_always_return() {
    let ir = compile_ir(
        "
        fn choose(x:i32) -> i32 {
            if x > 0 {
                return 1;
            } else {
                return 2;
            }
        }
        ",
    );
    let function = only_function(&ir);
    let dump = IrDump::new(&ir.program).dump();

    assert!(!dump.contains("ret i32 void"));
    assert!(function.blocks.iter().all(|block| {
        !matches!(
            block.terminator,
            Some(IrTerminator::Ret {
                ty: IrTy::I32,
                value: None
            })
        )
    }));
}

#[test]
fn ir_dump_prints_llvm_like_text() {
    let ir = compile_ir("fn main() { let mut x:i32 = 1; x = x + 1; }");
    let dump = IrDump::new(&ir.program).dump();

    assert!(dump.contains("; LLVM-like IR"));
    assert!(dump.contains("define void @main"));
    assert!(dump.contains("entry:"));
    assert!(dump.contains("alloca i32"));
    assert!(dump.contains("store i32 1"));
    assert!(dump.contains("load i32"));
    assert!(dump.contains("add i32"));
    assert!(dump.contains("ret void"));
    assert!(dump.contains("getelementptr") || !dump.contains("(gep,"));
    assert!(!dump.contains("(add,"));
    assert!(!dump.contains("(store,"));
    assert!(!dump.contains("(arg,"));
}

#[test]
fn ir_dump_uses_named_temporaries_instead_of_bare_numeric_values() {
    let ir = compile_ir(
        "
        fn main() -> i32 {
            let mut x:i32 = 1;
            x = x + 1;
            return x;
        }
        ",
    );
    let dump = IrDump::new(&ir.program).dump();

    assert!(!dump.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with('%')
            && line
                .chars()
                .nth(1)
                .map(|ch| ch.is_ascii_digit())
                .unwrap_or(false)
    }));
    assert!(dump.contains("%v"));
}
