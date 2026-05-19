use super::{ItemCore, Lr1Item};
use crate::parser::grammar::{Grammar, GrammarBuilder};
use crate::parser::production::ProductionId;
use crate::parser::symbol::{Symbol, TerminalId};

fn build_test_grammar() -> (Grammar, ProductionId, ProductionId, TerminalId) {
    let mut builder = GrammarBuilder::new();

    let t_a = builder.add_terminal("a");
    let t_b = builder.add_terminal("b");
    let t_eof = builder.add_terminal("#");

    let n_s = builder.add_non_terminal("S");
    let n_a = builder.add_non_terminal("A");
    let t_la = builder.add_terminal("la");

    let p0 = builder.add_production(n_s, vec![Symbol::T(t_a), Symbol::N(n_a), Symbol::T(t_b)]);
    let p1 = builder.add_production(n_a, Vec::<Symbol>::new());

    builder.set_start(n_s);
    builder.set_eof(t_eof);

    let grammar = builder.build().expect("grammar should be built");
    (grammar, p0, p1, t_la)
}

#[test]
fn item_core_new_sets_fields() {
    let core = ItemCore::new(ProductionId(7), 3);
    assert_eq!(core.production.0, 7);
    assert_eq!(core.dot, 3);
}

#[test]
fn lr1_item_new_and_getters_work() {
    let item = Lr1Item::new(ProductionId(2), 1, TerminalId(9));
    assert_eq!(item.production_id().0, 2);
    assert_eq!(item.dot(), 1);
    assert_eq!(item.lookahead.0, 9);
}

#[test]
fn next_symbol_and_has_next_symbol_follow_dot_position() {
    let (grammar, p0, _, lookahead) = build_test_grammar();

    let item0 = Lr1Item::new(p0, 0, lookahead);
    assert!(matches!(item0.next_symbol(&grammar), Some(Symbol::T(_))));
    assert!(item0.has_next_symbol(&grammar));

    let item1 = Lr1Item::new(p0, 1, lookahead);
    assert!(matches!(item1.next_symbol(&grammar), Some(Symbol::N(_))));
    assert!(item1.has_next_symbol(&grammar));

    let item2 = Lr1Item::new(p0, 2, lookahead);
    assert!(matches!(item2.next_symbol(&grammar), Some(Symbol::T(_))));
    assert!(item2.has_next_symbol(&grammar));
}

#[test]
fn next_symbol_is_none_at_or_beyond_end() {
    let (grammar, p0, _, lookahead) = build_test_grammar();

    let at_end = Lr1Item::new(p0, 3, lookahead);
    assert_eq!(at_end.next_symbol(&grammar), None);
    assert!(!at_end.has_next_symbol(&grammar));

    let beyond_end = Lr1Item::new(p0, 10, lookahead);
    assert_eq!(beyond_end.next_symbol(&grammar), None);
    assert!(!beyond_end.has_next_symbol(&grammar));
}

#[test]
fn is_reduce_item_for_empty_and_non_empty_productions() {
    let (grammar, p0, p1, lookahead) = build_test_grammar();

    let non_reduce = Lr1Item::new(p0, 2, lookahead);
    assert!(!non_reduce.is_reduce_item(&grammar));

    let reduce_at_end = Lr1Item::new(p0, 3, lookahead);
    assert!(reduce_at_end.is_reduce_item(&grammar));

    let reduce_beyond_end = Lr1Item::new(p0, 4, lookahead);
    assert!(reduce_beyond_end.is_reduce_item(&grammar));

    let empty_prod_item = Lr1Item::new(p1, 0, lookahead);
    assert!(empty_prod_item.is_reduce_item(&grammar));
}

#[test]
fn advance_increments_dot_and_keeps_identity_fields() {
    let (_, p0, _, lookahead) = build_test_grammar();

    let item = Lr1Item::new(p0, 1, lookahead);
    let advanced = item.advance();

    assert_eq!(advanced.production_id().0, p0.0);
    assert_eq!(advanced.dot(), 2);
    assert_eq!(advanced.lookahead.0, lookahead.0);

    // Original item should remain unchanged.
    assert_eq!(item.dot(), 1);
}
