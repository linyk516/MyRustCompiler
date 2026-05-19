use crate::parser::grammar::Grammar;
use crate::parser::symbol::Symbol::T;
use crate::parser::symbol::{NonTerminalId, Symbol, TerminalId};
use std::collections::BTreeSet;

/// 保存所有非终结符的First集
pub struct FirstSets {
    inner: Vec<BTreeSet<TerminalId>>,
}

/// 保存所有终结符是否可以推出空串
pub struct NullableSet {
    inner: Vec<bool>,
}
impl FirstSets {
    pub fn get_first_sets(&self) -> &Vec<BTreeSet<TerminalId>> {
        &self.inner
    }

    /// 计算文法的First集
    pub fn compute(grammar: &Grammar, nullables: &NullableSet) -> Self {
        let mut first = FirstSets {
            inner: vec![BTreeSet::new(); grammar.non_terminals.len()],
        };

        let mut change_flag = true;
        while change_flag {
            change_flag = false;

            for production in &grammar.productions {
                let lhs = production.lhs.0;

                for sym in &production.rhs {
                    match sym {
                        Symbol::T(tid) => {
                            if first.inner[lhs].insert(*tid) {
                                change_flag = true;
                            }
                            break;
                        }
                        Symbol::N(n_tid) => {
                            let rhs_first: BTreeSet<TerminalId> = first.inner[n_tid.0].clone();
                            let before_size = first.inner[lhs].len();
                            first.inner[lhs].extend(rhs_first);
                            if (first.inner[lhs].len() - before_size) > 0 {
                                change_flag = true;
                            }

                            if !nullables.is_nullable(*n_tid) {
                                break;
                            }
                        }
                    }
                }
            }
        }

        first
    }

    pub fn first_of_sequence(
        &self,
        nullables: &NullableSet,
        seq: &[Symbol],
        lookahead: &TerminalId,
    ) -> BTreeSet<TerminalId> {
        let first_sets = self.get_first_sets();
        let mut first: BTreeSet<TerminalId> = BTreeSet::new();
        let seq_with_lookahead = [seq, &[T(*lookahead)]].concat();
        for &sym in seq_with_lookahead.iter() {
            match sym {
                Symbol::T(tid) => {
                    first.insert(tid);
                    break;
                }
                Symbol::N(n_tid) => {
                    first.append(&mut first_sets[n_tid.0].clone());
                    if !nullables.is_nullable(n_tid) {
                        break;
                    }
                }
            }
        }
        first
    }
}

impl NullableSet {
    pub fn is_nullable(&self, n_tid: NonTerminalId) -> bool {
        self.inner[n_tid.0]
    }

    pub fn compute(grammar: &Grammar) -> Self {
        let mut change_flag: bool = true;
        let mut nullable: NullableSet = NullableSet {
            inner: vec![false; grammar.non_terminals.len()],
        };
        while change_flag {
            change_flag = false;
            for production in &grammar.productions {
                let lhs = production.lhs.0;
                // 代表非终结符可以推出空串
                if production.rhs.is_empty() {
                    if !nullable.inner[lhs] {
                        nullable.inner[lhs] = true;
                        change_flag = true;
                    }
                    continue;
                }
                let mut is_all_null: bool = true;
                for sym in &production.rhs {
                    match sym {
                        Symbol::T(_) => {
                            is_all_null = false;
                            break;
                        }
                        Symbol::N(n_tid) => {
                            if !nullable.inner[n_tid.0] {
                                is_all_null = false;
                                break;
                            }
                        }
                    }
                }
                if is_all_null {
                    if !nullable.inner[lhs] {
                        nullable.inner[lhs] = true;
                        change_flag = true;
                    }
                }
            }
        }
        nullable
    }
}

#[cfg(test)]
mod tests;
