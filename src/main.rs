use std::collections::BTreeSet;
use crate::parser::first::{FirstSets, NullableSet};
use crate::parser::automation::Automaton;
use crate::parser::item::Lr1Item;
use crate::parser::state::ItemSet;
use crate::parser::symbol::{Symbol, TerminalId};

use std::fs::File;
use std::io::{Write};
use crate::parser::table::ParseTable;

pub mod lexer;
pub mod parser;
mod my_grammar;

fn main() {
    let g = my_grammar::generate_my_grammar().unwrap();
    for production in &g.productions {
        println!("{}", g.display_production(production.id));
    }
    let nullables = NullableSet::compute(&g);
    let first_sets = FirstSets::compute(&g, &nullables);
    println!("{}", first_sets.display(&g));
    let augment_start_production = g.augmented_start_production().unwrap();
    println!("Augmented start production: {}", g.display_production(augment_start_production));
    let item_set = Automaton::closure_(
        &g,
        &nullables,
        &first_sets,
        &ItemSet{
            items: BTreeSet::from([Lr1Item::new(augment_start_production, 0, g.eof())]),
        });
    println!("Closure of initial item set: \n{}", item_set.display(&g));
    let next_item_set = Automaton::goto_(
        &g,
        &nullables,
        &first_sets,
        &item_set,
        Symbol::T(TerminalId(7))
    );
    println!("Goto of initial item set on 'fn': \n{}", next_item_set.display(&g));
    let automation = Automaton::build_canonical_collection(
        &g,
        &nullables,
        &first_sets,
    ).unwrap();
    // println!("Canonical collection of LR(1) item sets: \n{}", automation.display(&g));
    // let path = "canonical_collection.txt";
    //
    // let mut output = File::create(path).unwrap();
    // write!(output, "Canonical collection of LR(1) item sets: \n{}", automation.display(&g)).unwrap();
    let parse_table = match ParseTable::build_parse_table(&g, &automation) {
        Ok(table) => table,
        Err(err) => {
            eprintln!("{}", err.display(&g, &automation));
            return;
        }
    };
    println!("LR(1) parse table: \n{}", parse_table.display(&g));
    let path = "parse_table.txt";

    let mut output = File::create(path).unwrap();
    write!(output, "LR(1) parse table: \n{}", parse_table.display(&g)).unwrap();


}
