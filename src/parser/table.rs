use std::collections::BTreeMap;
use crate::parser::production::ProductionId;
use crate::parser::state::StateID;
use crate::parser::symbol::{NonTerminalId, Terminal, TerminalId};

/// Action枚举项
pub enum Action {
    Shift(StateID), // 移进状态
    Reduce(ProductionId), // 按照产生式进行规约
    Accept, // 接受输入
    // TODO: 可以添加更多的Action类型，例如错误处理等
}

/// 语法分析表
/// 包含了ACTION表和GOTO表
pub struct ParseTable {
    pub action: BTreeMap<(StateID, TerminalId), Action>,
    pub goto: BTreeMap<(StateID, NonTerminalId), StateID>,
}



