use std::collections::HashMap;

use crate::{
    ast::ty::BinaryOp,
    hir::id::DefId,
    ir::id::{IrBlockId, IrFunctionId, IrLocalId, IrTempId},
    lexer::token::Span,
    thir::id::ThirLocalId,
    typecheck::ty::TyId,
};

#[derive(Debug, Clone)]
/// 四元式 IR 顶层结构，按函数保存已经线性化为基本块的中间代码。
pub struct IrProgram {
    pub functions: Vec<IrFunction>,
    pub function_map: HashMap<DefId, IrFunctionId>,
}

impl IrProgram {
    pub fn new() -> Self {
        Self {
            functions: vec![],
            function_map: HashMap::new(),
        }
    }

    pub fn alloc_function(&mut self, owner: DefId, function: IrFunction) -> IrFunctionId {
        let id = IrFunctionId(self.functions.len());
        self.function_map.insert(owner, id);
        self.functions.push(function);
        id
    }

    pub fn function(&self, id: IrFunctionId) -> Option<&IrFunction> {
        self.functions.get(id.index())
    }
}

#[derive(Debug, Clone)]
pub struct IrFunction {
    pub owner: DefId,
    pub locals: Vec<IrLocal>,
    pub temps: Vec<IrTemp>,
    pub blocks: Vec<IrBasicBlock>,
    pub entry: IrBlockId,
}

impl IrFunction {
    pub fn new(owner: DefId) -> Self {
        Self {
            owner,
            locals: vec![],
            temps: vec![],
            blocks: vec![],
            entry: IrBlockId(usize::MAX),
        }
    }

    pub fn alloc_local(&mut self, local: IrLocal) -> IrLocalId {
        let id = IrLocalId(self.locals.len());
        self.locals.push(local);
        id
    }

    pub fn alloc_temp(&mut self, temp: IrTemp) -> IrTempId {
        let id = IrTempId(self.temps.len());
        self.temps.push(temp);
        id
    }

    pub fn alloc_block(&mut self) -> IrBlockId {
        let id = IrBlockId(self.blocks.len());
        self.blocks.push(IrBasicBlock::new());
        id
    }

    pub fn local(&self, id: IrLocalId) -> Option<&IrLocal> {
        self.locals.get(id.index())
    }

    pub fn temp(&self, id: IrTempId) -> Option<&IrTemp> {
        self.temps.get(id.index())
    }

    pub fn block(&self, id: IrBlockId) -> Option<&IrBasicBlock> {
        self.blocks.get(id.index())
    }

    pub fn block_mut(&mut self, id: IrBlockId) -> Option<&mut IrBasicBlock> {
        self.blocks.get_mut(id.index())
    }
}

#[derive(Debug, Clone)]
pub struct IrLocal {
    pub thir_local: Option<ThirLocalId>,
    pub name: String,
    pub mutable: bool,
    pub ty: TyId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IrTemp {
    pub ty: TyId,
}

#[derive(Debug, Clone)]
pub struct IrBasicBlock {
    pub quads: Vec<Quad>,
    pub terminator: Terminator,
}

impl IrBasicBlock {
    pub fn new() -> Self {
        Self {
            quads: vec![],
            terminator: Terminator::Unreachable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrOperand {
    ConstInt(i32),
    Param(usize),
    Local(IrLocalId),
    Temp(IrTempId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrPlace {
    Local(IrLocalId),
    Temp(IrTempId),
}

#[derive(Debug, Clone)]
pub struct Quad {
    pub op: QuadOp,
    pub arg1: Option<IrOperand>,
    pub arg2: Option<IrOperand>,
    pub result: Option<IrPlace>,
    pub span: Span,
}

impl Quad {
    pub fn new(
        op: QuadOp,
        arg1: Option<IrOperand>,
        arg2: Option<IrOperand>,
        result: Option<IrPlace>,
        span: Span,
    ) -> Self {
        Self {
            op,
            arg1,
            arg2,
            result,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuadOp {
    Alloca,
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Load,
    Store,
    Gep,
    Arg,
    Call(DefId),
}

impl QuadOp {
    pub fn from_binary(op: BinaryOp) -> Self {
        match op {
            BinaryOp::Add => Self::Add,
            BinaryOp::Sub => Self::Sub,
            BinaryOp::Mul => Self::Mul,
            BinaryOp::Div => Self::Div,
            BinaryOp::Eq => Self::Eq,
            BinaryOp::Ne => Self::Ne,
            BinaryOp::Lt => Self::Lt,
            BinaryOp::Le => Self::Le,
            BinaryOp::Gt => Self::Gt,
            BinaryOp::Ge => Self::Ge,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator {
    Goto(IrBlockId),
    If {
        cond: IrOperand,
        then_bb: IrBlockId,
        else_bb: IrBlockId,
    },
    Return(Option<IrOperand>),
    Unreachable,
}
