use std::fmt::{Display, Write};

use crate::ir::{
    id::{IrBlockId, IrFunctionId, IrLocalId, IrTempId},
    node::{IrOperand, IrPlace, IrProgram, QuadOp, Terminator},
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
        self.line(0, "IR Program");
        if self.program.functions.is_empty() {
            self.line(1, "<empty>");
            return;
        }

        for index in 0..self.program.functions.len() {
            self.function(IrFunctionId(index), 1);
        }
    }

    fn function(&mut self, id: IrFunctionId, indent: usize) {
        let Some(function) = self.program.function(id) else {
            self.line(indent, format!("{id:?}: <missing>"));
            return;
        };

        self.line(indent, format!("Function {:?}", function.owner));
        self.line(indent + 1, format!("entry bb{}", function.entry.index()));

        if !function.locals.is_empty() {
            self.line(indent + 1, "Locals");
            for index in 0..function.locals.len() {
                let id = IrLocalId(index);
                let local = &function.locals[index];
                let prefix = if local.mutable { "mut " } else { "" };
                self.line(
                    indent + 2,
                    format!("local{}: {}{}", id.index(), prefix, local.name),
                );
            }
        }

        if !function.temps.is_empty() {
            self.line(indent + 1, "Temps");
            for index in 0..function.temps.len() {
                self.line(indent + 2, format!("t{}", IrTempId(index).index()));
            }
        }

        self.line(indent + 1, "Quads");
        for index in 0..function.blocks.len() {
            let id = IrBlockId(index);
            let block = &function.blocks[index];
            self.line(indent + 2, format!("bb{}:", id.index()));
            if block.quads.is_empty() {
                self.line(indent + 3, "<empty>");
            } else {
                for quad in &block.quads {
                    self.line(
                        indent + 3,
                        format!(
                            "({}, {}, {}, {})",
                            self.op_text(&quad.op),
                            self.arg_text(&quad.arg1),
                            self.arg_text(&quad.arg2),
                            self.place_text(&quad.result)
                        ),
                    );
                }
            }
            self.line(indent + 3, self.terminator_text(&block.terminator));
        }
    }

    fn op_text(&self, op: &QuadOp) -> String {
        match op {
            QuadOp::Alloca => "alloca".to_string(),
            QuadOp::Add => "add".to_string(),
            QuadOp::Sub => "sub".to_string(),
            QuadOp::Mul => "mul".to_string(),
            QuadOp::Div => "sdiv".to_string(),
            QuadOp::Eq => "icmp_eq".to_string(),
            QuadOp::Ne => "icmp_ne".to_string(),
            QuadOp::Lt => "icmp_slt".to_string(),
            QuadOp::Le => "icmp_sle".to_string(),
            QuadOp::Gt => "icmp_sgt".to_string(),
            QuadOp::Ge => "icmp_sge".to_string(),
            QuadOp::Load => "load".to_string(),
            QuadOp::Store => "store".to_string(),
            QuadOp::Gep => "gep".to_string(),
            QuadOp::Arg => "arg".to_string(),
            QuadOp::Call(def) => format!("call {:?}", def),
        }
    }

    fn arg_text(&self, arg: &Option<IrOperand>) -> String {
        arg.as_ref()
            .map(|arg| self.operand_text(arg))
            .unwrap_or_else(|| "_".to_string())
    }

    fn operand_text(&self, operand: &IrOperand) -> String {
        match operand {
            IrOperand::ConstInt(value) => value.to_string(),
            IrOperand::Param(index) => format!("arg{index}"),
            IrOperand::Local(local) => format!("local{}", local.index()),
            IrOperand::Temp(temp) => format!("t{}", temp.index()),
        }
    }

    fn place_text(&self, place: &Option<IrPlace>) -> String {
        place
            .as_ref()
            .map(|place| self.ir_place_text(place))
            .unwrap_or_else(|| "_".to_string())
    }

    fn ir_place_text(&self, place: &IrPlace) -> String {
        match place {
            IrPlace::Local(local) => format!("local{}", local.index()),
            IrPlace::Temp(temp) => format!("t{}", temp.index()),
        }
    }

    fn terminator_text(&self, terminator: &Terminator) -> String {
        match terminator {
            Terminator::Goto(block) => format!("goto bb{}", block.index()),
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => format!(
                "if {} goto bb{} else bb{}",
                self.operand_text(cond),
                then_bb.index(),
                else_bb.index()
            ),
            Terminator::Return(value) => match value {
                Some(value) => format!("return {}", self.operand_text(value)),
                None => "return".to_string(),
            },
            Terminator::Unreachable => "unreachable".to_string(),
        }
    }
}
