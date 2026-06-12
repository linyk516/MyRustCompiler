use std::fmt::{Display, Write};

use crate::ast::ty::{
    BinaryOp, Block, ElseBranch, Expr, ExprKind, ExternFnDecl, FnDecl, FnSig, Ident, Item,
    ItemKind, Param, Place, PlaceKind, Program, Stmt, StmtKind, Ty, TyKind,
};

impl Program {
    pub fn dump(&self) -> String {
        let mut dumper = AstDumper::new();
        dumper.program(self);
        dumper.finish()
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump())
    }
}

struct AstDumper {
    out: String,
}

impl AstDumper {
    fn new() -> Self {
        Self { out: String::new() }
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

    fn program(&mut self, program: &Program) {
        self.line(0, "Program");
        for item in &program.items {
            self.item(item, 1);
        }
    }

    fn item(&mut self, item: &Item, indent: usize) {
        match &item.kind {
            ItemKind::Fn(func) => self.fn_decl(func, indent),
            ItemKind::ExternFn(func) => self.extern_fn_decl(func, indent),
        }
    }

    fn fn_decl(&mut self, func: &FnDecl, indent: usize) {
        self.fn_sig("Fn", &func.sig, indent);
        self.block(&func.body, indent + 1);
    }

    fn extern_fn_decl(&mut self, func: &ExternFnDecl, indent: usize) {
        self.fn_sig("ExternFn", &func.sig, indent);
    }

    fn fn_sig(&mut self, label: &str, sig: &FnSig, indent: usize) {
        let mut header = format!("{label} {}", self.ident(&sig.name));
        if sig.variadic {
            header.push_str(" variadic");
        }
        if let Some(ret_ty) = &sig.ret_ty {
            let _ = write!(header, " -> {}", self.ty_inline(ret_ty));
        }
        self.line(indent, header);

        if !sig.params.is_empty() {
            self.line(indent + 1, "Params");
            for param in &sig.params {
                self.param(param, indent + 2);
            }
        }
    }

    fn param(&mut self, param: &Param, indent: usize) {
        let mut text = String::from("Param ");
        if param.mutable {
            text.push_str("mut ");
        }
        let _ = write!(
            text,
            "{}: {}",
            self.ident(&param.name),
            self.ty_inline(&param.ty)
        );
        self.line(indent, text);
    }

    fn block(&mut self, block: &Block, indent: usize) {
        self.line(indent, "Block");
        for stmt in &block.kind.stmts {
            self.stmt(stmt, indent + 1);
        }
        if let Some(expr) = &block.kind.tail_expr {
            self.line(indent + 1, "TailExpr");
            self.expr(expr, indent + 2);
        }
    }

    fn stmt(&mut self, stmt: &Stmt, indent: usize) {
        match &stmt.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                init,
            } => {
                let mut text = String::from("Let ");
                if *mutable {
                    text.push_str("mut ");
                }
                text.push_str(self.ident(name));
                if let Some(ty) = ty {
                    let _ = write!(text, ": {}", self.ty_inline(ty));
                }
                self.line(indent, text);
                if let Some(init) = init {
                    self.line(indent + 1, "Init");
                    self.expr(init, indent + 2);
                }
            }
            StmtKind::Assign { target, value } => {
                self.line(indent, "Assign");
                self.line(indent + 1, "Target");
                self.place(target, indent + 2);
                self.line(indent + 1, "Value");
                self.expr(value, indent + 2);
            }
            StmtKind::Expr(expr) => {
                self.line(indent, "ExprStmt");
                self.expr(expr, indent + 1);
            }
            StmtKind::Semi(expr) => {
                self.line(indent, "Semi");
                self.expr(expr, indent + 1);
            }
            StmtKind::Return(expr) => {
                self.line(indent, "Return");
                if let Some(expr) = expr {
                    self.expr(expr, indent + 1);
                }
            }
            StmtKind::Break(expr) => {
                self.line(indent, "Break");
                if let Some(expr) = expr {
                    self.expr(expr, indent + 1);
                }
            }
            StmtKind::Continue => self.line(indent, "Continue"),
            StmtKind::While { cond, body } => {
                self.line(indent, "While");
                self.line(indent + 1, "Cond");
                self.expr(cond, indent + 2);
                self.block(body, indent + 1);
            }
            StmtKind::For {
                mutable,
                var,
                ty,
                iter,
                body,
            } => {
                let mut text = String::from("For ");
                if *mutable {
                    text.push_str("mut ");
                }
                text.push_str(self.ident(var));
                if let Some(ty) = ty {
                    let _ = write!(text, ": {}", self.ty_inline(ty));
                }
                self.line(indent, text);
                self.line(indent + 1, "Iter");
                self.expr(iter, indent + 2);
                self.block(body, indent + 1);
            }
            StmtKind::Loop { body } => {
                self.line(indent, "Loop");
                self.block(body, indent + 1);
            }
            StmtKind::If {
                cond,
                then_block,
                else_branch,
            } => {
                self.line(indent, "If");
                self.line(indent + 1, "Cond");
                self.expr(cond, indent + 2);
                self.line(indent + 1, "Then");
                self.block(then_block, indent + 2);
                if let Some(else_branch) = else_branch {
                    self.line(indent + 1, "Else");
                    self.else_branch(else_branch, indent + 2);
                }
            }
            StmtKind::Empty => self.line(indent, "Empty"),
        }
    }

    fn else_branch(&mut self, branch: &ElseBranch, indent: usize) {
        match branch {
            ElseBranch::Block(block) => self.block(block, indent),
            ElseBranch::If {
                cond,
                then_block,
                else_branch,
            } => {
                self.line(indent, "ElseIf");
                self.line(indent + 1, "Cond");
                self.expr(cond, indent + 2);
                self.line(indent + 1, "Then");
                self.block(then_block, indent + 2);
                if let Some(else_branch) = else_branch {
                    self.line(indent + 1, "Else");
                    self.else_branch(else_branch, indent + 2);
                }
            }
        }
    }

    fn expr(&mut self, expr: &Expr, indent: usize) {
        match &expr.kind {
            ExprKind::Int(value) => self.line(indent, format!("Int {value}")),
            ExprKind::String(value) => {
                self.line(indent, format!("String \"{}\"", escape_string(value)))
            }
            ExprKind::Place(place) => {
                self.line(indent, "PlaceExpr");
                self.place(place, indent + 1);
            }
            ExprKind::Binary { op, lhs, rhs } => {
                self.line(indent, format!("Binary {}", self.binary_op(op)));
                self.expr(lhs, indent + 1);
                self.expr(rhs, indent + 1);
            }
            ExprKind::Call { callee, args } => {
                self.line(indent, format!("Call {}", self.ident(callee)));
                for arg in args {
                    self.expr(arg, indent + 1);
                }
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                self.line(indent, "IfExpr");
                self.line(indent + 1, "Cond");
                self.expr(cond, indent + 2);
                self.line(indent + 1, "Then");
                self.block(then_block, indent + 2);
                self.line(indent + 1, "Else");
                self.block(else_block, indent + 2);
            }
            ExprKind::Loop { body } => {
                self.line(indent, "LoopExpr");
                self.block(body, indent + 1);
            }
            ExprKind::Block(block) => {
                self.line(indent, "BlockExpr");
                self.block(block, indent + 1);
            }
            ExprKind::Array(elems) => {
                self.line(indent, "Array");
                for elem in elems {
                    self.expr(elem, indent + 1);
                }
            }
            ExprKind::Tuple(elems) => {
                self.line(indent, "Tuple");
                for elem in elems {
                    self.expr(elem, indent + 1);
                }
            }
            ExprKind::Index { base, index } => {
                self.line(indent, "IndexExpr");
                self.line(indent + 1, "Base");
                self.expr(base, indent + 2);
                self.line(indent + 1, "Index");
                self.expr(index, indent + 2);
            }
            ExprKind::Range { start, end } => {
                self.line(indent, "Range");
                self.line(indent + 1, "Start");
                self.expr(start, indent + 2);
                self.line(indent + 1, "End");
                self.expr(end, indent + 2);
            }
            ExprKind::Borrow { mutable, expr } => {
                if *mutable {
                    self.line(indent, "BorrowMut");
                } else {
                    self.line(indent, "Borrow");
                }
                self.expr(expr, indent + 1);
            }
        }
    }

    fn place(&mut self, place: &Place, indent: usize) {
        match &place.kind {
            PlaceKind::Local(name) => self.line(indent, format!("Local {}", self.ident(name))),
            PlaceKind::Deref(expr) => {
                self.line(indent, "Deref");
                self.expr(expr, indent + 1);
            }
            PlaceKind::Index { base, index } => {
                self.line(indent, "IndexPlace");
                self.line(indent + 1, "Base");
                self.place(base, indent + 2);
                self.line(indent + 1, "Index");
                self.expr(index, indent + 2);
            }
            PlaceKind::Field { base, index } => {
                self.line(indent, format!("Field .{index}"));
                self.place(base, indent + 1);
            }
        }
    }

    fn ident<'a>(&self, ident: &'a Ident) -> &'a str {
        ident.kind.as_str()
    }

    fn ty_inline(&self, ty: &Ty) -> String {
        match &ty.kind {
            TyKind::I32 => "i32".to_string(),
            TyKind::Str => "str".to_string(),
            TyKind::Ref { mutable, inner } => {
                if *mutable {
                    format!("&mut {}", self.ty_inline(inner))
                } else {
                    format!("&{}", self.ty_inline(inner))
                }
            }
            TyKind::Array { elem, len } => format!("[{}; {len}]", self.ty_inline(elem)),
            TyKind::Tuple(elems) => {
                if elems.is_empty() {
                    return "()".to_string();
                }

                let mut text = String::from("(");
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        text.push_str(", ");
                    }
                    text.push_str(&self.ty_inline(elem));
                }
                if elems.len() == 1 {
                    text.push(',');
                }
                text.push(')');
                text
            }
        }
    }

    fn binary_op(&self, op: &BinaryOp) -> &'static str {
        match op {
            BinaryOp::Add => "Add",
            BinaryOp::Sub => "Sub",
            BinaryOp::Mul => "Mul",
            BinaryOp::Div => "Div",
            BinaryOp::Eq => "Eq",
            BinaryOp::Ne => "Ne",
            BinaryOp::Lt => "Lt",
            BinaryOp::Le => "Le",
            BinaryOp::Gt => "Gt",
            BinaryOp::Ge => "Ge",
        }
    }
}

fn escape_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\0' => out.push_str("\\0"),
            _ => out.push(ch),
        }
    }
    out
}
