use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use crate::parser::grammar::Grammar;
use crate::parser::item::Lr1Item;
use crate::parser::symbol::Symbol;

/// 状态ID
#[derive(Ord, Eq, PartialEq, PartialOrd, Clone, Debug, Serialize, Deserialize)]
pub struct StateID(pub usize);

/// 活前缀识别DFA的一个状态，包含了一个LR(1)项目集合
#[derive(Debug, Clone)]
#[derive(Eq, Hash, PartialEq)]
pub struct ItemSet {
    pub items: BTreeSet<Lr1Item>,
}

impl ItemSet {
    pub fn new() -> ItemSet {
        ItemSet { items: BTreeSet::new()}
    }

    pub fn from_items(items: BTreeSet<Lr1Item>) -> ItemSet {
        ItemSet { items }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn insert (&mut self, item: Lr1Item) -> bool {
        self.items.insert(item)
    }
    
    pub fn iter(&self) -> impl Iterator<Item=&Lr1Item> {
        self.items.iter()
    }

    /// 获取当前所有在点后的符号，方便生成goto
    pub fn next_symbols(&self, grammar: &Grammar) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        for item in self.iter() {
            match item.next_symbol(grammar) {
                Some(sym) => symbols.push(sym),
                None => continue,
            }
        }
        symbols
    }
}