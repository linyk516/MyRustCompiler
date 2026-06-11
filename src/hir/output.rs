use crate::hir::{
    node::HirProgram,
    table::{DefTable, LocalTable},
};

#[derive(Debug, Clone)]
pub struct HirOutput {
    pub hir: HirProgram,
    pub defs: DefTable,
    pub locals: LocalTable,
}
