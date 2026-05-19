use super::{FirstSets, NullableSet};
use crate::parser::grammar::GrammarBuilder;
use crate::parser::symbol::Symbol;

#[test]
fn compute_nullable_set_handles_direct_and_indirect_nullable() {
    let mut builder = GrammarBuilder::new();

    let t_a = builder.add_terminal("a");
    let t_b = builder.add_terminal("b");
    let t_d = builder.add_terminal("d");
    let t_eof = builder.add_terminal("#");

    let n_s = builder.add_non_terminal("S");
    let n_a = builder.add_non_terminal("A");
    let n_b = builder.add_non_terminal("B");
    let n_c = builder.add_non_terminal("C");
    let n_d = builder.add_non_terminal("D");

    builder.add_production(n_s, vec![Symbol::N(n_a), Symbol::N(n_b)]);
    builder.add_production(n_a, vec![]);
    builder.add_production(n_a, vec![Symbol::T(t_a)]);
    builder.add_production(n_b, vec![Symbol::N(n_c)]);
    builder.add_production(n_c, vec![]);
    builder.add_production(n_c, vec![Symbol::T(t_b)]);
    builder.add_production(n_d, vec![Symbol::T(t_d)]);

    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);

    assert!(nullable.inner[n_s.0]);
    assert!(nullable.inner[n_a.0]);
    assert!(nullable.inner[n_b.0]);
    assert!(nullable.inner[n_c.0]);
    assert!(!nullable.inner[n_d.0]);
}

#[test]
fn compute_first_sets_handles_nullable_prefix_and_multiple_productions() {
    let mut builder = GrammarBuilder::new();

    let t_a = builder.add_terminal("a");
    let t_b = builder.add_terminal("b");
    let t_c = builder.add_terminal("c");
    let t_d = builder.add_terminal("d");
    let t_eof = builder.add_terminal("#");

    let n_s = builder.add_non_terminal("S");
    let n_a = builder.add_non_terminal("A");
    let n_b = builder.add_non_terminal("B");
    let n_c = builder.add_non_terminal("C");

    // S -> A B | c
    // A -> a | ε
    // B -> b | C
    // C -> d | ε
    builder.add_production(n_s, vec![Symbol::N(n_a), Symbol::N(n_b)]);
    builder.add_production(n_s, vec![Symbol::T(t_c)]);
    builder.add_production(n_a, vec![Symbol::T(t_a)]);
    builder.add_production(n_a, vec![]);
    builder.add_production(n_b, vec![Symbol::T(t_b)]);
    builder.add_production(n_b, vec![Symbol::N(n_c)]);
    builder.add_production(n_c, vec![Symbol::T(t_d)]);
    builder.add_production(n_c, vec![]);

    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    assert!(first.inner[n_a.0].contains(&t_a));
    assert_eq!(first.inner[n_a.0].len(), 1);

    assert!(first.inner[n_c.0].contains(&t_d));
    assert_eq!(first.inner[n_c.0].len(), 1);

    assert!(first.inner[n_b.0].contains(&t_b));
    assert!(first.inner[n_b.0].contains(&t_d));
    assert_eq!(first.inner[n_b.0].len(), 2);

    assert!(first.inner[n_s.0].contains(&t_a));
    assert!(first.inner[n_s.0].contains(&t_b));
    assert!(first.inner[n_s.0].contains(&t_c));
    assert!(first.inner[n_s.0].contains(&t_d));
    assert_eq!(first.inner[n_s.0].len(), 4);
}

#[test]
fn compute_first_sets_propagates_across_indirect_chain() {
    let mut builder = GrammarBuilder::new();

    let t_x = builder.add_terminal("x");
    let t_eof = builder.add_terminal("#");

    let n_s = builder.add_non_terminal("S");
    let n_a = builder.add_non_terminal("A");
    let n_b = builder.add_non_terminal("B");

    // S -> A
    // A -> B
    // B -> x
    builder.add_production(n_s, vec![Symbol::N(n_a)]);
    builder.add_production(n_a, vec![Symbol::N(n_b)]);
    builder.add_production(n_b, vec![Symbol::T(t_x)]);

    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    assert!(first.inner[n_b.0].contains(&t_x));
    assert!(first.inner[n_a.0].contains(&t_x));
    assert!(first.inner[n_s.0].contains(&t_x));
    assert_eq!(first.inner[n_s.0].len(), 1);
}

#[test]
fn first_of_sequence_handles_nullable_prefix_and_lookahead_fallback() {
    let mut builder = GrammarBuilder::new();

    let t_a = builder.add_terminal("a");
    let t_b = builder.add_terminal("b");
    let t_c = builder.add_terminal("c");
    let t_z = builder.add_terminal("z");
    let t_eof = builder.add_terminal("#");

    let n_s = builder.add_non_terminal("S");
    let n_a = builder.add_non_terminal("A");
    let n_b = builder.add_non_terminal("B");
    let n_c = builder.add_non_terminal("C");

    // S -> A B
    // A -> ε | a
    // B -> C | b
    // C -> ε | c
    builder.add_production(n_s, vec![Symbol::N(n_a), Symbol::N(n_b)]);
    builder.add_production(n_a, vec![]);
    builder.add_production(n_a, vec![Symbol::T(t_a)]);
    builder.add_production(n_b, vec![Symbol::N(n_c)]);
    builder.add_production(n_b, vec![Symbol::T(t_b)]);
    builder.add_production(n_c, vec![]);
    builder.add_production(n_c, vec![Symbol::T(t_c)]);

    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let seq_first = first.first_of_sequence(&nullable, &[Symbol::N(n_a), Symbol::N(n_b)], &t_z);

    assert!(seq_first.contains(&t_a));
    assert!(seq_first.contains(&t_b));
    assert!(seq_first.contains(&t_c));
    assert!(seq_first.contains(&t_z));
    assert_eq!(seq_first.len(), 4);
}

#[test]
fn first_of_sequence_stops_after_terminal_in_sequence() {
    let mut builder = GrammarBuilder::new();

    let t_a = builder.add_terminal("a");
    let t_b = builder.add_terminal("b");
    let t_c = builder.add_terminal("c");
    let t_z = builder.add_terminal("z");
    let t_eof = builder.add_terminal("#");

    let n_s = builder.add_non_terminal("S");
    let n_a = builder.add_non_terminal("A");
    let n_c = builder.add_non_terminal("C");

    // A -> ε | a
    // C -> c
    builder.add_production(n_s, vec![Symbol::N(n_a), Symbol::T(t_b), Symbol::N(n_c)]);
    builder.add_production(n_a, vec![]);
    builder.add_production(n_a, vec![Symbol::T(t_a)]);
    builder.add_production(n_c, vec![Symbol::T(t_c)]);

    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let seq_first = first.first_of_sequence(
        &nullable,
        &[Symbol::N(n_a), Symbol::T(t_b), Symbol::N(n_c)],
        &t_z,
    );

    assert!(seq_first.contains(&t_a));
    assert!(seq_first.contains(&t_b));
    assert!(!seq_first.contains(&t_c));
    assert!(!seq_first.contains(&t_z));
    assert_eq!(seq_first.len(), 2);
}

#[test]
fn first_of_sequence_returns_lookahead_for_empty_sequence() {
    let mut builder = GrammarBuilder::new();

    let t_lookahead = builder.add_terminal("lookahead");
    let t_eof = builder.add_terminal("#");
    let n_s = builder.add_non_terminal("S");

    builder.add_production(n_s, Vec::<Symbol>::new());
    builder.set_start(n_s);
    builder.set_eof(t_eof);
    let grammar = builder.build().expect("grammar should be built");

    let nullable = NullableSet::compute(&grammar);
    let first = FirstSets::compute(&grammar, &nullable);

    let seq_first = first.first_of_sequence(&nullable, &[], &t_lookahead);

    assert!(seq_first.contains(&t_lookahead));
    assert_eq!(seq_first.len(), 1);
}
