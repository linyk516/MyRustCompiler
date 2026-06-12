use std::collections::HashMap;

pub type TyId = usize;
pub type TyVarId = usize;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyKind {
    Int,
    Str,
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
        self.intern(TyKind::Int)
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
