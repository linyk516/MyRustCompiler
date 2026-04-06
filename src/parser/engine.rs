use crate::lexer::token::{DelimiterKind, KeywordKind, LiteralKind, OperatorKind, SeparatorKind, Span, SpecialKind, Token};
use crate::lexer::token::TokenKind::*;
use crate::my_grammar::GrammarContext;
use crate::parser::cst::{CSTNode, CSTNodeID, CSTRuleNode, CSTTokenNode, CST};
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
        let mut state_stack: Vec<StateID> = vec![StateID(0)]; // I0为初始状态
        let mut node_stack: Vec<CSTNodeID> = Vec::new();
        let mut tokens = token_iter.peekable();
        let mut lookahead = tokens.next();
        let mut cst = CST { nodes: Vec::new(), root: CSTNodeID(0) };

        while let Some(token) = lookahead.clone() {
            let t = match self.current_terminal(&token) {
                Some(t) => t,
                None => return Err(ParseError::UnexpectedToken(token)),
            };

            loop {
                let current_state = state_stack.last().ok_or(ParseError::StackUnderflow)?.clone();
                let action = self.table.action.get(&(current_state, t))
                    .ok_or(ParseError::MissingAction)?;
                match action {
                    Action::Shift(next_state) => {
                        self.shift(
                            &mut state_stack,
                            &mut node_stack,
                            &mut cst,
                            next_state.clone(),
                            &t,
                            &token,
                        );
                        lookahead = tokens.next();
                        break;
                    }
                    Action::Reduce(production) => self.reduce(
                        &mut state_stack,
                        &mut node_stack,
                        &mut cst,
                        *production,
                        Some(&token),
                    )?,
                    Action::Accept => {
                        if let Some(root) = node_stack.last().copied() {
                            cst.root = root;
                        }
                        return Ok(ParseResult{ cst });
                    }
                }
            }
        }

        Err(ParseError::MissingAction)
    }

    fn shift(
        &self,
        state_stack: &mut Vec<StateID>,
        node_stack: &mut Vec<CSTNodeID>,
        cst: &mut CST,
        next_state: StateID,
        token_id: &TerminalId,
        token: &Token,
    ) {
        let node_id = cst.push_token(CSTTokenNode {
            token: token_id.clone(),
            span: token.span.clone(),
        });
        node_stack.push(node_id);
        state_stack.push(next_state);
    }

    fn reduce(
        &self,
        state_stack: &mut Vec<StateID>,
        node_stack: &mut Vec<CSTNodeID>,
        cst: &mut CST,
        production: ProductionId,
        lookahead: Option<&Token>,
    ) -> Result<(), ParseError> {
        let production = self.ctx.grammar.productions
            .get(production.0)
            .ok_or(ParseError::MissingProduction(production))?;
        let rhs_len = production.rhs.len();
        let lhs = production.lhs.clone();
        for _ in 0..rhs_len {
            state_stack.pop().ok_or(ParseError::StackUnderflow)?;
        }
        let mut children = Vec::with_capacity(rhs_len);
        for _ in 0..rhs_len {
            children.push(node_stack.pop().ok_or(ParseError::StackUnderflow)?);
        }
        children.reverse();
        let span = Self::compute_rule_span(cst, &children, lookahead);
        let node_id = cst.push_rule(CSTRuleNode {
            lhs,
            production: production.id,
            children,
            span,
        });
        node_stack.push(node_id);

        let stack_top = state_stack.last().ok_or(ParseError::StackUnderflow)?.clone();
        let next_state = self.table.goto
            .get(&(stack_top, lhs))
            .ok_or(ParseError::MissingGoto)?.clone();
        state_stack.push(next_state);
        Ok(())
    }

    fn compute_rule_span(cst: &CST, children: &[CSTNodeID], lookahead: Option<&Token>) -> Span {
        if let (Some(first), Some(last)) = (children.first(), children.last()) {
            let first_span = Self::node_span(cst, *first);
            let last_span = Self::node_span(cst, *last);
            return Span {
                start: first_span.start,
                end: last_span.end,
            };
        }

        let pos = lookahead.map(|token| token.span.start).unwrap_or(0);
        Span { start: pos, end: pos }
    }

    fn node_span(cst: &CST, node_id: CSTNodeID) -> Span {
        match cst.node(node_id) {
            CSTNode::Rule(node) => node.span.clone(),
            CSTNode::Token(node) => node.span.clone(),
        }
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
