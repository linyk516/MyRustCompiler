use std::fmt::Display;
use crate::parser::symbol::{NonTerminalId, Symbol};


/// 产生式ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProductionId(pub usize);

/// 产生式
/// 由一个非终结符（lhs）和一个符号序列（rhs）组成
#[derive(Debug)]
pub struct Production {
    pub id: ProductionId,
    pub lhs: NonTerminalId,
    pub rhs: Vec<Symbol>,
}

impl Production {
    pub fn len(&self) -> usize {
        self.rhs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn symbol_at(&self, pos: usize) -> Option<Symbol> {
        self.rhs.get(pos).cloned()
    }
}