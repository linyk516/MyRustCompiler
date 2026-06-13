use std::collections::HashMap;

use crate::hir::id::DefId;

pub type TyId = usize;
pub type TyVarId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntKind {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    Usize,
    Isize,
}

impl IntKind {
    /// Returns the source spelling for this integer kind.
    pub fn name(self) -> &'static str {
        match self {
            IntKind::I8 => "i8",
            IntKind::I16 => "i16",
            IntKind::I32 => "i32",
            IntKind::I64 => "i64",
            IntKind::U8 => "u8",
            IntKind::U16 => "u16",
            IntKind::U32 => "u32",
            IntKind::U64 => "u64",
            IntKind::Usize => "usize",
            IntKind::Isize => "isize",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyKind {
    Int(IntKind),
    Bool,
    Str,
    Adt(DefId),
    Unit,
    Never,

    Tuple(Vec<TyId>),
    Array {
        elem: TyId,
        len: usize,
    },

    Ref {
        mutable: bool,
        inner: TyId,
    },

    Fn {
        params: Vec<TyId>,
        ret: TyId,
        variadic: bool,
    },

    Infer(TyVarId),
    Error,
}

/// 这里维护类型id和类型的双向映射，其中类型可以是待推导类型
#[derive(Debug, Clone)]
pub struct TyStore {
    tys: Vec<TyKind>,
    map: HashMap<TyKind, TyId>,
}

impl TyStore {
    pub fn new() -> Self {
        Self {
            tys: vec![],
            map: HashMap::new(),
        }
    }

    pub fn intern(&mut self, kind: TyKind) -> TyId {
        match self.map.get(&kind) {
            Some(&id) => id,
            None => {
                let id = self.tys.len();
                self.map.insert(kind.clone(), id);
                self.tys.push(kind);
                id
            }
        }
    }

    pub fn kind(&self, ty: TyId) -> &TyKind {
        &self.tys[ty]
    }

    pub fn int(&mut self) -> TyId {
        self.int_kind(IntKind::I32)
    }

    /// Interns a concrete integer type such as `i8`, `u32`, or `usize`.
    pub fn int_kind(&mut self, kind: IntKind) -> TyId {
        self.intern(TyKind::Int(kind))
    }

    pub fn bool(&mut self) -> TyId {
        self.intern(TyKind::Bool)
    }

    pub fn unit(&mut self) -> TyId {
        self.intern(TyKind::Unit)
    }

    pub fn str(&mut self) -> TyId {
        self.intern(TyKind::Str)
    }

    pub fn never(&mut self) -> TyId {
        self.intern(TyKind::Never)
    }

    pub fn error(&mut self) -> TyId {
        self.intern(TyKind::Error)
    }
}
