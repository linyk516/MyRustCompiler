use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use crate::parser::automaton::Automaton;
use crate::parser::error::{ConflictAction, TableBuildError};
use crate::parser::grammar::Grammar;
use crate::parser::production::ProductionId;
use crate::parser::state::StateID;
use crate::parser::symbol::{NonTerminalId, Symbol, TerminalId};
use serde::{Serialize, Deserialize};
use serde_binary_adv::{Serializer, Deserializer};
/// Action枚举项
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Shift(StateID), // 移进状态
    Reduce(ProductionId), // 按照产生式进行规约
    Accept, // 接受输入
    // TODO: 可以添加更多的Action类型，例如错误处理等
}

/// 语法分析表
/// 包含了ACTION表和GOTO表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseTable {
    pub action: BTreeMap<(StateID, TerminalId), Action>,
    pub goto: BTreeMap<(StateID, NonTerminalId), StateID>,
}

impl ParseTable {
    fn action_to_conflict_action(action: &Action) -> ConflictAction {
        match action {
            Action::Shift(state) => ConflictAction::Shift(state.clone()),
            Action::Reduce(production) => ConflictAction::Reduce(*production),
            Action::Accept => ConflictAction::Accept,
        }
    }

    pub fn new() -> Self {
        ParseTable {
            action: BTreeMap::new(),
            goto: BTreeMap::new(),
        }
    }

    pub fn action(&self, id: StateID, terminal: TerminalId) -> Option<&Action> {
        self.action.get(&(id, terminal))
    }

    pub fn goto(&self, id: StateID, non_terminal: NonTerminalId) -> Option<StateID> {
        self.goto.get(&(id, non_terminal)).cloned()
    }

    pub fn set_action(
        &mut self,
        state: StateID,
        terminal: TerminalId,
        action: Action,
    ) -> Result<(), TableBuildError> {
        match self.action.get(&(state.clone(), terminal)) {
            None => {
                self.action.insert((state, terminal), action);
                Ok(())
            }
            Some(existing) if *existing == action => Ok(()),
            Some(existing) => {
                let existing_action = Self::action_to_conflict_action(existing);
                let incoming_action = Self::action_to_conflict_action(&action);
                match (&existing_action, &incoming_action) {
                    (ConflictAction::Reduce(_), ConflictAction::Reduce(_)) => {
                        Err(TableBuildError::ReduceReduceConflict {
                            state,
                            terminal,
                            existing: existing_action,
                            incoming: incoming_action,
                        })
                    }
                    _ => Err(TableBuildError::ShiftReduceConflict {
                        state,
                        terminal,
                        existing: existing_action,
                        incoming: incoming_action,
                    }),
                }
            },
        }
    }

    pub fn set_goto(
        &mut self,
        state: StateID,
        non_terminal: NonTerminalId,
        next: StateID,
    ) -> Result<(), TableBuildError> {
        match self.goto.get(&(state.clone(), non_terminal)) {
            None => {
                self.goto.insert((state, non_terminal), next);
                Ok(())
            }
            Some(existing) if *existing == next => Ok(()),
            Some(existing) => Err(TableBuildError::InvalidGoto {
                state,
                non_terminal,
                existing: existing.clone(),
                incoming: next,
            }),
        }
    }

    pub fn build_parse_table(
        grammar: &Grammar,
        automaton: &Automaton,
    ) -> Result<ParseTable, TableBuildError> {
        let mut table = ParseTable::new();
        let augmented_start_prod = grammar
            .augmented_start_production()
            .ok_or(TableBuildError::InvalidGrammar)?;

        // 先根据自动机中的边填入 shift / goto 项
        for ((state, symbol), next_state) in &automaton.transitions {
            match symbol {
                Symbol::T(terminal) => {
                    table.set_action(
                        state.clone(),
                        *terminal,
                        Action::Shift(next_state.clone()),
                    )?;
                }
                Symbol::N(non_terminal) => {
                    table.set_goto(state.clone(), *non_terminal, next_state.clone())?;
                }
            }
        }

        // 再根据状态中的规约项目填入 reduce / accept 项
        for (index, item_set) in automaton.states.iter().enumerate() {
            let state = StateID(index);
            for item in item_set.iter() {
                if !item.is_reduce_item(grammar) {
                    continue;
                }

                let production_id = item.production_id();
                if production_id == augmented_start_prod && item.lookahead == grammar.eof() {
                    table.set_action(state.clone(), item.lookahead, Action::Accept)?;
                } else {
                    table.set_action(
                        state.clone(),
                        item.lookahead,
                        Action::Reduce(production_id),
                    )?;
                }
            }
        }

        Ok(table)
    }
}

impl ParseTable {
    pub fn save_to_file(&self, file: &mut File) -> std::io::Result<()> {
        let serialized = Serializer::to_bytes(&self, false)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        file.write_all(&serialized)?;
        Ok(())
    }

    pub fn load_from_file(file: &mut File) -> std::io::Result<ParseTable> {
        let mut serialized = Vec::new();
        file.read_to_end(&mut serialized)?;
        let deserialized: ParseTable = Deserializer::from_bytes(&serialized, false)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(deserialized)
    }
}

