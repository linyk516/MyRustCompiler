use crate::lexer::token::Token;
use crate::parser::production::ProductionId;
use crate::parser::state::StateID;
use crate::parser::symbol::{NonTerminalId, TerminalId};

pub enum GrammarError {
    UndefinedSymbol,
    InvalidStartSymbol,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictAction {
    Shift(StateID),
    Reduce(ProductionId),
    Accept,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableBuildError {
    ShiftReduceConflict {
        state: StateID,
        terminal: TerminalId,
        existing: ConflictAction,
        incoming: ConflictAction,
    },
    ReduceReduceConflict {
        state: StateID,
        terminal: TerminalId,
        existing: ConflictAction,
        incoming: ConflictAction,
    },
    InvalidGoto {
        state: StateID,
        non_terminal: NonTerminalId,
        existing: StateID,
        incoming: StateID,
    },
    InvalidGrammar,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(Token),
    StackUnderflow,
    MissingProduction(ProductionId),
    MissingAction,
    MissingGoto,
}
