use crate::parser::grammar::Grammar;
use crate::parser::table::ParseTable;

/// 表驱动正规LR(1)解析器
pub struct ParserEngine<'a> {
    pub table: &'a ParseTable,
    pub grammar: &'a Grammar,
}