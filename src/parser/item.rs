use crate::parser::grammar::Grammar;
use crate::parser::production::ProductionId;
use crate::parser::symbol::{NonTerminalId, Symbol, TerminalId};

/// 指示产生式和点号位置
///
/// 对于abcde
/// - 如果点号在a前面，那么dot=0
/// - 如果点号在b前面，那么dot=1
/// - ...
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct ItemCore {
    pub production: ProductionId,
    pub dot: usize,
}

impl ItemCore {
    pub fn new(production: ProductionId, dot: usize) -> ItemCore {
        ItemCore { production, dot }
    }
}

/// Lr1项目
///
/// 即表示形如 [A→alpha·beta, a] 的项目，其中A是一个非终结符，alpha和beta是符号序列，a是一个终结符
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct Lr1Item {
    pub core: ItemCore,
    pub lookahead: TerminalId,
}

impl Lr1Item {
    pub fn new(production: ProductionId, dot: usize, lookahead: TerminalId) -> Lr1Item {
        Lr1Item { core: ItemCore::new(production, dot), lookahead }
    }

    pub fn production_id(&self) -> ProductionId {
        self.core.production
    }

    pub fn dot(&self) -> usize {
        self.core.dot
    }

    pub fn is_reduce_item(&self, grammar: &Grammar) -> bool {
        let production = &grammar.productions[self.core.production.0];
        // 如果点号位置已经在产生式右侧的末尾了，那么就是规约项目
        self.core.dot >= production.rhs.len()
    }

    pub fn next_symbol(&self, grammar: &Grammar) -> Option<Symbol> {
        let production = &grammar.productions[self.core.production.0];
        production.rhs.get(self.core.dot).cloned()
    }

    pub fn has_next_symbol(&self, grammar: &Grammar) -> bool {
        let production = &grammar.productions[self.core.production.0];
        production.rhs.get(self.core.dot).is_some()
    }

    pub fn advance(&self) -> Self {
        Lr1Item {
            core: ItemCore::new(self.core.production, self.core.dot + 1),
            lookahead: self.lookahead,
        }
    }
}
impl Lr1Item {
    /// 创建当前dot位置之后的符号序列，不包含lookahead
    pub fn construct_sequence_after_next_symbol<'a>(&self, grammar: &'a Grammar) -> &'a[Symbol] {
        let production = &grammar.productions[self.core.production.0];
        if production.rhs.len() <= self.core.dot {
            return &[];
        }
        // 保证不会panic
        &production.rhs[self.core.dot + 1..]
    }
}

#[cfg(test)]
mod tests;
