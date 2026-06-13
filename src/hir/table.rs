use std::collections::HashMap;

use crate::{
    hir::id::{DefId, LocalId},
    hir::ty::HirTy,
    lexer::token::Span,
};

#[derive(Debug, Clone)]
/// 定义表，维护收集到的顶层定义，通过 `ident -> DefId` 的索引来查找定义。
///
/// 函数和外部函数只需要名字与种类；结构体还会在 HIR lowering 后补充字段列表，
/// 供 typecheck 按字段名检查 struct literal 和字段访问。
pub struct DefTable {
    pub defs: Vec<DefData>,
    pub names: HashMap<String, DefId>,
}

#[derive(Debug, Clone)]
pub struct DefData {
    pub name: String,
    pub kind: DefKind,
    pub span: Span,
    pub struct_fields: Vec<StructFieldData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefKind {
    Fn,
    ExternFn,
    Struct,
}

#[derive(Debug, Clone)]
pub struct StructFieldData {
    pub name: String,
    pub ty: HirTy,
    pub span: Span,
}

impl DefTable {
    pub fn new() -> Self {
        Self {
            defs: vec![],
            names: HashMap::new(),
        }
    }

    pub fn alloc(&mut self, name: String, kind: DefKind, span: Span) -> DefId {
        let data = DefData {
            name: name.clone(),
            kind,
            span,
            struct_fields: vec![],
        };
        let id: DefId = self.defs.len().into();
        self.names.insert(name, id);
        self.defs.push(data);
        id
    }

    pub fn set_struct_fields(&mut self, id: DefId, fields: Vec<StructFieldData>) {
        if let Some(data) = self.defs.get_mut(id.index()) {
            data.struct_fields = fields;
        }
    }

    pub fn get(&self, id: DefId) -> Option<&DefData> {
        self.defs.get(id.index())
    }

    pub fn get_by_names(&self, name: &str) -> Option<(DefId, &DefData)> {
        let id = *self.names.get(name)?;
        let data = self.defs.get(id.index())?;

        Some((id, data))
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.names.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }
}

#[derive(Debug, Clone)]
/// 变量表（类似于符号表），记录 id->符号 的映射
pub struct LocalTable {
    pub locals: Vec<LocalData>,
}

#[derive(Debug, Clone)]
pub struct LocalData {
    pub name: String,
    pub mutable: bool,
    pub kind: LocalKind,
    pub owner: DefId,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    Param,
    Let,
    For,
    Synthetic,
}

impl LocalTable {
    pub fn new() -> Self {
        Self { locals: vec![] }
    }

    pub fn alloc(
        &mut self,
        name: String,
        mutable: bool,
        kind: LocalKind,
        owner: DefId,
        span: Span,
    ) -> LocalId {
        let id = self.locals.len().into();
        self.locals.push(LocalData {
            name,
            mutable,
            kind,
            owner,
            span,
        });
        id
    }

    pub fn get(&self, id: LocalId) -> Option<&LocalData> {
        self.locals.get(id.index())
    }

    pub fn len(&self) -> usize {
        self.locals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.locals.is_empty()
    }
}
