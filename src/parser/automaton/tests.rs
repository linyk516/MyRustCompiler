use super::Automaton;
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
    kernel.insert(Lr1Item::new(
        crate::parser::production::ProductionId(p_s),
        0,
        t_eof,
    ));

    let closure = Automaton::closure_(&grammar, &nullable, &first, &kernel);

    assert_eq!(closure.len(), 2);
    assert!(
        closure.iter().any(|item| item.production_id().0 == p_s
            && item.dot() == 0
            && item.lookahead == t_eof)
    );
    assert!(
        closure
            .iter()
            .any(|item| item.production_id().0 == p_b && item.dot() == 0 && item.lookahead == t_c)
    );
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

    let closure = Automaton::closure_(&grammar, &nullable, &first, &kernel);

    assert!(
        closure
            .iter()
            .any(|item| item.production_id() == p_b && item.dot() == 0 && item.lookahead == t_c)
    );
    assert!(
        closure
            .iter()
            .any(|item| item.production_id() == p_b && item.dot() == 0 && item.lookahead == t_eof)
    );
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

    let closure = Automaton::closure_(&grammar, &nullable, &first, &kernel);

    // kernel item + two unique expanded B items
    assert_eq!(closure.len(), 3);
    assert!(
        closure
            .iter()
            .any(|item| item.production_id() == p_s && item.dot() == 0 && item.lookahead == t_eof)
    );
    assert!(
        closure.iter().any(|item| item.production_id() == p_b_rec
            && item.dot() == 0
            && item.lookahead == t_eof)
    );
    assert!(closure.iter().any(|item| item.production_id() == p_b_term
        && item.dot() == 0
        && item.lookahead == t_eof));
}

#[test]
fn goto_returns_empty_when_no_item_matches_symbol() {
    let (grammar, p_s, _p_b, _t_c, t_eof) = build_grammar_with_rhs_terminal();
    let t_x = grammar
        .terminals
        .iter()
        .position(|t| t.name == "b")
        .map(TerminalId)
        .expect("b should exist");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let mut kernel = ItemSet::new();
    // S -> .B c, # ; next symbol is non-terminal B, not terminal b.
    kernel.insert(Lr1Item::new(
        crate::parser::production::ProductionId(p_s),
        0,
        t_eof,
    ));

    let goto = Automaton::goto_(&grammar, &nullable, &first, &kernel, Symbol::T(t_x));
    assert!(goto.is_empty());
}

#[test]
fn goto_shifts_items_with_matching_symbol() {
    let (grammar, p_s, _p_b, _t_c, t_eof) = build_grammar_with_rhs_terminal();
    let n_b = grammar
        .non_terminals
        .iter()
        .position(|n| n.name == "B")
        .expect("B should exist");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let mut kernel = ItemSet::new();
    kernel.insert(Lr1Item::new(
        crate::parser::production::ProductionId(p_s),
        0,
        t_eof,
    ));

    let goto = Automaton::goto_(
        &grammar,
        &nullable,
        &first,
        &kernel,
        Symbol::N(crate::parser::symbol::NonTerminalId(n_b)),
    );

    // Only S -> B . c, # should exist.
    assert_eq!(goto.len(), 1);
    assert!(
        goto.iter().any(|item| item.production_id().0 == p_s
            && item.dot() == 1
            && item.lookahead == t_eof)
    );
}

#[test]
fn goto_applies_closure_on_target_kernel() {
    let mut builder = GrammarBuilder::new();

    let t_x = builder.add_terminal("x");
    let t_hash = builder.add_terminal("#");

    let n_s = builder.add_non_terminal("S");
    let n_b = builder.add_non_terminal("B");
    let n_c = builder.add_non_terminal("C");

    let p_s = builder.add_production(n_s, vec![Symbol::N(n_b), Symbol::N(n_c)]);
    let p_b = builder.add_production(n_b, vec![Symbol::T(t_x)]);
    let p_c = builder.add_production(n_c, vec![Symbol::T(t_x)]);

    builder.set_start(n_s);
    builder.set_eof(t_hash);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let mut kernel = ItemSet::new();
    kernel.insert(Lr1Item::new(p_s, 0, t_hash));

    let goto = Automaton::goto_(&grammar, &nullable, &first, &kernel, Symbol::N(n_b));

    // Kernel shift: S -> B . C, #
    assert!(
        goto.iter()
            .any(|item| item.production_id() == p_s && item.dot() == 1 && item.lookahead == t_hash)
    );
    // Closure expansion on C because next symbol is non-terminal C.
    assert!(
        goto.iter()
            .any(|item| item.production_id() == p_c && item.dot() == 0 && item.lookahead == t_hash)
    );
    // Should not introduce unrelated B production here.
    assert!(
        !goto
            .iter()
            .any(|item| item.production_id() == p_b && item.dot() == 0 && item.lookahead == t_hash)
    );
}

#[test]
fn goto_preserves_distinct_lookaheads_after_shift() {
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
    let closure = Automaton::closure_(&grammar, &nullable, &first, &kernel);

    let goto = Automaton::goto_(&grammar, &nullable, &first, &closure, Symbol::T(t_b));

    // B -> b. should be produced with both lookaheads from closure expansion: c and #.
    assert!(
        goto.iter()
            .any(|item| item.production_id() == p_b && item.dot() == 1 && item.lookahead == t_c)
    );
    assert!(
        goto.iter()
            .any(|item| item.production_id() == p_b && item.dot() == 1 && item.lookahead == t_eof)
    );
}

#[test]
fn build_canonical_collection_succeeds_with_builder_augmented_start_production() {
    let mut builder = GrammarBuilder::new();
    let t_a = builder.add_terminal("a");
    let t_eof = builder.add_terminal("#");
    let n_s = builder.add_non_terminal("S");
    builder.add_production(n_s, vec![Symbol::T(t_a)]);
    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let result = Automaton::build_canonical_collection(&grammar, &nullable, &first);
    assert!(result.is_ok());
}

#[test]
fn build_canonical_collection_builds_states_and_transitions_for_minimal_grammar() {
    let mut builder = GrammarBuilder::new();
    let t_a = builder.add_terminal("a");
    let t_eof = builder.add_terminal("#");
    let n_s = builder.add_non_terminal("S");
    builder.add_production(n_s, vec![Symbol::T(t_a)]);
    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let automation = Automaton::build_canonical_collection(&grammar, &nullable, &first)
        .expect("canonical collection should be built");

    // For grammar S' -> S, S -> a: should at least have I0, I1, I2.
    assert!(automation.states.len() >= 3);

    // From I0 there should be transitions on S and a.
    assert!(
        automation
            .transitions
            .iter()
            .any(|((sid, sym), _)| sid.0 == 0 && *sym == Symbol::N(n_s))
    );
    assert!(
        automation
            .transitions
            .iter()
            .any(|((sid, sym), _)| sid.0 == 0 && *sym == Symbol::T(t_a))
    );
}

#[test]
fn build_canonical_collection_has_advance_target_for_shifted_terminal_item() {
    let mut builder = GrammarBuilder::new();
    let t_a = builder.add_terminal("a");
    let t_eof = builder.add_terminal("#");
    let n_s = builder.add_non_terminal("S");
    let n_a = builder.add_non_terminal("A");
    builder.add_production(n_s, vec![Symbol::N(n_a)]);
    builder.add_production(n_a, vec![Symbol::T(t_a)]);
    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let automation = Automaton::build_canonical_collection(&grammar, &nullable, &first)
        .expect("canonical collection should be built");

    // There should be some transition on terminal 'a'.
    let target = automation
        .transitions
        .iter()
        .find(|((_, sym), _)| *sym == Symbol::T(t_a))
        .map(|(_, target)| target.0);
    assert!(target.is_some());

    // The target state should include a reduce item A -> a .
    let target_state = target.expect("transition target should exist");
    assert!(
        automation.states[target_state]
            .iter()
            .any(|item| item.production_id().0 == 1 && item.dot() == 1)
    );
}
