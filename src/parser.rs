pub(crate) mod symbol;
mod production;
pub(crate) mod grammar;
pub(crate) mod item;
pub(crate) mod first;
pub(crate) mod state;
pub(crate) mod automaton;
pub(crate) mod table;
pub(crate) mod engine;
pub(crate) mod error;
mod fmt;
mod cst;

pub use cst::CST;
pub use fmt::CSTDisplay;

pub struct ParseResult {
    pub cst: CST,
}
