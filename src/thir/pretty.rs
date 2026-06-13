use std::fmt::{Display, Write};

use crate::{
    thir::{
        id::{ThirBodyId, ThirExprId, ThirLocalId, ThirStmtId},
        node::{
            ThirBlock, ThirExprKind, ThirPat, ThirPatKind, ThirPlace, ThirPlaceKind, ThirProgram,
            ThirStmtKind,
        },
    },
    typecheck::ty::{TyId, TyKind, TyStore},
};

pub struct ThirDump<'a> {
    program: &'a ThirProgram,
    tys: &'a TyStore,
}

impl<'a> ThirDump<'a> {
    pub fn new(program: &'a ThirProgram, tys: &'a TyStore) -> Self {
        Self { program, tys }
    }

    pub fn dump(&self) -> String {
        let mut dumper = ThirDumper::new(self.program, self.tys);
        dumper.program();
        dumper.finish()
    }
}

impl<'a> Display for ThirDump<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump())
    }
}

struct ThirDumper<'a> {
    program: &'a ThirProgram,
    tys: &'a TyStore,
    out: String,
}

impl<'a> ThirDumper<'a> {
    fn new(program: &'a ThirProgram, tys: &'a TyStore) -> Self {
        Self {
            program,
            tys,
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
        self.line(0, "THIR Program");
        if self.program.bodies.is_empty() {
            self.line(1, "<empty>");
            return;
        }

        self.line(1, "Bodies");
        for index in 0..self.program.bodies.len() {
            self.body(ThirBodyId(index), 2);
        }
    }

    fn body(&mut self, id: ThirBodyId, indent: usize) {
        let Some(body) = self.program.body(id) else {
            self.line(indent, format!("{id:?}: <missing>"));
            return;
        };

        self.line(indent, format!("{id:?} owner={:?}", body.owner));
        self.line(indent + 1, format!("Value {:?}", body.value));

        self.line(indent + 1, "Params");
        if body.params.is_empty() {
            self.line(indent + 2, "<empty>");
        } else {
            for param in &body.params {
                self.line(indent + 2, format!("{param:?}"));
            }
        }

        self.line(indent + 1, "Locals");
        if body.locals.is_empty() {
            self.line(indent + 2, "<empty>");
        } else {
            for index in 0..body.locals.len() {
                let id = ThirLocalId(index);
                let local = &body.locals[index];
                let mut name = String::new();
                if local.mutable {
                    name.push_str("mut ");
                }
                name.push_str(&local.name);
                self.line(
                    indent + 2,
                    format!("{id:?}: {name}: {}", self.ty_text(local.ty)),
                );
            }
        }

        self.line(indent + 1, "Stmts");
        if body.stmts.is_empty() {
            self.line(indent + 2, "<empty>");
        } else {
            for index in 0..body.stmts.len() {
                self.stmt(id, ThirStmtId(index), indent + 2);
            }
        }

        self.line(indent + 1, "Exprs");
        if body.exprs.is_empty() {
            self.line(indent + 2, "<empty>");
        } else {
            for index in 0..body.exprs.len() {
                self.expr(id, ThirExprId(index), indent + 2);
            }
        }
    }

    fn stmt(&mut self, body_id: ThirBodyId, id: ThirStmtId, indent: usize) {
        let Some(body) = self.program.body(body_id) else {
            self.line(indent, format!("{id:?}: <missing body>"));
            return;
        };
        let Some(stmt) = body.stmt(id) else {
            self.line(indent, format!("{id:?}: <missing>"));
            return;
        };

        self.line(indent, format!("{id:?}: {}", self.ty_text(stmt.ty)));
        match &stmt.kind {
            ThirStmtKind::Let { pat, init } => {
                self.line(indent + 1, "Let");
                self.pat(pat, indent + 2);
                if let Some(init) = init {
                    self.line(indent + 2, format!("Init {init:?}"));
                }
            }
            ThirStmtKind::Expr(expr) => self.line(indent + 1, format!("Expr {expr:?}")),
            ThirStmtKind::Semi(expr) => self.line(indent + 1, format!("Semi {expr:?}")),
            ThirStmtKind::Empty => self.line(indent + 1, "Empty"),
        }
    }

    fn expr(&mut self, body_id: ThirBodyId, id: ThirExprId, indent: usize) {
        let Some(body) = self.program.body(body_id) else {
            self.line(indent, format!("{id:?}: <missing body>"));
            return;
        };
        let Some(expr) = body.expr(id) else {
            self.line(indent, format!("{id:?}: <missing>"));
            return;
        };

        self.line(indent, format!("{id:?}: {}", self.ty_text(expr.ty)));
        self.expr_kind(&expr.kind, indent + 1);
    }

    fn pat(&mut self, pat: &ThirPat, indent: usize) {
        match &pat.kind {
            ThirPatKind::Wildcard => self.line(indent, "Wildcard"),
            ThirPatKind::Binding(local) => self.line(indent, format!("Binding {local:?}")),
            ThirPatKind::Tuple(elems) => {
                self.line(indent, "TuplePat");
                for elem in elems {
                    self.pat(elem, indent + 1);
                }
            }
            ThirPatKind::Struct { def_id, fields } => {
                self.line(indent, format!("StructPat {def_id:?}"));
                for (index, pat) in fields {
                    self.line(indent + 1, format!("Field {index}"));
                    self.pat(pat, indent + 2);
                }
            }
        }
    }

    fn expr_kind(&mut self, kind: &ThirExprKind, indent: usize) {
        match kind {
            ThirExprKind::Int(value) => self.line(indent, format!("Int {value}")),
            ThirExprKind::Bool(value) => self.line(indent, format!("Bool {value}")),
            ThirExprKind::String(value) => {
                self.line(indent, format!("String \"{}\"", escape_string(value)))
            }
            ThirExprKind::StructLit { def_id, fields } => {
                self.line(indent, format!("StructLit {def_id:?}"));
                for (index, expr) in fields {
                    self.line(indent + 1, format!("Field {index} {expr:?}"));
                }
            }
            ThirExprKind::Use(place) => {
                self.line(indent, "Use");
                self.place(place, indent + 1);
            }
            ThirExprKind::Binary { op, lhs, rhs } => {
                self.line(indent, format!("Binary {op:?}"));
                self.line(indent + 1, format!("Lhs {lhs:?}"));
                self.line(indent + 1, format!("Rhs {rhs:?}"));
            }
            ThirExprKind::Call { callee, args } => {
                self.line(indent, format!("Call {callee:?}"));
                for arg in args {
                    self.line(indent + 1, format!("Arg {arg:?}"));
                }
            }
            ThirExprKind::Assign { target, value } => {
                self.line(indent, "Assign");
                self.line(indent + 1, "Target");
                self.place(target, indent + 2);
                self.line(indent + 1, format!("Value {value:?}"));
            }
            ThirExprKind::Block(block) => {
                self.line(indent, "Block");
                self.block(block, indent + 1);
            }
            ThirExprKind::If {
                cond,
                then_expr,
                else_expr,
            } => {
                self.line(indent, "If");
                self.line(indent + 1, format!("Cond {cond:?}"));
                self.line(indent + 1, format!("Then {then_expr:?}"));
                match else_expr {
                    Some(else_expr) => self.line(indent + 1, format!("Else {else_expr:?}")),
                    None => self.line(indent + 1, "Else <none>"),
                }
            }
            ThirExprKind::While { cond, body } => {
                self.line(indent, format!("While cond={cond:?}"));
                self.block(body, indent + 1);
            }
            ThirExprKind::Loop { body } => {
                self.line(indent, "Loop");
                self.block(body, indent + 1);
            }
            ThirExprKind::ForRange {
                local,
                start,
                end,
                body,
            } => {
                self.line(indent, format!("ForRange {local:?}"));
                self.line(indent + 1, format!("Start {start:?}"));
                self.line(indent + 1, format!("End {end:?}"));
                self.block(body, indent + 1);
            }
            ThirExprKind::Return(value) => match value {
                Some(value) => self.line(indent, format!("Return {value:?}")),
                None => self.line(indent, "Return"),
            },
            ThirExprKind::Break(value) => match value {
                Some(value) => self.line(indent, format!("Break {value:?}")),
                None => self.line(indent, "Break"),
            },
            ThirExprKind::Continue => self.line(indent, "Continue"),
            ThirExprKind::Borrow { mutable, expr } => {
                let prefix = if *mutable { "&mut" } else { "&" };
                self.line(indent, format!("Borrow {prefix} {expr:?}"));
            }
            ThirExprKind::DerefValue(expr) => self.line(indent, format!("DerefValue {expr:?}")),
            ThirExprKind::IndexValue { base, index } => {
                self.line(indent, format!("IndexValue base={base:?} index={index:?}"));
            }
            ThirExprKind::FieldValue { base, index } => {
                self.line(indent, format!("FieldValue base={base:?} index={index}"));
            }
            ThirExprKind::Array(elems) => {
                self.line(indent, "Array");
                for elem in elems {
                    self.line(indent + 1, format!("Elem {elem:?}"));
                }
            }
            ThirExprKind::Tuple(elems) => {
                self.line(indent, "Tuple");
                for elem in elems {
                    self.line(indent + 1, format!("Elem {elem:?}"));
                }
            }
            ThirExprKind::Range { start, end } => {
                self.line(indent, format!("Range {start:?}..{end:?}"));
            }
        }
    }

    fn block(&mut self, block: &ThirBlock, indent: usize) {
        self.line(indent, "Stmts");
        if block.stmts.is_empty() {
            self.line(indent + 1, "<empty>");
        } else {
            for stmt in &block.stmts {
                self.line(indent + 1, format!("{stmt:?}"));
            }
        }

        match block.expr {
            Some(expr) => self.line(indent, format!("Tail {expr:?}")),
            None => self.line(indent, "Tail <none>"),
        }
    }

    fn place(&mut self, place: &ThirPlace, indent: usize) {
        self.line(indent, format!("Place {}", self.ty_text(place.ty)));
        match &place.kind {
            ThirPlaceKind::Local(local) => self.line(indent + 1, format!("Local {local:?}")),
            ThirPlaceKind::Deref { base } => self.line(indent + 1, format!("Deref {base:?}")),
            ThirPlaceKind::Index { base, index } => {
                self.line(indent + 1, "Index");
                self.place(base, indent + 2);
                self.line(indent + 2, format!("IndexExpr {index:?}"));
            }
            ThirPlaceKind::Field { base, index } => {
                self.line(indent + 1, format!("Field {index}"));
                self.place(base, indent + 2);
            }
        }
    }

    fn ty_text(&self, ty: TyId) -> String {
        match self.tys.kind(ty) {
            TyKind::Int(kind) => kind.name().to_string(),
            TyKind::Bool => "bool".to_string(),
            TyKind::Str => "str".to_string(),
            TyKind::Adt(def_id) => format!("adt {:?}", def_id),
            TyKind::Unit => "()".to_string(),
            TyKind::Never => "!".to_string(),
            TyKind::Tuple(elems) => {
                let elems = elems
                    .iter()
                    .map(|elem| self.ty_text(*elem))
                    .collect::<Vec<_>>();
                format!("({})", elems.join(", "))
            }
            TyKind::Array { elem, len } => format!("[{}; {len}]", self.ty_text(*elem)),
            TyKind::Ref { mutable, inner } => {
                if *mutable {
                    format!("&mut {}", self.ty_text(*inner))
                } else {
                    format!("&{}", self.ty_text(*inner))
                }
            }
            TyKind::Fn {
                params,
                ret,
                variadic,
            } => {
                let mut params = params
                    .iter()
                    .map(|param| self.ty_text(*param))
                    .collect::<Vec<_>>();
                if *variadic {
                    params.push("...".to_string());
                }
                format!("fn({}) -> {}", params.join(", "), self.ty_text(*ret))
            }
            TyKind::Infer(var) => format!("?T{var}"),
            TyKind::Error => "<error>".to_string(),
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
