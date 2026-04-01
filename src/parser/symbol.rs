/// 终结符
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminal {
    pub name: String,
}

/// 非终结符
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonTerminal {
    pub name: String,
}

/// 终结符ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalId(pub usize);

/// 非终结符ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct  NonTerminalId(pub usize);

/// 一般文法符号
/// 分为终结符和非终结符，并保存相应的唯一ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Symbol {
    T(TerminalId),
    N(NonTerminalId),
}

impl Symbol {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Symbol::T(_))
    }
    pub fn is_non_terminal(&self) -> bool {
        matches!(self, Symbol::N(_))
    }
}

impl From<TerminalId> for Symbol {
    fn from(value: TerminalId) -> Self {
        Symbol::T(value)
    }
}

impl From<NonTerminalId> for Symbol {
    fn from(value: NonTerminalId) -> Self {
        Symbol::N(value)
    }
}