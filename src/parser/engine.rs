use crate::lexer::token::{DelimiterKind, KeywordKind, LiteralKind, OperatorKind, SeparatorKind, SpecialKind, Token};
use crate::lexer::token::TokenKind::*;
use crate::my_grammar::GrammarContext;
use crate::parser::error::ParseError;
use crate::parser::ParseResult;
use crate::parser::production::ProductionId;
use crate::parser::state::StateID;
use crate::parser::symbol::TerminalId;
use crate::parser::table::{Action, ParseTable};

/// 表驱动正规LR(1)解析器
pub struct ParserEngine<'a> {
    pub table: &'a ParseTable,
    pub ctx: &'a GrammarContext,
}

impl<'a> ParserEngine<'a> {
    pub fn new(table: &'a ParseTable, ctx: &'a GrammarContext) -> Self {
        ParserEngine { table, ctx }
    }

    pub fn parse(&self, token_iter: impl Iterator<Item = Token>) -> Result<ParseResult, ParseError> {
        let mut stack: Vec<StateID> = vec![StateID(0)]; // I0为初始状态
        let mut tokens = token_iter.peekable();
        let mut lookahead = tokens.next();

        while let Some(token) = lookahead.clone() {
            let t = match self.current_terminal(&token) {
                Some(t) => t,
                None => return Err(ParseError::UnexpectedToken(token)),
            };

            loop {
                let current_state = stack.last().ok_or(ParseError::StackUnderflow)?.clone();
                let action = self.table.action.get(&(current_state, t))
                    .ok_or(ParseError::MissingAction)?;
                match action {
                    Action::Shift(next_state) => {
                        self.shift(&mut stack, next_state.clone());
                        lookahead = tokens.next();
                        break;
                    }
                    Action::Reduce(production) => self.reduce(&mut stack, *production)?,
                    Action::Accept => return Ok(ParseResult{}),
                }
            }
        }

        Err(ParseError::MissingAction)
    }

    fn shift(&self, stack: &mut Vec<StateID>, next_state: StateID) {
        stack.push(next_state);
    }

    fn reduce(&self, stack: &mut Vec<StateID>, production: ProductionId) -> Result<(), ParseError> {
        let production = self.ctx.grammar.productions
            .get(production.0)
            .ok_or(ParseError::MissingProduction(production))?;
        let rhs_len = production.rhs.len();
        let lhs = production.lhs.clone();
        for _ in 0..rhs_len {
            stack.pop().ok_or(ParseError::StackUnderflow)?;
        }
        let stack_top = stack.last().ok_or(ParseError::StackUnderflow)?.clone();
        let next_state = self.table.goto
            .get(&(stack_top, lhs))
            .ok_or(ParseError::MissingGoto)?.clone();
        stack.push(next_state);
        Ok(())
    }

    fn current_terminal(&self, token: &Token) -> Option<TerminalId> {
        let t = &self.ctx.terminals;
        match &token.kind {
            Ident => Some(t.ident),
            Keyword(kind) => match kind {
                KeywordKind::Fn => Some(t.fn_),
                KeywordKind::Int32 => Some(t.i32_),
                KeywordKind::Let => Some(t.let_),
                KeywordKind::If => Some(t.if_),
                KeywordKind::Else => Some(t.else_),
                KeywordKind::While => Some(t.while_),
                KeywordKind::Return => Some(t.return_),
                KeywordKind::Mut => Some(t.mut_),
                KeywordKind::For => Some(t.for_),
                KeywordKind::In => Some(t.in_),
                KeywordKind::Loop => Some(t.loop_),
                KeywordKind::Break => Some(t.break_),
                KeywordKind::Continue => Some(t.continue_),
            },
            Literal(kind) => match kind {
                LiteralKind::Int32 => Some(t.literal_i32),
            },
            Assign => Some(t.assignment),
            Operator(kind) => match kind {
                OperatorKind::Plus => Some(t.plus),
                OperatorKind::Minus => Some(t.minus),
                OperatorKind::Star => Some(t.star),
                OperatorKind::Slash => Some(t.slash),
                OperatorKind::EqEq => Some(t.eqeq),
                OperatorKind::Gt => Some(t.gt),
                OperatorKind::Ge => Some(t.ge),
                OperatorKind::Lt => Some(t.lt),
                OperatorKind::Le => Some(t.le),
                OperatorKind::Ne => Some(t.ne),
                OperatorKind::Amp => Some(t.amp),
            },
            Delimiter(kind) => match kind {
                DelimiterKind::LParen => Some(t.l_paren),
                DelimiterKind::RParen => Some(t.r_paren),
                DelimiterKind::LBrace => Some(t.l_bracket),
                DelimiterKind::RBrace => Some(t.r_bracket),
                DelimiterKind::LBracket => Some(t.l_brace),
                DelimiterKind::RBracket => Some(t.r_brace),
            },
            Separator(kind) => match kind {
                SeparatorKind::Semicolon => Some(t.semicolon),
                SeparatorKind::Colon => Some(t.colon),
                SeparatorKind::Comma => Some(t.comma),
            },
            Special(kind) => match kind {
                SpecialKind::Arrow => Some(t.arrow),
                SpecialKind::Dot => Some(t.dot),
                SpecialKind::DotDot => Some(t.dotdot),
            },
            Eof => Some(t.eof),
            Error => None,
        }
    }
}

#[cfg(test)]
mod tests;
