use std::collections::BTreeSet;
use crate::parser::first::{FirstSets, NullableSet};
use crate::parser::automation::Automation;
use crate::parser::item::Lr1Item;
use crate::parser::state::ItemSet;

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
    let item_set = Automation::closure_(
        &g,
        &nullables,
        &first_sets,
        &ItemSet{
            items: BTreeSet::from([Lr1Item::new(augment_start_production, 0, g.eof())]),
        });
    println!("Closure of initial item set: \n{}", item_set.display(&g));
}