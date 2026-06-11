use std::collections::HashMap;

use crate::hir::id::LocalId;

/// 作用域栈
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

/// 作用域，维护作用域中变量名到符号表id的映射
pub struct Scope {
    pub kind: ScopeKind,
    pub bindings: HashMap<String, LocalId>,
}

impl Scope {
    pub fn new(kind: ScopeKind) -> Self {
        Self {
            kind,
            bindings: HashMap::new(),
        }
    }
}

/// 额外的作用域种类信息
pub enum ScopeKind {
    Function,
    Block,
    Loop,
    For,
}

/// 作用域中声明相关的错误
pub enum ScopeDeclareError {
    NoScope,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self { scopes: vec![] }
    }

    /// 创建并进入一个新的作用域
    pub fn enter(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope::new(kind));
    }

    /// 退出并返回退出的作用域（如果栈中仍有作用域）
    pub fn exit(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }

    pub fn depth(&self) -> usize {
        self.scopes.iter().len()
    }

    pub fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }

    pub fn current(&self) -> Option<&Scope> {
        self.scopes.last()
    }

    pub fn current_mut(&mut self) -> Option<&mut Scope> {
        self.scopes.last_mut()
    }

    pub fn declare(&mut self, name: String, local_id: LocalId) -> Result<(), ScopeDeclareError> {
        let cur_scope = self.scopes.last_mut().ok_or(ScopeDeclareError::NoScope)?;

        cur_scope.bindings.insert(name, local_id);
        Ok(())
    }

    /// 从当前作用域开始，向外层作用域逐层寻找变量
    pub fn resolve_local(&self, name: &str) -> Option<LocalId> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.bindings.get(name) {
                return Some(id.clone());
            }
        }
        None
    }

    /// 寻找当前作用域中的变量
    pub fn contains_in_current(&self, name: &str) -> Option<LocalId> {
        let cur_scope = self.scopes.last()?;

        cur_scope.bindings.get(name).copied()
    }
}
