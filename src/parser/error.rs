pub enum GrammarError {
    UndefinedSymbol,
    InvalidStartSymbol,
}

pub enum TableBuildError {
    ShiftReduceConflict,
    ReduceReduceConflict,
    InvalidGrammar,
}

pub enum ParseError {
    UnexpectedToken,
    MissingAction,
    MissingGoto,
}