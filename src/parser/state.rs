use std::collections::BTreeSet;
use crate::parser::item::Lr1Item;

/// 状态ID
pub struct StateID(pub usize);

/// 活前缀识别DFA的一个状态，包含了一个LR(1)项目集合
#[derive(Debug, Clone)]
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
}