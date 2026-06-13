use std::fmt::{Display, Write};

use crate::ir::{
    id::{IrBlockId, IrFunctionId, IrValueId},
    node::{
        IrBinaryOp, IrFunction, IrIcmpPred, IrInstr, IrInstrKind, IrProgram, IrTerminator, IrTy,
        IrValueKind,
    },
};

pub struct IrDump<'a> {
    program: &'a IrProgram,
}

impl<'a> IrDump<'a> {
    pub fn new(program: &'a IrProgram) -> Self {
        Self { program }
    }

    pub fn dump(&self) -> String {
        let mut dumper = IrDumper::new(self.program);
        dumper.program();
        dumper.finish()
    }
}

impl<'a> Display for IrDump<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump())
    }
}

struct IrDumper<'a> {
    program: &'a IrProgram,
    out: String,
}

impl<'a> IrDumper<'a> {
    fn new(program: &'a IrProgram) -> Self {
        Self {
            program,
            out: String::new(),
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn line(&mut self, indent: usize, text: impl AsRef<str>) {
        for _ in 0..indent {
            self.out.push_str("  ");
        }
        let _ = writeln!(self.out, "{}", text.as_ref());
    }

    fn program(&mut self) {
        self.line(0, "; LLVM-like IR");

        for string in &self.program.global_strings {
            self.line(
                0,
                format!(
                    "@{} = private unnamed_addr constant [{} x i8] c\"{}\"",
                    string.name,
                    string.bytes.len(),
                    llvm_escape_bytes(&string.bytes)
                ),
            );
        }

        if !self.program.global_strings.is_empty() && !self.program.extern_functions.is_empty() {
            self.line(0, "");
        }

        for function in &self.program.extern_functions {
            let mut params = function
                .params
                .iter()
                .map(|ty| self.ty_text(ty))
                .collect::<Vec<_>>();
            if function.variadic {
                params.push("...".to_string());
            }
            self.line(
                0,
                format!(
                    "declare {} @{}({})",
                    self.ty_text(&function.ret_ty),
                    function.symbol_name,
                    params.join(", ")
                ),
            );
        }

        if !self.program.extern_functions.is_empty() && !self.program.functions.is_empty() {
            self.line(0, "");
        }

        if self.program.functions.is_empty() {
            if self.program.extern_functions.is_empty() && self.program.global_strings.is_empty() {
                self.line(0, "; <empty>");
            }
            return;
        }

        for index in 0..self.program.functions.len() {
            if index > 0 {
                self.line(0, "");
            }
            self.function(IrFunctionId(index));
        }
    }

    fn function(&mut self, id: IrFunctionId) {
        let Some(function) = self.program.function(id) else {
            self.line(0, format!("; missing function {:?}", id));
            return;
        };

        let params = function
            .params
            .iter()
            .map(|param| {
                format!(
                    "{} {}",
                    self.ty_text(&param.ty),
                    self.value_text(function, param.value)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.line(
            0,
            format!(
                "define {} @{}({}) {{",
                self.ty_text(&function.ret_ty),
                function.symbol_name,
                params
            ),
        );

        for index in 0..function.blocks.len() {
            self.block(function, IrBlockId(index));
        }

        self.line(0, "}");
    }

    fn block(&mut self, function: &IrFunction, id: IrBlockId) {
        let Some(block) = function.block(id) else {
            return;
        };

        self.line(0, format!("{}:", block.label));
        for instr in &block.instrs {
            self.instr(function, instr);
        }

        match &block.terminator {
            Some(terminator) => self.line(1, self.terminator_text(function, terminator)),
            None => self.line(1, "; <missing terminator>"),
        }
    }

    fn instr(&mut self, function: &IrFunction, instr: &IrInstr) {
        let result = instr
            .result
            .map(|value| format!("{} = ", self.value_text(function, value)))
            .unwrap_or_default();

        let text = match &instr.kind {
            IrInstrKind::Alloca { alloc_ty } => {
                format!("{}alloca {}", result, self.ty_text(alloc_ty))
            }
            IrInstrKind::Load { ty, ptr } => format!(
                "{}load {}, ptr {}",
                result,
                self.ty_text(ty),
                self.value_text(function, *ptr)
            ),
            IrInstrKind::Store { ty, value, ptr } => format!(
                "store {} {}, ptr {}",
                self.ty_text(ty),
                self.value_text(function, *value),
                self.value_text(function, *ptr)
            ),
            IrInstrKind::Gep {
                source_ty,
                base,
                indices,
            } => {
                let indices = indices
                    .iter()
                    .map(|index| {
                        let ty = function
                            .value(*index)
                            .map(|value| self.ty_text(&value.ty))
                            .unwrap_or_else(|| "i32".to_string());
                        format!("{} {}", ty, self.value_text(function, *index))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "{}getelementptr inbounds {}, ptr {}, {}",
                    result,
                    self.ty_text(source_ty),
                    self.value_text(function, *base),
                    indices
                )
            }
            IrInstrKind::Binary { op, ty, lhs, rhs } => format!(
                "{}{} {} {}, {}",
                result,
                self.binary_op_text(op),
                self.ty_text(ty),
                self.value_text(function, *lhs),
                self.value_text(function, *rhs)
            ),
            IrInstrKind::Icmp { pred, ty, lhs, rhs } => format!(
                "{}icmp {} {} {}, {}",
                result,
                self.icmp_pred_text(pred),
                self.ty_text(ty),
                self.value_text(function, *lhs),
                self.value_text(function, *rhs)
            ),
            IrInstrKind::Zext {
                from_ty,
                value,
                to_ty,
            } => format!(
                "{}zext {} {} to {}",
                result,
                self.ty_text(from_ty),
                self.value_text(function, *value),
                self.ty_text(to_ty)
            ),
            IrInstrKind::Call {
                callee,
                ret_ty,
                param_tys,
                variadic,
                args,
            } => {
                let args = args
                    .iter()
                    .map(|(ty, value)| {
                        format!("{} {}", self.ty_text(ty), self.value_text(function, *value))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                if *variadic {
                    let mut params = param_tys
                        .iter()
                        .map(|ty| self.ty_text(ty))
                        .collect::<Vec<_>>();
                    params.push("...".to_string());
                    format!(
                        "{}call {} ({}) @{}({})",
                        result,
                        self.ty_text(ret_ty),
                        params.join(", "),
                        self.callee_text(*callee),
                        args
                    )
                } else {
                    format!(
                        "{}call {} @{}({})",
                        result,
                        self.ty_text(ret_ty),
                        self.callee_text(*callee),
                        args
                    )
                }
            }
        };

        self.line(1, text);
    }

    fn terminator_text(&self, function: &IrFunction, terminator: &IrTerminator) -> String {
        match terminator {
            IrTerminator::Br { target } => {
                format!("br label %{}", self.block_label(function, *target))
            }
            IrTerminator::CondBr {
                cond,
                then_bb,
                else_bb,
            } => format!(
                "br i1 {}, label %{}, label %{}",
                self.value_text(function, *cond),
                self.block_label(function, *then_bb),
                self.block_label(function, *else_bb)
            ),
            IrTerminator::Ret { ty, value } => match value {
                Some(value) => format!(
                    "ret {} {}",
                    self.ty_text(ty),
                    self.value_text(function, *value)
                ),
                None => "ret void".to_string(),
            },
            IrTerminator::Unreachable => "unreachable".to_string(),
        }
    }

    fn value_text(&self, function: &IrFunction, id: IrValueId) -> String {
        let Some(value) = function.value(id) else {
            return format!("%missing{}", id.index());
        };

        match &value.kind {
            IrValueKind::ConstInt(value) => value.to_string(),
            IrValueKind::Unit => "void".to_string(),
            IrValueKind::GlobalStringAddr(id) => self
                .program
                .global_string(*id)
                .map(|string| format!("@{}", string.name))
                .unwrap_or_else(|| format!("@.missing{}", id.index())),
            IrValueKind::Param(_) | IrValueKind::SlotAddr(_) | IrValueKind::InstrResult => value
                .name
                .clone()
                .map(|name| format!("%{name}"))
                .unwrap_or_else(|| format!("%v{}", id.index())),
        }
    }

    fn callee_text(&self, callee: crate::hir::id::DefId) -> String {
        self.program
            .function_map
            .get(&callee)
            .and_then(|id| self.program.function(*id))
            .map(|function| function.symbol_name.clone())
            .or_else(|| {
                self.program
                    .extern_function_map
                    .get(&callee)
                    .and_then(|id| self.program.extern_function(*id))
                    .map(|function| function.symbol_name.clone())
            })
            .unwrap_or_else(|| format!("fn{}", callee.index()))
    }

    fn block_label(&self, function: &IrFunction, id: IrBlockId) -> String {
        function
            .block(id)
            .map(|block| block.label.clone())
            .unwrap_or_else(|| format!("missing{}", id.index()))
    }

    fn ty_text(&self, ty: &IrTy) -> String {
        match ty {
            IrTy::I1 => "i1".to_string(),
            IrTy::I8 => "i8".to_string(),
            IrTy::I16 => "i16".to_string(),
            IrTy::I32 => "i32".to_string(),
            IrTy::I64 => "i64".to_string(),
            IrTy::Void => "void".to_string(),
            IrTy::Ptr => "ptr".to_string(),
            IrTy::Array { elem, len } => format!("[{} x {}]", len, self.ty_text(elem)),
            IrTy::Struct(elems) => {
                let elems = elems
                    .iter()
                    .map(|elem| self.ty_text(elem))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {} }}", elems)
            }
            IrTy::Error => "<error>".to_string(),
        }
    }

    fn binary_op_text(&self, op: &IrBinaryOp) -> &'static str {
        match op {
            IrBinaryOp::Add => "add",
            IrBinaryOp::Sub => "sub",
            IrBinaryOp::Mul => "mul",
            IrBinaryOp::SDiv => "sdiv",
        }
    }

    fn icmp_pred_text(&self, pred: &IrIcmpPred) -> &'static str {
        match pred {
            IrIcmpPred::Eq => "eq",
            IrIcmpPred::Ne => "ne",
            IrIcmpPred::Slt => "slt",
            IrIcmpPred::Sle => "sle",
            IrIcmpPred::Sgt => "sgt",
            IrIcmpPred::Sge => "sge",
        }
    }
}

fn llvm_escape_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &byte in bytes {
        match byte {
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\t' => out.push_str("\\09"),
            0 => out.push_str("\\00"),
            0x20..=0x7e => out.push(byte as char),
            _ => {
                let _ = write!(out, "\\{byte:02X}");
            }
        }
    }
    out
}
