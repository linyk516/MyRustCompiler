#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IrFunctionId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IrBlockId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IrLocalId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IrTempId(pub usize);

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

impl_id_from_usize!(IrFunctionId, IrBlockId, IrLocalId, IrTempId);
