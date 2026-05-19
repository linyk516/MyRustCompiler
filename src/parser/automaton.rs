use crate::parser::first::{FirstSets, NullableSet};
use crate::parser::grammar::Grammar;
use crate::parser::item::Lr1Item;
use crate::parser::state::{ItemSet, StateID};
use crate::parser::symbol::Symbol;
use std::collections::{BTreeMap, HashMap, VecDeque};

#[derive(Debug)]
pub enum AutomationBuildErr {
    GrammarError,
}

/// 定义活前缀识别自动机，包含状态和相应的转移关系
pub struct Automaton {
    pub states: Vec<ItemSet>,
    pub transitions: BTreeMap<(StateID, Symbol), StateID>,
}

impl Automaton {
    pub fn new() -> Self {
        Automaton {
            states: Vec::new(),
            transitions: BTreeMap::new(),
        }
    }

    /// 计算项目集I的闭包，输出新的项目集
    pub fn closure_(
        grammar: &Grammar,
        nullable_set: &NullableSet,
        first_sets: &FirstSets,
        items: &ItemSet,
    ) -> ItemSet {
        let mut closure = items.clone();
        let mut queue: VecDeque<Lr1Item> = items.iter().cloned().collect();
        while let Some(item) = queue.pop_front() {
            // 保证有下一个符号
            // 检查符号是否为非终结符
            let sym = match item.next_symbol(grammar) {
                Some(s) => match s {
                    Symbol::N(sym) => sym,
                    Symbol::T(_) => continue,
                },
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

    /// 计算GOTO(I,X)，输出目标项目集合
    pub fn goto_(
        grammar: &Grammar,
        nullable_set: &NullableSet,
        first_sets: &FirstSets,
        items: &ItemSet,
        symbol: Symbol,
    ) -> ItemSet {
        let mut goto_set = ItemSet::new();
        for item in items.items.iter() {
            let next_sym = match item.next_symbol(grammar) {
                Some(sym) => sym,
                None => continue,
            };
            if next_sym == symbol {
                // 若点号的下一个符号是目标转移符号，则吸收并下移
                goto_set.insert(item.clone().advance());
            }
        }
        goto_set = Self::closure_(grammar, &nullable_set, first_sets, &goto_set);
        goto_set
    }

    /// 构建正则项目集
    pub fn build_canonical_collection(
        grammar: &Grammar,
        nullable_set: &NullableSet,
        first_sets: &FirstSets,
    ) -> Result<Automaton, AutomationBuildErr> {
        let mut automation = Automaton::new();
        let mut initial_item_set = ItemSet::new();
        initial_item_set.insert(Lr1Item::new(
            grammar
                .augmented_start_production()
                .ok_or(AutomationBuildErr::GrammarError)?,
            0,
            grammar.eof,
        ));
        // 构造I0，作为迭代基础
        let initial_item_set =
            Self::closure_(grammar, &nullable_set, &first_sets, &initial_item_set);
        automation.states.push(initial_item_set.clone());
        // 构造待处理集合队列和已出现记录
        let mut queue: VecDeque<ItemSet> = VecDeque::new();
        queue.push_back(initial_item_set.clone());
        let mut seen: HashMap<ItemSet, StateID> = HashMap::new();
        seen.insert(initial_item_set, StateID(0));
        while let Some(item) = queue.pop_front() {
            let current_state_id = seen[&item].clone();
            let mut next_symbols = item.next_symbols(grammar);
            next_symbols.sort();
            next_symbols.dedup();
            for next_symbol in next_symbols {
                let new_item_set =
                    Self::goto_(grammar, &nullable_set, &first_sets, &item, next_symbol);
                if new_item_set.is_empty() {
                    continue;
                }
                if let Some(new_state_id) = seen.get(&new_item_set).cloned() {
                    // 已出现过，直接添加转移
                    automation
                        .transitions
                        .insert((current_state_id.clone(), next_symbol), new_state_id);
                } else {
                    // 未出现过，加入并添加转移
                    let new_state_id = StateID(automation.states.len());
                    automation.states.push(new_item_set.clone());
                    seen.insert(new_item_set.clone(), new_state_id.clone());
                    queue.push_back(new_item_set);
                    automation
                        .transitions
                        .insert((current_state_id.clone(), next_symbol), new_state_id);
                }
            }
        }
        Ok(automation)
    }
}

#[cfg(test)]
mod tests;
