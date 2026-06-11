use crate::thir::{error::ThirLowerError, node::ThirProgram};

#[derive(Debug, Clone)]
pub struct ThirOutput {
    pub program: ThirProgram,
    pub errors: Vec<ThirLowerError>,
}
