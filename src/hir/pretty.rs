use std::fmt::{Display, Write};

use crate::hir::{
    id::{DefId, HirBodyId, HirExprId, HirItemId, HirStmtId, LocalId},
    node::{HirBlock, HirExprKind, HirItemKind, HirProgram, HirStmtKind},
    res::Res,
    table::{DefTable, LocalTable},
    ty::{HirTy, HirTyKind},
};

pub struct HirDump<'a> {
    pub hir: &'a HirProgram,
    pub defs: &'a DefTable,
    pub locals: &'a LocalTable,
}

impl<'a> HirDump<'a> {
    pub fn new(hir: &'a HirProgram, defs: &'a DefTable, locals: &'a LocalTable) -> Self {
        Self { hir, defs, locals }
    }

    pub fn dump(&self) -> String {
        let mut dumper = HirDumper::new(self.hir, self.defs, self.locals);
        dumper.program();
        dumper.finish()
    }

    pub fn dum_def(&self) -> String {
        let mut dumper = HirDumper::new(self.hir, self.defs, self.locals);
        dumper.def_table();
        dumper.finish()
    }

    pub fn dum_local(&self) -> String {
        let mut dumper = HirDumper::new(self.hir, self.defs, self.locals);
        dumper.local_table();
        dumper.finish()
    }
}

impl<'a> Display for HirDump<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump())?;
        Ok(())
    }
}

struct HirDumper<'a> {
    hir: &'a HirProgram,
    defs: &'a DefTable,
    locals: &'a LocalTable,
    out: String,
}

impl<'a> HirDumper<'a> {
    fn new(hir: &'a HirProgram, defs: &'a DefTable, locals: &'a LocalTable) -> Self {
        Self {
            hir,
            defs,
            locals,
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
        self.line(0, "HIR Program");
        for item in self.hir.root_items.clone() {
            self.item(item, 1);
        }
        if !self.hir.bodies.is_empty() {
            self.line(1, "Bodies");
            for index in 0..self.hir.bodies.len() {
                self.body(HirBodyId(index), 2);
            }
        }
    }

    fn def_table(&mut self) {
        self.line(0, "DefTable");

        if self.defs.defs.is_empty() {
            self.line(1, "<empty>");
        } else {
            for index in 0..self.defs.defs.len() {
                let id = DefId(index);
                let def = &self.defs.defs[index];
                self.line(
                    1,
                    format!(
                        "{:?} {} kind={:?} span={}",
                        id,
                        def.name,
                        def.kind,
                        self.span_text(&def.span)
                    ),
                );
            }
        }

        if !self.defs.names.is_empty() {
            self.line(1, "Names");
            let mut names = self
                .defs
                .names
                .iter()
                .map(|(name, id)| (name.clone(), *id))
                .collect::<Vec<_>>();
            names.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));

            for (name, id) in names {
                self.line(2, format!("{name} -> {:?}", id));
            }
        }
    }

    fn local_table(&mut self) {
        self.line(0, "LocalTable");

        if self.locals.locals.is_empty() {
            self.line(1, "<empty>");
            return;
        }

        for index in 0..self.locals.locals.len() {
            let id = LocalId(index);
            let local = &self.locals.locals[index];
            let mut name = String::new();
            if local.mutable {
                name.push_str("mut ");
            }
            name.push_str(&local.name);

            self.line(
                1,
                format!(
                    "{:?} {} kind={:?} owner={} span={}",
                    id,
                    name,
                    local.kind,
                    self.def_text(local.owner),
                    self.span_text(&local.span)
                ),
            );
        }
    }

    fn item(&mut self, id: HirItemId, indent: usize) {
        let Some(item) = self.hir.item(id).cloned() else {
            self.line(indent, format!("MissingItem {:?}", id));
            return;
        };

        match item.kind {
            HirItemKind::Fn(hir_fn) => {
                self.line(
                    indent,
                    format!("FnItem {:?} {}", item.def_id, self.def_text(item.def_id)),
                );
                self.line(indent + 1, format!("Name {}", hir_fn.name));
                if !hir_fn.sig.params.is_empty() {
                    self.line(indent + 1, "SigParams");
                    for param in &hir_fn.sig.params {
                        let mut text = String::from("Param ");
                        if param.mutable {
                            text.push_str("mut ");
                        }
                        let _ = write!(
                            text,
                            "{} {:?}: {}",
                            param.name,
                            param.local_id,
                            self.ty_text(&param.ty)
                        );
                        self.line(indent + 2, text);
                    }
                }
                self.line(
                    indent + 1,
                    format!("Return {}", self.ty_text(&hir_fn.sig.ret_ty)),
                );
                self.body(hir_fn.body, indent + 1);
            }
        }
    }

    fn body(&mut self, id: HirBodyId, indent: usize) {
        let Some(body) = self.hir.body(id).cloned() else {
            self.line(indent, format!("MissingBody {:?}", id));
            return;
        };

        self.line(indent, format!("Body owner={}", self.def_text(body.owner)));

        if !body.params.is_empty() {
            self.line(indent + 1, "Params");
            for param in body.params {
                self.line(indent + 2, self.local_text(param));
            }
        }

        self.line(indent + 1, "Value");
        self.expr(body.value, indent + 2);
    }

    fn block(&mut self, block: &HirBlock, indent: usize) {
        self.line(indent, "Block");

        for stmt in &block.stmts {
            self.stmt(*stmt, indent + 1);
        }

        if let Some(expr) = block.expr {
            self.line(indent + 1, "TailExpr");
            self.expr(expr, indent + 2);
        }
    }

    fn stmt(&mut self, id: HirStmtId, indent: usize) {
        let Some(stmt) = self.hir.stmt(id).cloned() else {
            self.line(indent, format!("MissingStmt {:?}", id));
            return;
        };

        self.stmt_kind(&stmt.kind, indent);
    }

    fn stmt_kind(&mut self, kind: &HirStmtKind, indent: usize) {
        match kind {
            HirStmtKind::Let {
                local_id,
                name,
                mutable,
                ty,
                init,
            } => {
                let mut text = String::from("Let ");
                if *mutable {
                    text.push_str("mut ");
                }
                let _ = write!(
                    text,
                    "{} {:?} {}",
                    name,
                    local_id,
                    self.local_text(*local_id)
                );
                if let Some(ty) = ty {
                    let _ = write!(text, ": {}", self.ty_text(ty));
                }
                self.line(indent, text);

                if let Some(init) = init {
                    self.line(indent + 1, "Init");
                    self.expr(*init, indent + 2);
                }
            }
            HirStmtKind::Expr(expr) => {
                self.line(indent, "ExprStmt");
                self.expr(*expr, indent + 1);
            }
            HirStmtKind::Semi(expr) => {
                self.line(indent, "Semi");
                self.expr(*expr, indent + 1);
            }
            HirStmtKind::Empty => self.line(indent, "Empty"),
        }
    }

    fn expr(&mut self, id: HirExprId, indent: usize) {
        let Some(expr) = self.hir.expr(id).cloned() else {
            self.line(indent, format!("MissingExpr {:?}", id));
            return;
        };

        self.expr_kind(&expr.kind, indent);
    }

    fn expr_kind(&mut self, kind: &HirExprKind, indent: usize) {
        match kind {
            HirExprKind::Int(value) => self.line(indent, format!("Int {value}")),
            HirExprKind::Path(res) => self.line(indent, format!("Path {}", self.res_text(*res))),
            HirExprKind::Binary { op, lhs, rhs } => {
                self.line(indent, format!("Binary {}", self.binary_op(op)));
                self.expr(*lhs, indent + 1);
                self.expr(*rhs, indent + 1);
            }
            HirExprKind::Call { callee, args } => {
                self.line(indent, format!("Call {}", self.res_text(*callee)));
                for arg in args {
                    self.expr(*arg, indent + 1);
                }
            }
            HirExprKind::Assign { lhs, rhs } => {
                self.line(indent, "Assign");
                self.line(indent + 1, "Lhs");
                self.expr(*lhs, indent + 2);
                self.line(indent + 1, "Rhs");
                self.expr(*rhs, indent + 2);
            }
            HirExprKind::Block(block) => self.block(block, indent),
            HirExprKind::If {
                cond,
                then_block,
                else_expr,
            } => {
                self.line(indent, "If");
                self.line(indent + 1, "Cond");
                self.expr(*cond, indent + 2);
                self.line(indent + 1, "Then");
                self.block(then_block, indent + 2);
                if let Some(else_expr) = else_expr {
                    self.line(indent + 1, "Else");
                    self.expr(*else_expr, indent + 2);
                }
            }
            HirExprKind::While { cond, body } => {
                self.line(indent, "While");
                self.line(indent + 1, "Cond");
                self.expr(*cond, indent + 2);
                self.line(indent + 1, "Body");
                self.block(body, indent + 2);
            }
            HirExprKind::Loop { body } => {
                self.line(indent, "Loop");
                self.block(body, indent + 1);
            }
            HirExprKind::ForRange {
                local_id,
                name,
                mutable,
                ty,
                start,
                end,
                body,
            } => {
                let mut text = String::from("ForRange ");
                if *mutable {
                    text.push_str("mut ");
                }
                let _ = write!(
                    text,
                    "{} {:?} {}",
                    name,
                    local_id,
                    self.local_text(*local_id)
                );
                if let Some(ty) = ty {
                    let _ = write!(text, ": {}", self.ty_text(ty));
                }
                self.line(indent, text);
                self.line(indent + 1, "Start");
                self.expr(*start, indent + 2);
                self.line(indent + 1, "End");
                self.expr(*end, indent + 2);
                self.line(indent + 1, "Body");
                self.block(body, indent + 2);
            }
            HirExprKind::Return(expr) => {
                self.line(indent, "Return");
                if let Some(expr) = expr {
                    self.expr(*expr, indent + 1);
                }
            }
            HirExprKind::Break(expr) => {
                self.line(indent, "Break");
                if let Some(expr) = expr {
                    self.expr(*expr, indent + 1);
                }
            }
            HirExprKind::Continue => self.line(indent, "Continue"),
            HirExprKind::Borrow { mutable, expr } => {
                if *mutable {
                    self.line(indent, "BorrowMut");
                } else {
                    self.line(indent, "Borrow");
                }
                self.expr(*expr, indent + 1);
            }
            HirExprKind::Deref(expr) => {
                self.line(indent, "Deref");
                self.expr(*expr, indent + 1);
            }
            HirExprKind::Index { base, index } => {
                self.line(indent, "Index");
                self.line(indent + 1, "Base");
                self.expr(*base, indent + 2);
                self.line(indent + 1, "Index");
                self.expr(*index, indent + 2);
            }
            HirExprKind::Field { base, index } => {
                self.line(indent, format!("Field .{index}"));
                self.expr(*base, indent + 1);
            }
            HirExprKind::Array(elems) => {
                self.line(indent, "Array");
                for elem in elems {
                    self.expr(*elem, indent + 1);
                }
            }
            HirExprKind::Tuple(elems) => {
                self.line(indent, "Tuple");
                for elem in elems {
                    self.expr(*elem, indent + 1);
                }
            }
            HirExprKind::Range { start, end } => {
                self.line(indent, "Range");
                self.line(indent + 1, "Start");
                self.expr(*start, indent + 2);
                self.line(indent + 1, "End");
                self.expr(*end, indent + 2);
            }
            HirExprKind::Err => self.line(indent, "Err"),
        }
    }

    fn res_text(&self, res: Res) -> String {
        match res {
            Res::Def(id) => self.def_text(id),
            Res::Local(id) => self.local_text(id),
            Res::Err => "Err".to_string(),
        }
    }

    fn local_text(&self, id: LocalId) -> String {
        match self.locals.get(id) {
            Some(local) => {
                let mut text = String::new();
                if local.mutable {
                    text.push_str("mut ");
                }
                let _ = write!(text, "{}({:?})", local.name, id);
                text
            }
            None => format!("<missing local {:?}>", id),
        }
    }

    fn def_text(&self, id: DefId) -> String {
        match self.defs.get(id) {
            Some(def) => format!("{}({:?})", def.name, id),
            None => format!("<missing def {:?}>", id),
        }
    }

    fn ty_text(&self, ty: &HirTy) -> String {
        match &ty.kind {
            HirTyKind::I32 => "i32".to_string(),
            HirTyKind::Unit => "()".to_string(),
            HirTyKind::Ref { mutable, inner } => {
                if *mutable {
                    format!("&mut {}", self.ty_text(inner))
                } else {
                    format!("&{}", self.ty_text(inner))
                }
            }
            HirTyKind::Array { elem, len } => format!("[{}; {len}]", self.ty_text(elem)),
            HirTyKind::Tuple(elems) => {
                let mut text = String::from("(");
                for (index, elem) in elems.iter().enumerate() {
                    if index > 0 {
                        text.push_str(", ");
                    }
                    text.push_str(&self.ty_text(elem));
                }
                if elems.len() == 1 {
                    text.push(',');
                }
                text.push(')');
                text
            }
            HirTyKind::Err => "<err>".to_string(),
        }
    }

    fn binary_op(&self, op: &crate::ast::ty::BinaryOp) -> &'static str {
        match op {
            crate::ast::ty::BinaryOp::Add => "Add",
            crate::ast::ty::BinaryOp::Sub => "Sub",
            crate::ast::ty::BinaryOp::Mul => "Mul",
            crate::ast::ty::BinaryOp::Div => "Div",
            crate::ast::ty::BinaryOp::Eq => "Eq",
            crate::ast::ty::BinaryOp::Ne => "Ne",
            crate::ast::ty::BinaryOp::Lt => "Lt",
            crate::ast::ty::BinaryOp::Le => "Le",
            crate::ast::ty::BinaryOp::Gt => "Gt",
            crate::ast::ty::BinaryOp::Ge => "Ge",
        }
    }

    fn span_text(&self, span: &crate::lexer::token::Span) -> String {
        format!("[{}..{})", span.start, span.end)
    }
}
