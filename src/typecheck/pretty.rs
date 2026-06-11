use std::fmt::{Display, Write};

use crate::typecheck::{
    result::TypeckResults,
    ty::{TyId, TyKind, TyStore},
};

pub struct TypeckDump<'a> {
    pub results: &'a TypeckResults,
    pub tys: &'a TyStore,
}

impl<'a> TypeckDump<'a> {
    pub fn new(results: &'a TypeckResults, tys: &'a TyStore) -> Self {
        Self { results, tys }
    }

    pub fn dump(&self) -> String {
        let mut dumper = TypeckDumper::new(self.results, self.tys);
        dumper.results();
        dumper.finish()
    }
}

impl<'a> Display for TypeckDump<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump())
    }
}

struct TypeckDumper<'a> {
    results: &'a TypeckResults,
    tys: &'a TyStore,
    out: String,
}

impl<'a> TypeckDumper<'a> {
    fn new(results: &'a TypeckResults, tys: &'a TyStore) -> Self {
        Self {
            results,
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

    fn results(&mut self) {
        self.line(0, "TypeckResults");
        self.def_tys();
        self.local_tys();
        self.stmt_tys();
        self.expr_tys();
    }

    fn def_tys(&mut self) {
        self.line(1, "DefTys");
        if self.results.def_tys.is_empty() {
            self.line(2, "<empty>");
            return;
        }

        let mut entries = self
            .results
            .def_tys
            .iter()
            .map(|(&id, &ty)| (id, ty))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(id, _)| id.index());

        for (id, ty) in entries {
            self.line(2, format!("{:?}: {}", id, self.ty_text(ty)));
        }
    }

    fn local_tys(&mut self) {
        self.line(1, "LocalTys");
        if self.results.local_tys.is_empty() {
            self.line(2, "<empty>");
            return;
        }

        let mut entries = self
            .results
            .local_tys
            .iter()
            .map(|(&id, &ty)| (id, ty))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(id, _)| id.index());

        for (id, ty) in entries {
            self.line(2, format!("{:?}: {}", id, self.ty_text(ty)));
        }
    }

    fn stmt_tys(&mut self) {
        self.line(1, "StmtTys");
        if self.results.stmt_tys.is_empty() {
            self.line(2, "<empty>");
            return;
        }

        let mut entries = self
            .results
            .stmt_tys
            .iter()
            .map(|(&id, &ty)| (id, ty))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(id, _)| id.index());

        for (id, ty) in entries {
            self.line(2, format!("{:?}: {}", id, self.ty_text(ty)));
        }
    }

    fn expr_tys(&mut self) {
        self.line(1, "ExprTys");
        if self.results.expr_tys.is_empty() {
            self.line(2, "<empty>");
            return;
        }

        let mut entries = self
            .results
            .expr_tys
            .iter()
            .map(|(&id, &ty)| (id, ty))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(id, _)| id.index());

        for (id, ty) in entries {
            self.line(2, format!("{:?}: {}", id, self.ty_text(ty)));
        }
    }

    fn ty_text(&self, ty: TyId) -> String {
        match self.tys.kind(ty) {
            TyKind::Int => "i32".to_string(),
            TyKind::Unit => "()".to_string(),
            TyKind::Never => "!".to_string(),
            TyKind::Tuple(elems) => {
                let elems = elems
                    .iter()
                    .map(|&elem| self.ty_text(elem))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", elems)
            }
            TyKind::Array { elem, len } => format!("[{}; {}]", self.ty_text(*elem), len),
            TyKind::Ref { mutable, inner } => {
                if *mutable {
                    format!("&mut {}", self.ty_text(*inner))
                } else {
                    format!("&{}", self.ty_text(*inner))
                }
            }
            TyKind::Fn { params, ret } => {
                let params = params
                    .iter()
                    .map(|&param| self.ty_text(param))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fn({}) -> {}", params, self.ty_text(*ret))
            }
            TyKind::Infer(var) => format!("?T{}", var),
            TyKind::Error => "<error>".to_string(),
        }
    }
}
