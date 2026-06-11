#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirItemId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirBodyId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirExprId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirStmtId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirTyId(pub usize);

macro_rules! impl_id_from_usize {
    ($($id:ident),+ $(,)?) => {
        $(
            impl From<usize> for $id {
                fn from(value: usize) -> Self {
                    Self(value)
                }
            }

            impl From<$id> for usize {
                fn from(value: $id) -> Self {
                    value.0
                }
            }

            impl $id {
                pub fn index(self) -> usize {
                    self.0
                }
            }
        )+
    };
}

impl_id_from_usize!(DefId, LocalId, HirExprId, HirItemId, HirStmtId, HirBodyId,);
