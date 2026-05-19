pub(crate) mod automaton;
mod cst;
pub(crate) mod engine;
pub(crate) mod error;
pub(crate) mod first;
mod fmt;
pub(crate) mod grammar;
pub(crate) mod item;
mod production;
pub(crate) mod state;
pub(crate) mod symbol;
pub(crate) mod table;

pub use cst::CST;
pub use fmt::{CSTDisplay, CstSpanDisplayMode};

pub struct ParseResult {
    pub cst: CST,
}

pub struct ParseOutcome {
    pub result: Option<ParseResult>,
    pub errors: Vec<error::ParseError>,
    pub recovered: bool,
}

impl ParseOutcome {
    pub fn success(result: ParseResult) -> Self {
        Self {
            result: Some(result),
            errors: Vec::new(),
            recovered: false,
        }
    }

    pub fn failure(errors: Vec<error::ParseError>) -> Self {
        Self {
            result: None,
            recovered: !errors.is_empty(),
            errors,
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}
