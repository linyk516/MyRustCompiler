use std::collections::{BTreeMap, VecDeque};
use crate::parser::first::{FirstSets, NullableSet};
use crate::parser::grammar::Grammar;
use crate::parser::item::Lr1Item;
use crate::parser::state::{ItemSet, StateID};
use crate::parser::symbol::Symbol;

/// 定义活前缀识别自动机，包含状态和相应的转移关系
pub struct Automation {
    pub states: Vec<ItemSet>,
    pub transitions: BTreeMap<(StateID, Symbol), ItemSet>,
}

impl Automation {
    pub fn new() -> Self {
        Automation{
            states: Vec::new(),
            transitions: BTreeMap::new(),
        }
    }

    /// 计算项目集I的闭包，输出新的项目集
    pub fn closure_(grammar: &Grammar, nullable_set: &NullableSet, first_sets: &FirstSets, items: &ItemSet) -> ItemSet {
        let mut closure = items.clone();
        let mut queue: VecDeque<Lr1Item> = items.iter().cloned().collect();
        while let Some(item) = queue.pop_front() {
            // 保证有下一个符号
            // 检查符号是否为非终结符
            let sym = match item.next_symbol(grammar) {
                Some(s) => {
                    match s {
                        Symbol::N(sym) => sym,
                        Symbol::T(_) => continue,
                    }
                }
                None => continue,
            };
            let lookaheads = first_sets.first_of_sequence(
                nullable_set,
                item.construct_sequence_after_next_symbol(grammar),
                &item.lookahead,
            );
            for production in &grammar.productions_for_lhs(sym) {
                for lookahead in &lookaheads {
                    let new_item = Lr1Item::new(*production, 0, *lookahead);
                    if closure.insert(new_item.clone()) {
                        queue.push_back(new_item);
                    }
                }
            }
        }
        closure
    }
}

#[cfg(test)]
mod tests;

