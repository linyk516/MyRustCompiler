use crate::lexer::cursor::Cursor;
use crate::lexer::rules::*;
use crate::lexer::token::{KeywordKind, OperatorKind, Span, Token, TokenKind};
use crate::lexer::token::TokenKind::Keyword;

pub(crate) mod token;
mod cursor;
mod error;
mod tests;
mod rules;


pub(crate) struct Lexer<'a> {
    src: &'a str,
    cursor: Cursor<'a>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            cursor: Cursor::new(src),
        }
    }
}

impl Lexer<'_> {
    /// 获取并解析下一个token的类型
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        if let Some(ch) = self.cursor.peek(){
            if EOF_CH(ch) {
                return Token{
                    kind: TokenKind::Eof,
                    span: Span { start: self.cursor.pos(), end: self.cursor.pos() }
                }
            }
            self.lex_special()
                .or_else(|| self.lex_ident_or_keyword())
                .or_else(|| self.lex_number())
                .or_else(|| self.lex_operator_or_assign())
                .or_else(|| self.lex_delimiter())
                .or_else(|| self.lex_separator())
                .unwrap_or_else(|| {
                    Token{
                        kind: TokenKind::Error,
                        span: Span { start: self.cursor.pos(), end: self.cursor.pos() },
                    }
                })
        } else {
            Token{
                kind: TokenKind::Error,
                span: Span { start: self.cursor.pos(), end: self.cursor.pos() },
            }
        }
    }

    fn lex_one_or_two_char_token(
        &mut self,
        first_char_ok: fn(char) -> bool,
        classify: impl Fn(&str) -> Option<TokenKind>,
    ) -> Option<Token> {
        let l_pos = self.cursor.pos();
        let mut r_pos = l_pos;
        if let Some(ch) = self.cursor.peek() {
            if !first_char_ok(ch) { return None }
            else { r_pos = r_pos + ch.len_utf8(); }
        } else { return None }
        if let Some(ch) = self.cursor.peek_next() {
            let r_pos = r_pos + ch.len_utf8();
            let text_span = token::Span { start: l_pos, end: r_pos };
            let text = text_span.text(self.src)?;
            if let Some(token_kind) = classify(&text) {
                self.cursor.bump();
                self.cursor.bump();
                return Some (Token{
                    kind: token_kind,
                    span: text_span,
                })
            }
        }
        let text_span = token::Span { start: l_pos, end: r_pos };
        let text = text_span.text(self.src)?;
        if let Some(token_kind) = classify(&text) {
            self.cursor.bump();
            Some (Token{
                kind: token_kind,
                span: text_span,
            })
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        self.cursor.eat_while(|ch| {
            ch.is_whitespace() || ch == '\n' || ch == '\r' || ch == '\t'
        });
    }

    /// 解析标识符或关键字，接受一个以字母或下划线开头的字符串，后续可以包含字母、数字或下划线
    fn lex_ident_or_keyword(&mut self) -> Option<Token> {
        let l_pos = self.cursor.pos();
        if !self.cursor.eat_if(IDENT_KEYWORD_FIRST_CH) {
            return None
        }
        self.cursor.eat_while(IDENT_KEYWORD_CH);
        let r_pos = self.cursor.pos();
        if r_pos <= l_pos { return None } // 此情况下不会消耗字符
        let text_span = token::Span { start: l_pos, end: r_pos };
        let text = text_span.text(self.src)?;
        if let Some(keyword) = KeywordKind::from_str(&text) {
            Some(Token{
                kind: Keyword(keyword),
                span: text_span,
            })
        }
        else {
            Some(Token{
                kind: TokenKind::Ident,
                span: text_span,
            })
        }
    }

    /// 解析数值字面量，目前只支持Int32整数
    fn lex_number(&mut self) -> Option<Token> {
        let l_pos = self.cursor.pos();
        self.cursor.eat_while(INT32_LITERAL_CH);
        let r_pos = self.cursor.pos();
        if r_pos <= l_pos { return None } // 此情况下不会消耗字符
        Some(Token{
            kind: TokenKind::Literal(token::LiteralKind::Int32),
            span: token::Span { start: l_pos, end: r_pos },
        })
    }

    /// 解析操作符或赋值，目前支持单字符操作符 + - * / > < ! & 二字符操作符 == <= >= != 和赋值运算符 =
    fn lex_operator_or_assign(&mut self) -> Option<Token> {
        self.lex_one_or_two_char_token(OPERATOR_FIRST_CH, |text| {
            if text == "=" {
                Some(TokenKind::Assign)
            } else if let Some(operator) = OperatorKind::from_str(text) {
                Some(TokenKind::Operator(operator))
            } else {
                None
            }
        })
    }

    /// 解析定界符，目前支持 () {} []
    fn  lex_delimiter(&mut self) -> Option<Token> {
        let l_pos = self.cursor.pos();
        self.cursor.eat_if(DELIMITER_CH);
        let r_pos = self.cursor.pos();
        let text_span = token::Span { start: l_pos, end: r_pos };
        let text = text_span.text(self.src)?;
        if let Some(delimiter) = token::DelimiterKind::from_str(&text) {
            Some (Token{
                kind: TokenKind::Delimiter(delimiter),
                span: text_span,
            })
        } else {
            None // 保证了最长前缀匹配，返回None的情况只有可能是读到了空字符，无需回退
        }
    }

    /// 解析分隔符
    fn lex_separator(&mut self) -> Option<Token> {
        let l_pos = self.cursor.pos();
        self.cursor.eat_if(SEPARATOR_CH);
        let r_pos = self.cursor.pos();
        let text_span = token::Span { start: l_pos, end: r_pos };
        let text = text_span.text(self.src)?;
        if let Some(separator) = token::SeparatorKind::from_str(&text) {
            Some (Token{
                kind: TokenKind::Separator(separator),
                span: text_span,
            })
        } else {
            None // 保证了最长前缀匹配，返回None的情况只有可能是读到了空字符，无需回退
        }
    }

    /// 解析特殊字符，应该优先进行
    fn lex_special(&mut self) -> Option<Token> {
        self.lex_one_or_two_char_token(SPECIAL_FIRST_CH, |text| {
            if let Some(special) = token::SpecialKind::from_str(text) {
                Some(TokenKind::Special(special))
            } else {
                None
            }
        })
    }
}