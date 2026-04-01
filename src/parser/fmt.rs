use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use crate::parser::automation::Automaton;
use crate::parser::error::{ConflictAction, TableBuildError};
use crate::parser::first::FirstSets;
use crate::parser::grammar::Grammar;
use crate::parser::item::Lr1Item;
use crate::parser::production::{Production, ProductionId};
use crate::parser::state::ItemSet;
use crate::parser::symbol::Symbol;
use crate::parser::table::{Action, ParseTable};

/// 为需要 Grammar 上下文的结构提供统一的字符串绘制入口
pub trait DrawWithGrammar {
    fn draw(&self, grammar: &Grammar) -> String;
}

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

impl Lr1Item {
    pub fn display<'a>(&'a self, grammar: &'a Grammar) -> ItemDisplay<'a> {
        ItemDisplay{item: self, grammar}
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
            writeln!(f, "({}) [{}]", i, item.display(self.grammar))?;
        }
        Ok(())
    }
}

impl ItemSet {
    pub fn display<'a>(&'a self, grammar: &'a Grammar) -> ItemSetDisplay<'a> {
        ItemSetDisplay{items: self, grammar}
    }
}

/// 自动机可视化wrapper
pub struct AutomationDisplay<'a> {
    automation: &'a Automaton,
    grammar: &'a Grammar,
}

impl Display for AutomationDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // 首先逐个打印状态
        for (i, state) in self.automation.states.iter().enumerate() {
            write!(f, "State I{}:\n{}\n", i, state.display(self.grammar))?;
        }

        write!(f, "Transitions:\n")?;
        // 逐个打印转移
        for transition in self.automation.transitions.iter() {
            let from_state = transition.0 .0 .0;
            let symbol = &transition.0 .1;
            let to_state = transition.1 .0;
            let symbol_str = match symbol {
                Symbol::T(tid) => self.grammar.terminals[tid.0].name.clone(),
                Symbol::N(n_tid) => format!("<{}>", self.grammar.non_terminals[n_tid.0].name),
            };
            write!(f, "I{} --{}--> I{}\n", from_state, symbol_str, to_state)?;
        }

        Ok(())
    }
}

impl Automaton {
    pub fn display<'a>(&'a self, grammar: &'a Grammar) -> AutomationDisplay<'a> {
        AutomationDisplay{automation: self, grammar}
    }
}

impl DrawWithGrammar for Automaton {
    fn draw(&self, grammar: &Grammar) -> String {
        format!("{}", self.display(grammar))
    }
}

/// Action显示wrapper
pub struct ActionDisplay<'a> {
    action: &'a Action,
    grammar: &'a Grammar,
}

impl Display for ActionDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.action {
            Action::Shift(state) => write!(f, "shift I{}", state.0),
            Action::Reduce(production) => {
                write!(f, "reduce {}", self.grammar.display_production(*production))
            }
            Action::Accept => write!(f, "accept"),
        }
    }
}

impl Action {
    pub fn display<'a>(&'a self, grammar: &'a Grammar) -> ActionDisplay<'a> {
        ActionDisplay { action: self, grammar }
    }
}

/// 语法分析表显示wrapper
pub struct ParseTableDisplay<'a> {
    table: &'a ParseTable,
    grammar: &'a Grammar,
}

impl Display for ParseTableDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut states = BTreeSet::new();
        for ((state, _), _) in &self.table.action {
            states.insert(state.clone());
        }
        for ((state, _), _) in &self.table.goto {
            states.insert(state.clone());
        }

        for state in states {
            writeln!(f, "State I{}:", state.0)?;

            if self.table.action.keys().any(|(s, _)| *s == state) {
                writeln!(f, "  ACTION:")?;
                for ((s, terminal), action) in &self.table.action {
                    if *s != state {
                        continue;
                    }
                    let terminal_name = &self.grammar.terminals[terminal.0].name;
                    writeln!(f, "    {} => {}", terminal_name, action.display(self.grammar))?;
                }
            }

            if self.table.goto.keys().any(|(s, _)| *s == state) {
                writeln!(f, "  GOTO:")?;
                for ((s, non_terminal), next) in &self.table.goto {
                    if *s != state {
                        continue;
                    }
                    let non_terminal_name = &self.grammar.non_terminals[non_terminal.0].name;
                    writeln!(f, "    <{}> => I{}", non_terminal_name, next.0)?;
                }
            }

            writeln!(f)?;
        }

        Ok(())
    }
}

impl ParseTable {
    pub fn display<'a>(&'a self, grammar: &'a Grammar) -> ParseTableDisplay<'a> {
        ParseTableDisplay { table: self, grammar }
    }
}

impl DrawWithGrammar for ParseTable {
    fn draw(&self, grammar: &Grammar) -> String {
        format!("{}", self.display(grammar))
    }
}

/// 冲突动作显示wrapper
pub struct ConflictActionDisplay<'a> {
    action: &'a ConflictAction,
    grammar: &'a Grammar,
}

impl Display for ConflictActionDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.action {
            ConflictAction::Shift(state) => write!(f, "shift I{}", state.0),
            ConflictAction::Reduce(production) => {
                write!(f, "reduce {}", self.grammar.display_production(*production))
            }
            ConflictAction::Accept => write!(f, "accept"),
        }
    }
}

impl ConflictAction {
    pub fn display<'a>(&'a self, grammar: &'a Grammar) -> ConflictActionDisplay<'a> {
        ConflictActionDisplay { action: self, grammar }
    }
}

/// 构表错误显示wrapper
pub struct TableBuildErrorDisplay<'a> {
    error: &'a TableBuildError,
    grammar: &'a Grammar,
    automaton: &'a Automaton,
}

impl Display for TableBuildErrorDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.error {
            TableBuildError::ShiftReduceConflict {
                state,
                terminal,
                existing,
                incoming,
            } => {
                writeln!(
                    f,
                    "Shift/Reduce conflict at state I{} on terminal {}",
                    state.0,
                    self.grammar.terminals[terminal.0].name
                )?;
                writeln!(f, "  existing: {}", existing.display(self.grammar))?;
                writeln!(f, "  incoming: {}", incoming.display(self.grammar))?;
                if let Some(item_set) = self.automaton.states.get(state.0) {
                    writeln!(f)?;
                    writeln!(f, "State I{}:", state.0)?;
                    write!(f, "{}", item_set.display(self.grammar))?;
                }
                Ok(())
            }
            TableBuildError::ReduceReduceConflict {
                state,
                terminal,
                existing,
                incoming,
            } => {
                writeln!(
                    f,
                    "Reduce/Reduce conflict at state I{} on terminal {}",
                    state.0,
                    self.grammar.terminals[terminal.0].name
                )?;
                writeln!(f, "  existing: {}", existing.display(self.grammar))?;
                writeln!(f, "  incoming: {}", incoming.display(self.grammar))?;
                if let Some(item_set) = self.automaton.states.get(state.0) {
                    writeln!(f)?;
                    writeln!(f, "State I{}:", state.0)?;
                    write!(f, "{}", item_set.display(self.grammar))?;
                }
                Ok(())
            }
            TableBuildError::InvalidGoto {
                state,
                non_terminal,
                existing,
                incoming,
            } => {
                writeln!(
                    f,
                    "Invalid GOTO entry at state I{} on non-terminal <{}>",
                    state.0,
                    self.grammar.non_terminals[non_terminal.0].name
                )?;
                writeln!(f, "  existing: I{}", existing.0)?;
                writeln!(f, "  incoming: I{}", incoming.0)?;
                Ok(())
            }
            TableBuildError::InvalidGrammar => write!(f, "Invalid grammar while building parse table"),
        }
    }
}

impl TableBuildError {
    pub fn display<'a>(
        &'a self,
        grammar: &'a Grammar,
        automaton: &'a Automaton,
    ) -> TableBuildErrorDisplay<'a> {
        TableBuildErrorDisplay {
            error: self,
            grammar,
            automaton,
        }
    }
}
