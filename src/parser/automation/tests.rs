use super::Automation;
use crate::parser::first::{FirstSets, NullableSet};
use crate::parser::grammar::{Grammar, GrammarBuilder};
use crate::parser::item::Lr1Item;
use crate::parser::state::ItemSet;
use crate::parser::symbol::{Symbol, TerminalId};

fn build_grammar_with_rhs_terminal() -> (Grammar, usize, usize, TerminalId, TerminalId) {
    let mut builder = GrammarBuilder::new();

    let t_b = builder.add_terminal("b");
    let t_c = builder.add_terminal("c");
    let t_eof = builder.add_terminal("#");

    let n_s = builder.add_non_terminal("S");
    let n_b_nt = builder.add_non_terminal("B");

    let p_s = builder.add_production(n_s, vec![Symbol::N(n_b_nt), Symbol::T(t_c)]);
    let p_b = builder.add_production(n_b_nt, vec![Symbol::T(t_b)]);

    builder.set_start(n_s);
    builder.set_eof(t_eof);

    let grammar = builder.build().expect("grammar should be built");
    (grammar, p_s.0, p_b.0, t_c, t_eof)
}

#[test]
fn closure_expands_non_terminal_with_first_of_following_symbol() {
    let (grammar, p_s, p_b, t_c, t_eof) = build_grammar_with_rhs_terminal();
    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let mut kernel = ItemSet::new();
    kernel.insert(Lr1Item::new(crate::parser::production::ProductionId(p_s), 0, t_eof));

    let closure = Automation::closure_(&grammar, &nullable, &first, &kernel);

    assert_eq!(closure.len(), 2);
    assert!(closure.iter().any(|item| item.production_id().0 == p_s && item.dot() == 0 && item.lookahead == t_eof));
    assert!(closure.iter().any(|item| item.production_id().0 == p_b && item.dot() == 0 && item.lookahead == t_c));
}

#[test]
fn closure_propagates_multiple_lookaheads_when_suffix_nullable() {
    let mut builder = GrammarBuilder::new();

    let t_b = builder.add_terminal("b");
    let t_c = builder.add_terminal("c");
    let t_eof = builder.add_terminal("#");

    let n_s = builder.add_non_terminal("S");
    let n_b_nt = builder.add_non_terminal("B");
    let n_c_nt = builder.add_non_terminal("C");

    let p_s = builder.add_production(n_s, vec![Symbol::N(n_b_nt), Symbol::N(n_c_nt)]);
    let p_b = builder.add_production(n_b_nt, vec![Symbol::T(t_b)]);
    builder.add_production(n_c_nt, vec![]);
    builder.add_production(n_c_nt, vec![Symbol::T(t_c)]);

    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let mut kernel = ItemSet::new();
    kernel.insert(Lr1Item::new(p_s, 0, t_eof));

    let closure = Automation::closure_(&grammar, &nullable, &first, &kernel);

    assert!(closure.iter().any(|item| item.production_id() == p_b && item.dot() == 0 && item.lookahead == t_c));
    assert!(closure.iter().any(|item| item.production_id() == p_b && item.dot() == 0 && item.lookahead == t_eof));
}

#[test]
fn closure_deduplicates_items_on_recursive_productions() {
    let mut builder = GrammarBuilder::new();

    let t_b = builder.add_terminal("b");
    let t_eof = builder.add_terminal("#");

    let n_s = builder.add_non_terminal("S");
    let n_b_nt = builder.add_non_terminal("B");

    let p_s = builder.add_production(n_s, vec![Symbol::N(n_b_nt)]);
    let p_b_rec = builder.add_production(n_b_nt, vec![Symbol::N(n_b_nt)]);
    let p_b_term = builder.add_production(n_b_nt, vec![Symbol::T(t_b)]);

    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let mut kernel = ItemSet::new();
    kernel.insert(Lr1Item::new(p_s, 0, t_eof));

    let closure = Automation::closure_(&grammar, &nullable, &first, &kernel);

    // kernel item + two unique expanded B items
    assert_eq!(closure.len(), 3);
    assert!(closure.iter().any(|item| item.production_id() == p_s && item.dot() == 0 && item.lookahead == t_eof));
    assert!(closure.iter().any(|item| item.production_id() == p_b_rec && item.dot() == 0 && item.lookahead == t_eof));
    assert!(closure.iter().any(|item| item.production_id() == p_b_term && item.dot() == 0 && item.lookahead == t_eof));
}

