use crate::ir::{error::IrLowerError, node::IrProgram};

#[derive(Debug, Clone)]
pub struct IrOutput {
    pub program: IrProgram,
    pub errors: Vec<IrLowerError>,
}
