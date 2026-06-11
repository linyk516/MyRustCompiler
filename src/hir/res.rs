use crate::hir::id::{DefId, LocalId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Res {
    Def(DefId),     // 指向顶层定义
    Local(LocalId), // 指向变量或参数
    Err,
}
