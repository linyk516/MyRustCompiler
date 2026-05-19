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

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedToken {
        // 记录当前分析状态，下一个字符和预期的字符
        state: StateID,
        lookahead: Option<Token>,
        expected: Vec<TerminalId>,
    },
    StackUnderflow,
    MissingProduction(ProductionId),
    MissingGoto {
        // 记录当前分析状态和非终结符
        state: StateID,
        non_terminal: NonTerminalId,
    },
}
