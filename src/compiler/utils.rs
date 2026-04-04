use std::fs::File;
use std::io::{Read, Write};
use serde::{Deserialize, Serialize};
use serde_binary_adv::{Serializer, Deserializer};
use crate::my_grammar::GrammarContext;
use crate::parser::automaton::Automaton;
use crate::parser::first::{FirstSets, NullableSet};
use crate::parser::table::ParseTable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendUtil {
    pub grammar_ctx: GrammarContext,
    pub parse_table: ParseTable,
}

impl FrontendUtil {
    pub fn build(grammar_ctx: GrammarContext) -> Result<FrontendUtil, String> {
        let g = &grammar_ctx.grammar;
        let nullable_set = NullableSet::compute(g);
        let first_sets = FirstSets::compute(g, &nullable_set);
        let automaton = Automaton::build_canonical_collection(g, &nullable_set, &first_sets)
            .map_err(|err| format!("Failed to build automaton: {:?}", err))?;
        let parse_table = ParseTable::build_parse_table(g, &automaton)
            .map_err(|err| format!("Failed to build parse table: {:?}", err))?;
        let front_end = Self{
            grammar_ctx,
            parse_table,
        };
        Ok(front_end)
    }

    pub fn save_to_file(&self, file: &mut File) -> Result<(), String> {
        let serialized = Serializer::to_bytes(self, false)
            .map_err(|err| format!("Failed to serialize FrontendUtil: {:?}", err))?;
        file.write_all(&serialized)
            .map_err(|err| format!("Failed to write FrontendUtil to file: {:?}", err))?;
        Ok(())
    }

    pub fn load_from_file(file: &mut File) -> Result<FrontendUtil, String> {
        let mut serialized = Vec::new();
        file.read_to_end(&mut serialized)
            .map_err(|err| format!("Failed to read FrontendUtil from file: {:?}", err))?;
        let front_end = Deserializer::from_bytes(&serialized, false)
            .map_err(|err| format!("Failed to deserialize FrontendUtil: {:?}", err))?;
        Ok(front_end)
    }
}
