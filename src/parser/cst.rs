use crate::lexer::token::Span;
use crate::parser::production::ProductionId;
use crate::parser::symbol::{NonTerminalId, TerminalId};

#[derive(Debug, Clone)]
pub struct CST {
    pub nodes: Vec<CSTNode>,
    pub root: CSTNodeID,
}

#[derive(Debug, Clone)]
pub enum CSTNode {
    Rule(CSTRuleNode),
    Token(CSTTokenNode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CSTNodeID(pub usize);

#[derive(Debug, Clone)]
pub struct CSTRuleNode {
    pub lhs: NonTerminalId,
    pub production: ProductionId,
    pub children: Vec<CSTNodeID>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CSTTokenNode {
    pub token: TerminalId,
    pub span: Span,
}

impl CST {
    pub fn node(&self, id: CSTNodeID) -> &CSTNode {
        &self.nodes[id.0]
    }
    pub fn root(&self) -> CSTNodeID {
        self.root
    }
    pub fn push_rule(&mut self, node: CSTRuleNode) -> CSTNodeID {
        let node_id = CSTNodeID(self.nodes.len());
        self.nodes.push(CSTNode::Rule(node));
        node_id
    }
    pub fn push_token(&mut self, node: CSTTokenNode) -> CSTNodeID {
        let node_id = CSTNodeID(self.nodes.len());
        self.nodes.push(CSTNode::Token(node));
        node_id
    }
}
