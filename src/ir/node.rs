use std::collections::HashMap;

use crate::{
    ast::ty::BinaryOp,
    hir::id::DefId,
    ir::id::{
        IrBlockId, IrExternalFunctionId, IrFunctionId, IrGlobalStringId, IrSlotId, IrValueId,
    },
    lexer::token::Span,
    thir::id::ThirLocalId,
    typecheck::ty::TyId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
/// LLVM-like IR 类型。`Ptr` 使用现代 LLVM opaque pointer 风格。
pub enum IrTy {
    I1,
    I8,
    I16,
    I32,
    I64,
    Void,
    Ptr,
    Array { elem: Box<IrTy>, len: usize },
    Struct(Vec<IrTy>),
    Error,
}

impl IrTy {
    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }

    pub fn is_i1(&self) -> bool {
        matches!(self, Self::I1)
    }
}

#[derive(Debug, Clone)]
/// LLVM-like IR 顶层结构，按函数保存基本块和 typed instruction。
pub struct IrProgram {
    pub functions: Vec<IrFunction>,
    pub function_map: HashMap<DefId, IrFunctionId>,
    pub extern_functions: Vec<IrExternalFunction>,
    pub extern_function_map: HashMap<DefId, IrExternalFunctionId>,
    pub global_strings: Vec<IrGlobalString>,
}

impl IrProgram {
    pub fn new() -> Self {
        Self {
            functions: vec![],
            function_map: HashMap::new(),
            extern_functions: vec![],
            extern_function_map: HashMap::new(),
            global_strings: vec![],
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

    pub fn alloc_extern_function(
        &mut self,
        owner: DefId,
        function: IrExternalFunction,
    ) -> IrExternalFunctionId {
        let id = IrExternalFunctionId(self.extern_functions.len());
        self.extern_function_map.insert(owner, id);
        self.extern_functions.push(function);
        id
    }

    pub fn extern_function(&self, id: IrExternalFunctionId) -> Option<&IrExternalFunction> {
        self.extern_functions.get(id.index())
    }

    pub fn alloc_global_string(&mut self, string: IrGlobalString) -> IrGlobalStringId {
        let id = IrGlobalStringId(self.global_strings.len());
        self.global_strings.push(string);
        id
    }

    pub fn global_string(&self, id: IrGlobalStringId) -> Option<&IrGlobalString> {
        self.global_strings.get(id.index())
    }
}

#[derive(Debug, Clone)]
pub struct IrExternalFunction {
    pub owner: DefId,
    pub symbol_name: String,
    pub params: Vec<IrTy>,
    pub ret_ty: IrTy,
    pub variadic: bool,
}

#[derive(Debug, Clone)]
pub struct IrGlobalString {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct IrFunction {
    pub owner: DefId,
    pub symbol_name: String,
    pub params: Vec<IrParam>,
    pub ret_ty: IrTy,
    pub slots: Vec<IrSlot>,
    pub values: Vec<IrValue>,
    pub blocks: Vec<IrBasicBlock>,
    pub entry: IrBlockId,
}

impl IrFunction {
    pub fn new(owner: DefId, symbol_name: String, ret_ty: IrTy) -> Self {
        Self {
            owner,
            symbol_name,
            params: vec![],
            ret_ty,
            slots: vec![],
            values: vec![],
            blocks: vec![],
            entry: IrBlockId(usize::MAX),
        }
    }

    pub fn alloc_param(&mut self, ty: IrTy) -> IrValueId {
        let index = self.params.len();
        let value = self.alloc_value(IrValue {
            ty: ty.clone(),
            kind: IrValueKind::Param(index),
            name: Some(format!("arg{index}")),
        });
        self.params.push(IrParam { ty, value });
        value
    }

    pub fn alloc_slot(&mut self, slot: IrSlot) -> IrSlotId {
        let id = IrSlotId(self.slots.len());
        self.slots.push(slot);
        id
    }

    pub fn alloc_value(&mut self, value: IrValue) -> IrValueId {
        let id = IrValueId(self.values.len());
        self.values.push(value);
        id
    }

    pub fn alloc_block(&mut self, label: impl Into<String>) -> IrBlockId {
        let id = IrBlockId(self.blocks.len());
        self.blocks.push(IrBasicBlock::new(label.into()));
        id
    }

    pub fn slot(&self, id: IrSlotId) -> Option<&IrSlot> {
        self.slots.get(id.index())
    }

    pub fn slot_mut(&mut self, id: IrSlotId) -> Option<&mut IrSlot> {
        self.slots.get_mut(id.index())
    }

    pub fn value(&self, id: IrValueId) -> Option<&IrValue> {
        self.values.get(id.index())
    }

    pub fn block(&self, id: IrBlockId) -> Option<&IrBasicBlock> {
        self.blocks.get(id.index())
    }

    pub fn block_mut(&mut self, id: IrBlockId) -> Option<&mut IrBasicBlock> {
        self.blocks.get_mut(id.index())
    }
}

#[derive(Debug, Clone)]
pub struct IrParam {
    pub ty: IrTy,
    pub value: IrValueId,
}

#[derive(Debug, Clone)]
pub struct IrSlot {
    pub thir_local: Option<ThirLocalId>,
    pub name: String,
    pub mutable: bool,
    pub source_ty: TyId,
    pub value_ty: IrTy,
    pub addr: Option<IrValueId>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IrValue {
    pub ty: IrTy,
    pub kind: IrValueKind,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum IrValueKind {
    ConstInt(i32),
    Unit,
    Param(usize),
    SlotAddr(IrSlotId),
    GlobalStringAddr(IrGlobalStringId),
    InstrResult,
}

#[derive(Debug, Clone)]
pub struct IrBasicBlock {
    pub label: String,
    pub instrs: Vec<IrInstr>,
    pub terminator: Option<IrTerminator>,
}

impl IrBasicBlock {
    pub fn new(label: String) -> Self {
        Self {
            label,
            instrs: vec![],
            terminator: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IrInstr {
    pub result: Option<IrValueId>,
    pub kind: IrInstrKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum IrInstrKind {
    Alloca {
        alloc_ty: IrTy,
    },
    Load {
        ty: IrTy,
        ptr: IrValueId,
    },
    Store {
        ty: IrTy,
        value: IrValueId,
        ptr: IrValueId,
    },
    Gep {
        source_ty: IrTy,
        base: IrValueId,
        indices: Vec<IrValueId>,
    },
    Binary {
        op: IrBinaryOp,
        ty: IrTy,
        lhs: IrValueId,
        rhs: IrValueId,
    },
    Icmp {
        pred: IrIcmpPred,
        ty: IrTy,
        lhs: IrValueId,
        rhs: IrValueId,
    },
    Zext {
        from_ty: IrTy,
        value: IrValueId,
        to_ty: IrTy,
    },
    Call {
        callee: DefId,
        ret_ty: IrTy,
        param_tys: Vec<IrTy>,
        variadic: bool,
        args: Vec<(IrTy, IrValueId)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrBinaryOp {
    Add,
    Sub,
    Mul,
    SDiv,
}

impl IrBinaryOp {
    pub fn from_binary(op: BinaryOp) -> Option<Self> {
        match op {
            BinaryOp::Add => Some(Self::Add),
            BinaryOp::Sub => Some(Self::Sub),
            BinaryOp::Mul => Some(Self::Mul),
            BinaryOp::Div => Some(Self::SDiv),
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrIcmpPred {
    Eq,
    Ne,
    Slt,
    Sle,
    Sgt,
    Sge,
}

impl IrIcmpPred {
    pub fn from_binary(op: BinaryOp) -> Option<Self> {
        match op {
            BinaryOp::Eq => Some(Self::Eq),
            BinaryOp::Ne => Some(Self::Ne),
            BinaryOp::Lt => Some(Self::Slt),
            BinaryOp::Le => Some(Self::Sle),
            BinaryOp::Gt => Some(Self::Sgt),
            BinaryOp::Ge => Some(Self::Sge),
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrTerminator {
    Br {
        target: IrBlockId,
    },
    CondBr {
        cond: IrValueId,
        then_bb: IrBlockId,
        else_bb: IrBlockId,
    },
    Ret {
        ty: IrTy,
        value: Option<IrValueId>,
    },
    Unreachable,
}
