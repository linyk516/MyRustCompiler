use crate::lexer::token::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeID(pub usize);

#[derive(Debug, Clone)]
pub struct AstNode<T> {
    pub id: NodeID,
    pub kind: T,
    pub span: Span,
}

impl<T> AstNode<T> {
    pub fn new(id: NodeID, kind: T, span: Span) -> Self {
        Self { id, kind, span }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> AstNode<U> {
        AstNode {
            id: self.id,
            kind: f(self.kind),
            span: self.span,
        }
    }
}

#[derive(Debug, Default)]
pub struct NodeIdAllocator {
    next: usize,
}

impl NodeIdAllocator {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn alloc(&mut self) -> NodeID {
        let id = NodeID(self.next);
        self.next += 1;
        id
    }
}
