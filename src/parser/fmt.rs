use std::fmt::{write, Display, Formatter};
use crate::parser::first::FirstSets;
use crate::parser::grammar::Grammar;
use crate::parser::item::Lr1Item;
use crate::parser::production::{Production, ProductionId};
use crate::parser::state::ItemSet;
use crate::parser::symbol::Symbol;

/// 产生式显示wrapper
pub struct ProductionDisplay<'a> {
    production: &'a Production,
    grammar: &'a Grammar,
}

impl Display for ProductionDisplay<'_>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let lhs = &self.grammar.non_terminals[self.production.lhs.0];
        let rhs: Vec<String> = self.production.rhs.iter().map(|sym| {
            match sym {
                Symbol::T(tid) => {
                    format!("{}", self.grammar.terminals[tid.0].name)
                },
                Symbol::N(n_tid) => {
                    format!("<{}>", self.grammar.non_terminals[n_tid.0].name)
                },
            }
        }).collect();
        write!(f, "<{}> -> {}", lhs.name, rhs.join(" "))
    }
}

impl Grammar {
    pub fn display_production(&'_ self, id: ProductionId) -> ProductionDisplay<'_> {
        ProductionDisplay{
            production: &self.productions[id.0],
            grammar: self,
        }
    }
}

/// First集显示wrapper
pub struct FirstSetsDisplay<'a> {
    first: &'a FirstSets,
    grammar: &'a Grammar,
}

impl FirstSets {
    pub fn display<'a>(&'a self, grammar: &'a Grammar) -> FirstSetsDisplay<'a> {
        FirstSetsDisplay {
            first: self,
            grammar,
        }
    }
}

impl Display for FirstSetsDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, sym) in self.grammar.non_terminals.iter().enumerate() {
            let first_set: Vec<String> = self.first.get_first_sets()[i].iter().
                map(|tid| {
                self.grammar.terminals[tid.0].name.clone()
            }).collect();
            writeln!(f, "FIRST(<{}>) = {{ {} }}", sym.name, first_set.join(", "))?;
        }
        Ok(())
    }
}

/// 项目显示wrapper
pub struct ItemDisplay<'a> {
    item: &'a Lr1Item,
    grammar: &'a Grammar,
}

impl Display for ItemDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let production = &self.grammar.productions[self.item.core.production.0];
        let mut rhs: Vec<String> = production.rhs.iter().map(|sym| {
            match sym {
                Symbol::T(tid) => {
                    format!("{}", self.grammar.terminals[tid.0].name)
                },
                Symbol::N(n_tid) => {
                    format!("<{}>", self.grammar.non_terminals[n_tid.0].name)
                },
            }
        }).collect();
        rhs.insert(self.item.core.dot, "·".to_string());
        let lookahead = &self.grammar.terminals[self.item.lookahead.0].name;
        let lhs = &self.grammar.non_terminals[production.lhs.0].name;
        write!(f, "<{}> -> {}, {}", lhs, rhs.join(" "), lookahead)?;
        Ok(())
    }
}

/// 项目集显示wrapper
pub struct ItemSetDisplay<'a> {
    items: &'a ItemSet,
    grammar: &'a Grammar,
}

impl Display for ItemSetDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, item) in self.items.iter().enumerate() {
            writeln!(f, "({}) {}", i, ItemDisplay{item, grammar: self.grammar})?;
        }
        Ok(())
    }
}

impl ItemSet {
    pub fn display<'a>(&'a self, grammar: &'a Grammar) -> ItemSetDisplay<'a> {
        ItemSetDisplay{items: self, grammar}
    }
}