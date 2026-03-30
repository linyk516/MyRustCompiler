/// 标识符和关键字首字符规则
pub(crate) static IDENT_KEYWORD_FIRST_CH: fn(char) -> bool = |ch| {
    ch.is_alphabetic() || ch == '_'
};

/// 标识符和关键字后续字符规则
pub(crate) static IDENT_KEYWORD_CH: fn(char) -> bool = |ch| {
    ch.is_alphanumeric() || ch == '_'
};

/// 整数字面量规则
pub(crate) static INT32_LITERAL_CH: fn(char) -> bool = |ch| {
    ch.is_numeric()
};

/// 操作符规则，识别第一个字符
pub(crate) static OPERATOR_FIRST_CH: fn(char) -> bool = |ch| {
    matches!(ch, '+' | '-' | '*' | '/' | '=' | '>' | '<' | '!' | '&')
};

// /// 操作符规则，识别第二个字符，组成可能的二字符操作符: == <= >= !=
// pub(crate) static OPERATOR_SECOND_CH: fn(char) -> bool = |ch| {
//     matches!(ch, '=')
// };

/// 定界符规则
pub(crate) static DELIMITER_CH: fn(char) -> bool = |ch| {
    matches!(ch, '(' | ')' | '{' | '}' | '[' | ']')
};

/// 分隔符规则
pub(crate) static SEPARATOR_CH: fn(char) -> bool = |ch| {
    matches!(ch, ':' | ',' | ';')
};

/// 特殊符号规则，识别可能的二字符特殊符号: -> ..
pub(crate) static SPECIAL_FIRST_CH: fn(char) -> bool = |ch| {
    matches!(ch, '-' | '.' )
};
// pub(crate) static SPECIAL_SECOND_CH: fn(char) -> bool = |ch| {
//     matches!(ch, '.' | '>')
// };

/// 文件结尾
pub(crate) static EOF_CH: fn(char) -> bool = |ch| {
    matches!(ch, '#')
};



