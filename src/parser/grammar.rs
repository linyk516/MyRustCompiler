use std::collections::HashMap;
use crate::parser::production::{Production, ProductionId};
use crate::parser::symbol::*;

/// 保存完整的文法对象
/// # 定义解释
/// 包含了终结符、非终结符、产生式，起始符号位置和终结符号位置
#[derive(Debug)]
pub struct Grammar {
    pub terminals: Vec<Terminal>,
    pub non_terminals: Vec<NonTerminal>,
    pub productions: Vec<Production>,
    pub start: NonTerminalId,
    pub augmented_start: NonTerminalId,
    pub eof: TerminalId, // #
}

impl Grammar {
    pub fn terminal(&self, id: TerminalId) -> &Terminal {
        &self.terminals[id.0]
    }

    pub fn non_terminal(&self, id: TerminalId) -> &NonTerminal {
        &self.non_terminals[id.0]
    }

    pub fn production(&self, id: ProductionId) -> &Production {
        &self.productions[id.0]
    }

    pub fn productions_for_lhs(&self, lhs: NonTerminalId) -> Vec<ProductionId> {
        let mut result = Vec::new();
        for (i, production) in self.productions.iter().enumerate() {
            if production.lhs == lhs {
                result.push(ProductionId(i));
            }
        }
        result
    }

    pub fn augmented_start_production(&self) -> Option<ProductionId> {
        // augmented_start -> start
        self.productions_for_lhs(self.augmented_start).get(0).cloned()
    }

    pub fn eof(&self) -> TerminalId {
        self.eof
    }
}

/// 文法构造辅助结构
#[derive(Debug)]
pub struct GrammarBuilder {
    terminals: Vec<Terminal>,
    non_terminals: Vec<NonTerminal>,
    productions: Vec<Production>,
    start: Option<NonTerminalId>,
    eof: Option<TerminalId>,
    // 构造时辅助查找
    name_to_terminal: HashMap<String, TerminalId>,
    name_to_non_terminal: HashMap<String, NonTerminalId>,
}

/// 文法构造错误枚举
#[derive(Debug)]
pub enum GrammarBuilderErr {
    MissingStartSymbol,
    MissingEndSymbol,
}

impl GrammarBuilder {
    pub fn new() -> Self {
        GrammarBuilder{
            terminals: Vec::new(),
            non_terminals: Vec::new(),
            productions: Vec::new(),
            start: None,
            eof: None,
            name_to_terminal: HashMap::new(),
            name_to_non_terminal: HashMap::new(),
        }
    }

    /// 添加终结符，返回终结符ID，若已经包含，则返回对应ID
    pub fn add_terminal(&mut self, terminal: impl Into<String>) -> TerminalId {
        let name = terminal.into();
        if let Some(&id) = self.name_to_terminal.get(&name) {
            return id;
        }

        // 获取“下一个有效ID”
        let id = TerminalId(self.terminals.len());
        self.terminals.push(Terminal { name: name.clone()});
        self.name_to_terminal.insert(name, id);
        id
    }

    /// 添加非终结符，返回非终结符ID，若已经包含，则返回对应ID
    pub fn add_non_terminal(&mut self, non_terminal: impl Into<String>) -> NonTerminalId {
        let name = non_terminal.into();
        if let Some(&id) = self.name_to_non_terminal.get(&name) {
            return id;
        }

        // 获取“下一个有效ID”
        let id = NonTerminalId(self.non_terminals.len());
        self.non_terminals.push(NonTerminal { name: name.clone() });
        self.name_to_non_terminal.insert(name, id);
        id
    }

    /// 添加产生式
    pub fn add_production<I>(&mut self, lhs: NonTerminalId, rhs: I) -> ProductionId
    where
        I: IntoIterator<Item=Symbol>
    {
        let id = ProductionId(self.productions.len());
        self.productions.push(
            Production{
                id,
                lhs,
                rhs: rhs.into_iter().collect(),
            }
        );
        id
    }

    /// 设置起始符号
    pub fn set_start(&mut self, start: NonTerminalId) {
        self.start = Some(start);
    }

    /// 设置终止符号
    pub fn set_eof(&mut self, eof: TerminalId) {
        self.eof = Some(eof);
    }

    /// 构造文法，同时会自动添加扩展起始状态
    pub fn build(mut self) -> Result<Grammar, GrammarBuilderErr> {
        let start = self.start.ok_or(GrammarBuilderErr::MissingStartSymbol)?;
        let eof = self.eof.ok_or(GrammarBuilderErr::MissingEndSymbol)?;
        // 构造扩展起始符号，需要选取未使用的名字
        let mut augmented_start_name = format!("{}'", self.non_terminals[start.0].name);
        while self.name_to_non_terminal.contains_key(&augmented_start_name) {
            augmented_start_name.push('\'');
        }
        let augmented_start = self.add_non_terminal(augmented_start_name);
        // 添加扩展起始产生式
        self.add_production(augmented_start, [Symbol::N(start)]);
        Ok(
            Grammar{
                terminals: self.terminals,
                non_terminals: self.non_terminals,
                productions: self.productions,
                start,
                augmented_start,
                eof,
            }
        )
    }
}
