use super::ParserEngine;
use crate::lexer::token::{KeywordKind, SeparatorKind, Span, Token, TokenKind};
use crate::my_grammar::generate_my_grammar_context;
use crate::parser::error::ParseError;
use crate::parser::production::ProductionId;
use crate::parser::state::StateID;
use crate::parser::table::{Action, ParseTable};

fn token(kind: TokenKind) -> Token {
    Token {
        kind,
        span: Span { start: 0, end: 0 },
    }
}

fn first_empty_production_id() -> ProductionId {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    ctx.grammar
        .productions
        .iter()
        .find(|p| p.rhs.is_empty())
        .map(|p| p.id)
        .expect("grammar should contain epsilon productions")
}

fn first_single_rhs_production_id() -> ProductionId {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    ctx.grammar
        .productions
        .iter()
        .find(|p| p.rhs.len() == 1)
        .map(|p| p.id)
        .expect("grammar should contain a production with one symbol on RHS")
}

#[test]
fn parse_returns_ok_on_accept_action() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let mut table = ParseTable::new();
    table
        .set_action(StateID(0), ctx.terminals.eof, Action::Accept)
        .expect("setting action should succeed");

    let tokens = vec![token(TokenKind::Eof)];
    let engine = ParserEngine::new(&table, &ctx);
    let result = engine.parse(tokens.into_iter());

    assert!(result.is_ok());
}

#[test]
fn parse_returns_ok_on_shift_then_accept() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let mut table = ParseTable::new();
    table
        .set_action(StateID(0), ctx.terminals.ident, Action::Shift(StateID(1)))
        .expect("setting action should succeed");
    table
        .set_action(StateID(1), ctx.terminals.eof, Action::Accept)
        .expect("setting action should succeed");

    let tokens = vec![token(TokenKind::Ident), token(TokenKind::Eof)];
    let engine = ParserEngine::new(&table, &ctx);
    let result = engine.parse(tokens.into_iter());

    assert!(result.is_ok());
}

#[test]
fn parse_returns_missing_action_when_input_is_empty() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let table = ParseTable::new();

    let tokens: Vec<Token> = Vec::new();
    let engine = ParserEngine::new(&table, &ctx);
    let result = engine.parse(tokens.into_iter());

    assert!(matches!(
        result,
        Err(ParseError::UnexpectedToken {
            state: StateID(0),
            lookahead: None,
            expected,
        }) if expected.is_empty()
    ));
}

#[test]
fn parse_returns_missing_action_without_accept_after_shift() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let mut table = ParseTable::new();
    table
        .set_action(StateID(0), ctx.terminals.ident, Action::Shift(StateID(1)))
        .expect("setting action should succeed");

    let tokens = vec![token(TokenKind::Ident)];
    let engine = ParserEngine::new(&table, &ctx);
    let result = engine.parse(tokens.into_iter());

    assert!(matches!(
        result,
        Err(ParseError::UnexpectedToken {
            state: StateID(1),
            lookahead: None,
            expected,
        }) if expected.is_empty()
    ));
}

#[test]
fn parse_reprocesses_same_lookahead_after_reduce() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let mut table = ParseTable::new();
    let reduce_prod = first_empty_production_id();
    let lhs = ctx
        .grammar
        .productions
        .iter()
        .find(|p| p.id == reduce_prod)
        .map(|p| p.lhs)
        .expect("reduce production should exist");

    table
        .set_action(StateID(0), ctx.terminals.ident, Action::Reduce(reduce_prod))
        .expect("setting action should succeed");
    table
        .set_goto(StateID(0), lhs, StateID(1))
        .expect("setting goto should succeed");
    table
        .set_action(StateID(1), ctx.terminals.ident, Action::Shift(StateID(2)))
        .expect("setting action should succeed");
    table
        .set_action(StateID(2), ctx.terminals.eof, Action::Accept)
        .expect("setting action should succeed");

    let tokens = vec![token(TokenKind::Ident), token(TokenKind::Eof)];
    let engine = ParserEngine::new(&table, &ctx);
    let result = engine.parse(tokens.into_iter());

    assert!(result.is_ok());
}

#[test]
fn parse_returns_missing_action_when_terminal_has_no_action_entry() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let table = ParseTable::new();

    let tokens = vec![token(TokenKind::Eof)];
    let engine = ParserEngine::new(&table, &ctx);
    let result = engine.parse(tokens.into_iter());

    assert!(matches!(
        result,
        Err(ParseError::UnexpectedToken {
            state: StateID(0),
            lookahead: Some(token),
            expected,
        }) if token.kind == TokenKind::Eof && expected.is_empty()
    ));
}

#[test]
fn parse_recovering_skips_to_sync_token_and_accepts() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let mut table = ParseTable::new();
    table
        .set_action(
            StateID(0),
            ctx.terminals.semicolon,
            Action::Shift(StateID(1)),
        )
        .expect("setting action should succeed");
    table
        .set_action(StateID(1), ctx.terminals.eof, Action::Accept)
        .expect("setting action should succeed");

    let tokens = vec![
        token(TokenKind::Ident),
        token(TokenKind::Separator(SeparatorKind::Semicolon)),
        token(TokenKind::Eof),
    ];
    let engine = ParserEngine::new(&table, &ctx);
    let outcome = engine.parse_with_recovering(tokens.into_iter());

    assert!(outcome.result.is_some());
    assert!(outcome.recovered);
    assert!(matches!(
        outcome.errors.as_slice(),
        [ParseError::UnexpectedToken {
            state: StateID(0),
            lookahead: Some(token),
            expected,
        }] if token.kind == TokenKind::Ident && expected == &vec![ctx.terminals.semicolon]
    ));
}

#[test]
fn parse_recovering_uses_statement_start_as_sync_token() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let mut table = ParseTable::new();
    table
        .set_action(StateID(0), ctx.terminals.let_, Action::Shift(StateID(1)))
        .expect("setting action should succeed");
    table
        .set_action(StateID(1), ctx.terminals.eof, Action::Accept)
        .expect("setting action should succeed");

    let tokens = vec![
        token(TokenKind::Ident),
        token(TokenKind::Keyword(KeywordKind::Let)),
        token(TokenKind::Eof),
    ];
    let engine = ParserEngine::new(&table, &ctx);
    let outcome = engine.parse_with_recovering(tokens.into_iter());

    assert!(outcome.result.is_some());
    assert!(outcome.recovered);
    assert!(matches!(
        outcome.errors.as_slice(),
        [ParseError::UnexpectedToken {
            state: StateID(0),
            lookahead: Some(token),
            expected,
        }] if token.kind == TokenKind::Ident && expected == &vec![ctx.terminals.let_]
    ));
}

#[test]
fn parse_returns_missing_production_for_invalid_reduce_id() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let mut table = ParseTable::new();
    let invalid = ProductionId(usize::MAX);
    table
        .set_action(StateID(0), ctx.terminals.eof, Action::Reduce(invalid))
        .expect("setting action should succeed");

    let tokens = vec![token(TokenKind::Eof)];
    let engine = ParserEngine::new(&table, &ctx);
    let result = engine.parse(tokens.into_iter());

    assert!(
        matches!(result, Err(ParseError::MissingProduction(ProductionId(id))) if id == usize::MAX)
    );
}

#[test]
fn parse_returns_stack_underflow_when_reduce_pops_initial_stack() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let mut table = ParseTable::new();
    let reduce_prod = first_single_rhs_production_id();
    table
        .set_action(StateID(0), ctx.terminals.eof, Action::Reduce(reduce_prod))
        .expect("setting action should succeed");

    let tokens = vec![token(TokenKind::Eof)];
    let engine = ParserEngine::new(&table, &ctx);
    let result = engine.parse(tokens.into_iter());

    assert!(matches!(result, Err(ParseError::StackUnderflow)));
}

#[test]
fn parse_returns_missing_goto_when_reduce_requires_absent_goto_entry() {
    let ctx = generate_my_grammar_context().expect("grammar context should be built");
    let mut table = ParseTable::new();
    let reduce_prod = first_empty_production_id();
    let lhs = ctx
        .grammar
        .productions
        .iter()
        .find(|p| p.id == reduce_prod)
        .map(|p| p.lhs)
        .expect("reduce production should exist");
    table
        .set_action(StateID(0), ctx.terminals.eof, Action::Reduce(reduce_prod))
        .expect("setting action should succeed");

    let tokens = vec![token(TokenKind::Eof)];
    let engine = ParserEngine::new(&table, &ctx);
    let result = engine.parse(tokens.into_iter());

    assert!(matches!(
        result,
        Err(ParseError::MissingGoto {
            state: StateID(0),
            non_terminal,
        }) if non_terminal == lhs
    ));
}
