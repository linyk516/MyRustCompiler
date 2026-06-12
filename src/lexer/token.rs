#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn text<'a>(&self, src: &'a str) -> Option<&'a str> {
        src.get(self.start..self.end)
    }

    pub fn default() -> Self {
        Self { start: 0, end: 0 }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    Ident, // 标识符
    Keyword(KeywordKind),
    Literal(LiteralKind),
    Assign, // 赋值 =
    Operator(OperatorKind),
    Delimiter(DelimiterKind),
    Separator(SeparatorKind),
    Special(SpecialKind),
    Eof, // 文件结束 #
}

/// 关键字枚举类
#[derive(Debug, PartialEq, Clone)]
pub enum KeywordKind {
    Int32,
    Let,
    If,
    Else,
    While,
    Return,
    Mut,
    Fn,
    For,
    In,
    Loop,
    Break,
    Continue,
    Extern,
    Str,
}

impl KeywordKind {
    pub fn from_str(s: &str) -> Option<KeywordKind> {
        match s {
            "i32" => Some(KeywordKind::Int32),
            "let" => Some(KeywordKind::Let),
            "if" => Some(KeywordKind::If),
            "else" => Some(KeywordKind::Else),
            "while" => Some(KeywordKind::While),
            "return" => Some(KeywordKind::Return),
            "mut" => Some(KeywordKind::Mut),
            "fn" => Some(KeywordKind::Fn),
            "for" => Some(KeywordKind::For),
            "in" => Some(KeywordKind::In),
            "loop" => Some(KeywordKind::Loop),
            "break" => Some(KeywordKind::Break),
            "continue" => Some(KeywordKind::Continue),
            "extern" => Some(KeywordKind::Extern),
            "str" => Some(KeywordKind::Str),
            _ => None,
        }
    }
}

/// 字面值枚举类
#[derive(Debug, PartialEq, Clone)]
pub enum LiteralKind {
    Int32,
    String,
}

/// 运算符号枚举类
#[derive(Debug, PartialEq, Clone)]
pub enum OperatorKind {
    Plus,  // +
    Minus, // -
    Star,  // *
    Slash, // /
    EqEq,  // ==
    Gt,    // >
    Ge,    // >=
    Lt,    // <
    Le,    // <=
    Ne,    // !=
    Amp,   // &
}

impl OperatorKind {
    pub fn from_str(s: &str) -> Option<OperatorKind> {
        match s {
            "+" => Some(OperatorKind::Plus),
            "-" => Some(OperatorKind::Minus),
            "*" => Some(OperatorKind::Star),
            "/" => Some(OperatorKind::Slash),
            "==" => Some(OperatorKind::EqEq),
            ">" => Some(OperatorKind::Gt),
            ">=" => Some(OperatorKind::Ge),
            "<" => Some(OperatorKind::Lt),
            "<=" => Some(OperatorKind::Le),
            "!=" => Some(OperatorKind::Ne),
            "&" => Some(OperatorKind::Amp),
            _ => None,
        }
    }
}

/// 界符枚举类
#[derive(Debug, PartialEq, Clone)]
pub enum DelimiterKind {
    LParen,   // (
    RParen,   // )
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
}

impl DelimiterKind {
    pub fn from_str(s: &str) -> Option<DelimiterKind> {
        match s {
            "(" => Some(DelimiterKind::LParen),
            ")" => Some(DelimiterKind::RParen),
            "[" => Some(DelimiterKind::LBracket),
            "]" => Some(DelimiterKind::RBracket),
            "{" => Some(DelimiterKind::LBrace),
            "}" => Some(DelimiterKind::RBrace),
            _ => None,
        }
    }
}

/// 分隔符枚举类
#[derive(Debug, PartialEq, Clone)]
pub enum SeparatorKind {
    Semicolon, // ;
    Colon,     // :
    Comma,     // ,
}

impl SeparatorKind {
    pub fn from_str(s: &str) -> Option<SeparatorKind> {
        match s {
            ";" => Some(SeparatorKind::Semicolon),
            ":" => Some(SeparatorKind::Colon),
            "," => Some(SeparatorKind::Comma),
            _ => None,
        }
    }
}

/// 特殊类型枚举类
#[derive(Debug, PartialEq, Clone)]
pub enum SpecialKind {
    Arrow,    // ->
    Dot,      // .
    DotDot,   // ..
    Ellipsis, // ...
}

impl SpecialKind {
    pub fn from_str(s: &str) -> Option<SpecialKind> {
        match s {
            "->" => Some(SpecialKind::Arrow),
            "." => Some(SpecialKind::Dot),
            ".." => Some(SpecialKind::DotDot),
            "..." => Some(SpecialKind::Ellipsis),
            _ => None,
        }
    }
}
