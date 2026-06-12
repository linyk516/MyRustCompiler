#[cfg(test)]
mod cursor_tests {
    use super::super::cursor::Cursor;

    #[test]
    fn cursor_new_starts_at_zero() {
        let mut cursor = Cursor::new("abc");
        assert_eq!(cursor.pos(), 0);
        assert!(!cursor.is_eof());
    }

    #[test]
    fn cursor_peek_on_empty_returns_none() {
        let mut cursor = Cursor::new("");
        assert_eq!(cursor.peek(), None);
        assert_eq!(cursor.peek_next(), None);
        assert_eq!(cursor.bump(), None);
        assert!(cursor.is_eof());
        assert_eq!(cursor.pos(), 0);
    }

    #[test]
    fn cursor_peek_next_on_single_char_returns_none() {
        let cursor = Cursor::new("a");
        assert_eq!(cursor.peek(), Some('a'));
        assert_eq!(cursor.peek_next(), None);
    }

    #[test]
    fn cursor_bump_advances_and_returns_char() {
        let mut cursor = Cursor::new("ab");

        assert_eq!(cursor.bump(), Some('a'));
        assert_eq!(cursor.pos(), 1);
        assert_eq!(cursor.peek(), Some('b'));
        assert_eq!(cursor.peek_next(), None);

        assert_eq!(cursor.bump(), Some('b'));
        assert_eq!(cursor.pos(), 2);
        assert!(cursor.is_eof());
        assert_eq!(cursor.bump(), None);
    }

    #[test]
    fn cursor_is_eof_after_consuming_all_chars() {
        let mut cursor = Cursor::new("xyz");
        assert!(!cursor.is_eof());

        cursor.bump();
        cursor.bump();
        cursor.bump();

        assert!(cursor.is_eof());
        assert_eq!(cursor.peek(), None);
    }

    #[test]
    fn cursor_eat_if_true_consumes_one_char() {
        let mut cursor = Cursor::new("abc");

        assert!(cursor.eat_if(|ch| ch == 'a'));
        assert_eq!(cursor.pos(), 1);
        assert_eq!(cursor.peek(), Some('b'));
    }

    #[test]
    fn cursor_eat_if_false_keeps_position() {
        let mut cursor = Cursor::new("abc");

        assert!(!cursor.eat_if(|ch| ch == 'z'));
        assert_eq!(cursor.pos(), 0);
        assert_eq!(cursor.peek(), Some('a'));
    }

    #[test]
    fn cursor_eat_while_consumes_until_predicate_fails() {
        let mut cursor = Cursor::new("123abc");

        cursor.eat_while(|ch| ch.is_ascii_digit());

        assert_eq!(cursor.pos(), 3);
        assert_eq!(cursor.peek(), Some('a'));
        assert_eq!(cursor.peek_next(), Some('b'));
    }

    #[test]
    fn cursor_eat_while_no_match_keeps_position() {
        let mut cursor = Cursor::new("abc");

        cursor.eat_while(|ch| ch.is_ascii_digit());

        assert_eq!(cursor.pos(), 0);
        assert_eq!(cursor.peek(), Some('a'));
    }

    #[test]
    fn cursor_eat_while_on_eof_is_idempotent() {
        let mut cursor = Cursor::new("");

        cursor.eat_while(|_| true);

        assert_eq!(cursor.pos(), 0);
        assert!(cursor.is_eof());
    }

    #[test]
    fn cursor_pos_tracks_utf8_byte_offset() {
        let mut cursor = Cursor::new("a中b");

        assert_eq!(cursor.pos(), 0);
        assert_eq!(cursor.bump(), Some('a'));
        assert_eq!(cursor.pos(), 1);

        assert_eq!(cursor.bump(), Some('中'));
        assert_eq!(cursor.pos(), 4);

        assert_eq!(cursor.bump(), Some('b'));
        assert_eq!(cursor.pos(), 5);
        assert!(cursor.is_eof());
    }
}

#[cfg(test)]
mod lexer_ident_or_keyword_tests {
    use super::super::Lexer;
    use super::super::token::{KeywordKind, TokenKind};

    fn assert_ident(token: &super::super::token::Token) {
        assert!(matches!(token.kind, TokenKind::Ident));
    }

    fn assert_keyword(token: &super::super::token::Token, text: &str) {
        match text {
            "i32" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Int32))),
            "let" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Let))),
            "if" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::If))),
            "else" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Else))),
            "while" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::While))),
            "return" => assert!(matches!(
                token.kind,
                TokenKind::Keyword(KeywordKind::Return)
            )),
            "mut" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Mut))),
            "fn" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Fn))),
            "for" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::For))),
            "in" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::In))),
            "loop" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Loop))),
            "break" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Break))),
            "continue" => assert!(matches!(
                token.kind,
                TokenKind::Keyword(KeywordKind::Continue)
            )),
            "extern" => assert!(matches!(
                token.kind,
                TokenKind::Keyword(KeywordKind::Extern)
            )),
            "str" => assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Str))),
            _ => panic!("unexpected keyword text: {text}"),
        }
    }

    #[test]
    fn lex_ident_or_keyword_parses_keyword_let() {
        let mut lexer = Lexer::new("let value");

        let token = lexer
            .lex_ident_or_keyword()
            .expect("token should be produced");

        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 3);
        assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Let)));
        assert_eq!(lexer.cursor.pos(), 3);
    }

    #[test]
    fn lex_ident_or_keyword_parses_identifier_with_digits_and_underscore() {
        let mut lexer = Lexer::new("name_123+");

        let token = lexer
            .lex_ident_or_keyword()
            .expect("token should be produced");

        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 8);
        assert!(matches!(token.kind, TokenKind::Ident));
        assert_eq!(lexer.cursor.pos(), 8);
    }

    #[test]
    fn lex_ident_or_keyword_parses_i32_as_keyword() {
        let mut lexer = Lexer::new("i32");

        let token = lexer
            .lex_ident_or_keyword()
            .expect("token should be produced");

        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 3);
        assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Int32)));
        assert_eq!(lexer.cursor.pos(), 3);
    }

    #[test]
    fn lex_ident_or_keyword_covers_all_keywords() {
        let keywords = [
            "i32", "let", "if", "else", "while", "return", "mut", "fn", "for", "in", "loop",
            "break", "continue", "extern", "str",
        ];

        for keyword in keywords {
            let src = format!("{} ", keyword);
            let mut lexer = Lexer::new(&src);

            let token = lexer
                .lex_ident_or_keyword()
                .expect("token should be produced");

            assert_eq!(token.span.start, 0);
            assert_eq!(token.span.end, keyword.len());
            assert_keyword(&token, keyword);
            assert_eq!(lexer.cursor.pos(), keyword.len());
        }
    }

    #[test]
    fn lex_ident_or_keyword_treats_keyword_like_text_as_ident() {
        let cases = ["let1", "_let", "Let", "continue_", "i32x"];

        for case in cases {
            let src = format!("{}+", case);
            let mut lexer = Lexer::new(&src);

            let token = lexer
                .lex_ident_or_keyword()
                .expect("token should be produced");

            assert_eq!(token.span.start, 0);
            assert_eq!(token.span.end, case.len());
            assert_ident(&token);
            assert_eq!(lexer.cursor.pos(), case.len());
        }
    }

    #[test]
    fn lex_ident_or_keyword_stops_on_non_ident_characters() {
        let mut lexer = Lexer::new("name+(rest)");

        let token = lexer
            .lex_ident_or_keyword()
            .expect("token should be produced");

        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 4);
        assert_ident(&token);
        assert_eq!(lexer.cursor.pos(), 4);
        assert_eq!(lexer.cursor.peek(), Some('+'));
    }

    #[test]
    fn lex_ident_or_keyword_works_from_non_zero_cursor_position() {
        let mut lexer = Lexer::new("let value+");
        lexer.cursor.bump();
        lexer.cursor.bump();
        lexer.cursor.bump();
        lexer.cursor.bump();

        let token = lexer
            .lex_ident_or_keyword()
            .expect("token should be produced");

        assert_eq!(token.span.start, 4);
        assert_eq!(token.span.end, 9);
        assert_ident(&token);
        assert_eq!(lexer.cursor.pos(), 9);
        assert_eq!(lexer.cursor.peek(), Some('+'));
    }

    #[test]
    fn lex_ident_or_keyword_returns_none_when_not_on_ident_start() {
        let mut lexer = Lexer::new("+");

        let token = lexer.lex_ident_or_keyword();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert_eq!(lexer.cursor.peek(), Some('+'));
    }

    #[test]
    fn lex_ident_or_keyword_returns_none_on_empty_input() {
        let mut lexer = Lexer::new("");

        let token = lexer.lex_ident_or_keyword();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert!(lexer.cursor.is_eof());
    }

    #[test]
    fn lex_ident_or_keyword_returns_none_when_starts_with_digit() {
        let mut lexer = Lexer::new("123abc");

        let token = lexer.lex_ident_or_keyword();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert_eq!(lexer.cursor.peek(), Some('1'));
    }

    #[test]
    fn lex_ident_or_keyword_accepts_underscore_start_identifier() {
        let mut lexer = Lexer::new("_value1+");

        let token = lexer
            .lex_ident_or_keyword()
            .expect("token should be produced");

        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 7);
        assert_ident(&token);
        assert_eq!(lexer.cursor.pos(), 7);
        assert_eq!(lexer.cursor.peek(), Some('+'));
    }

    #[test]
    fn lex_ident_or_keyword_accepts_utf8_alphanumeric_identifier() {
        let mut lexer = Lexer::new("变量1+");

        let token = lexer
            .lex_ident_or_keyword()
            .expect("token should be produced");

        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, "变量1".len());
        assert_ident(&token);
        assert_eq!(lexer.cursor.pos(), "变量1".len());
        assert_eq!(lexer.cursor.peek(), Some('+'));
    }
}

#[cfg(test)]
mod lexer_number_tests {
    use super::super::Lexer;
    use super::super::token::{LiteralKind, TokenKind};

    #[test]
    fn lex_number_parses_single_zero() {
        let mut lexer = Lexer::new("0+");

        let token = lexer.lex_number().expect("token should be produced");

        assert!(matches!(token.kind, TokenKind::Literal(LiteralKind::Int32)));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 1);
        assert_eq!(lexer.cursor.pos(), 1);
        assert_eq!(lexer.cursor.peek(), Some('+'));
    }

    #[test]
    fn lex_number_parses_multiple_digits_and_stops_before_non_digit() {
        let mut lexer = Lexer::new("12345abc");

        let token = lexer.lex_number().expect("token should be produced");

        assert!(matches!(token.kind, TokenKind::Literal(LiteralKind::Int32)));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 5);
        assert_eq!(lexer.cursor.pos(), 5);
        assert_eq!(lexer.cursor.peek(), Some('a'));
    }

    #[test]
    fn lex_number_returns_none_on_non_digit_start() {
        let mut lexer = Lexer::new("abc");

        let token = lexer.lex_number();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert_eq!(lexer.cursor.peek(), Some('a'));
    }

    #[test]
    fn lex_number_returns_none_on_empty_input() {
        let mut lexer = Lexer::new("");

        let token = lexer.lex_number();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert!(lexer.cursor.is_eof());
    }
}

#[cfg(test)]
mod lexer_operator_or_assign_tests {
    use super::super::Lexer;
    use super::super::token::{OperatorKind, TokenKind};

    #[test]
    fn lex_operator_or_assign_parses_single_char_operators() {
        let cases = [
            ("+", OperatorKind::Plus),
            ("-", OperatorKind::Minus),
            ("*", OperatorKind::Star),
            ("/", OperatorKind::Slash),
            (">", OperatorKind::Gt),
            ("<", OperatorKind::Lt),
            ("&", OperatorKind::Amp),
        ];

        for (src, kind) in cases {
            let mut lexer = Lexer::new(src);
            let token = lexer
                .lex_operator_or_assign()
                .expect("token should be produced");
            assert_eq!(token.span.start, 0);
            assert_eq!(token.span.end, 1);
            match kind {
                OperatorKind::Plus => assert!(matches!(
                    token.kind,
                    TokenKind::Operator(OperatorKind::Plus)
                )),
                OperatorKind::Minus => assert!(matches!(
                    token.kind,
                    TokenKind::Operator(OperatorKind::Minus)
                )),
                OperatorKind::Star => assert!(matches!(
                    token.kind,
                    TokenKind::Operator(OperatorKind::Star)
                )),
                OperatorKind::Slash => assert!(matches!(
                    token.kind,
                    TokenKind::Operator(OperatorKind::Slash)
                )),
                OperatorKind::Gt => {
                    assert!(matches!(token.kind, TokenKind::Operator(OperatorKind::Gt)))
                }
                OperatorKind::Lt => {
                    assert!(matches!(token.kind, TokenKind::Operator(OperatorKind::Lt)))
                }
                OperatorKind::Amp => {
                    assert!(matches!(token.kind, TokenKind::Operator(OperatorKind::Amp)))
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn lex_operator_or_assign_parses_double_char_operators() {
        let cases = [
            ("==", OperatorKind::EqEq),
            (">=", OperatorKind::Ge),
            ("<=", OperatorKind::Le),
            ("!=", OperatorKind::Ne),
        ];

        for (src, kind) in cases {
            let mut lexer = Lexer::new(src);
            let token = lexer
                .lex_operator_or_assign()
                .expect("token should be produced");
            assert_eq!(token.span.start, 0);
            assert_eq!(token.span.end, 2);
            match kind {
                OperatorKind::EqEq => assert!(matches!(
                    token.kind,
                    TokenKind::Operator(OperatorKind::EqEq)
                )),
                OperatorKind::Ge => {
                    assert!(matches!(token.kind, TokenKind::Operator(OperatorKind::Ge)))
                }
                OperatorKind::Le => {
                    assert!(matches!(token.kind, TokenKind::Operator(OperatorKind::Le)))
                }
                OperatorKind::Ne => {
                    assert!(matches!(token.kind, TokenKind::Operator(OperatorKind::Ne)))
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn lex_operator_or_assign_parses_assign() {
        let mut lexer = Lexer::new("=");

        let token = lexer
            .lex_operator_or_assign()
            .expect("token should be produced");

        assert!(matches!(token.kind, TokenKind::Assign));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 1);
    }

    #[test]
    fn lex_operator_or_assign_returns_none_on_non_operator_start() {
        let mut lexer = Lexer::new("a+");

        let token = lexer.lex_operator_or_assign();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert_eq!(lexer.cursor.peek(), Some('a'));
    }
}

#[cfg(test)]
mod lexer_delimiter_tests {
    use super::super::Lexer;
    use super::super::token::{DelimiterKind, TokenKind};

    #[test]
    fn lex_delimiter_parses_all_delimiters() {
        let cases = [
            ("(", DelimiterKind::LParen),
            (")", DelimiterKind::RParen),
            ("{", DelimiterKind::LBrace),
            ("}", DelimiterKind::RBrace),
            ("[", DelimiterKind::LBracket),
            ("]", DelimiterKind::RBracket),
        ];

        for (src, kind) in cases {
            let mut lexer = Lexer::new(src);
            let token = lexer.lex_delimiter().expect("token should be produced");
            assert_eq!(token.span.start, 0);
            assert_eq!(token.span.end, 1);
            match kind {
                DelimiterKind::LParen => assert!(matches!(
                    token.kind,
                    TokenKind::Delimiter(DelimiterKind::LParen)
                )),
                DelimiterKind::RParen => assert!(matches!(
                    token.kind,
                    TokenKind::Delimiter(DelimiterKind::RParen)
                )),
                DelimiterKind::LBrace => assert!(matches!(
                    token.kind,
                    TokenKind::Delimiter(DelimiterKind::LBrace)
                )),
                DelimiterKind::RBrace => assert!(matches!(
                    token.kind,
                    TokenKind::Delimiter(DelimiterKind::RBrace)
                )),
                DelimiterKind::LBracket => assert!(matches!(
                    token.kind,
                    TokenKind::Delimiter(DelimiterKind::LBracket)
                )),
                DelimiterKind::RBracket => assert!(matches!(
                    token.kind,
                    TokenKind::Delimiter(DelimiterKind::RBracket)
                )),
            }
        }
    }

    #[test]
    fn lex_delimiter_returns_none_on_non_delimiter() {
        let mut lexer = Lexer::new("a(");

        let token = lexer.lex_delimiter();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert_eq!(lexer.cursor.peek(), Some('a'));
    }
}

#[cfg(test)]
mod lexer_separator_tests {
    use super::super::Lexer;
    use super::super::token::{SeparatorKind, TokenKind};

    #[test]
    fn lex_separator_parses_all_separators() {
        let cases = [
            (";", SeparatorKind::Semicolon),
            (":", SeparatorKind::Colon),
            (",", SeparatorKind::Comma),
        ];

        for (src, kind) in cases {
            let mut lexer = Lexer::new(src);
            let token = lexer.lex_separator().expect("token should be produced");
            assert_eq!(token.span.start, 0);
            assert_eq!(token.span.end, 1);
            match kind {
                SeparatorKind::Semicolon => assert!(matches!(
                    token.kind,
                    TokenKind::Separator(SeparatorKind::Semicolon)
                )),
                SeparatorKind::Colon => assert!(matches!(
                    token.kind,
                    TokenKind::Separator(SeparatorKind::Colon)
                )),
                SeparatorKind::Comma => assert!(matches!(
                    token.kind,
                    TokenKind::Separator(SeparatorKind::Comma)
                )),
            }
        }
    }

    #[test]
    fn lex_separator_returns_none_on_non_separator() {
        let mut lexer = Lexer::new(".+");

        let token = lexer.lex_separator();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert_eq!(lexer.cursor.peek(), Some('.'));
    }
}

#[cfg(test)]
mod lexer_special_tests {
    use super::super::Lexer;
    use super::super::token::{SpecialKind, TokenKind};

    #[test]
    fn lex_special_parses_arrow_dot_and_dotdot() {
        let mut lexer_arrow = Lexer::new("->x");
        let token_arrow = lexer_arrow.lex_special().expect("token should be produced");
        assert!(matches!(
            token_arrow.kind,
            TokenKind::Special(SpecialKind::Arrow)
        ));
        assert_eq!(token_arrow.span.start, 0);
        assert_eq!(token_arrow.span.end, 2);
        assert_eq!(lexer_arrow.cursor.peek(), Some('x'));

        let mut lexer_dot = Lexer::new(".x");
        let token_dot = lexer_dot.lex_special().expect("token should be produced");
        assert!(matches!(
            token_dot.kind,
            TokenKind::Special(SpecialKind::Dot)
        ));
        assert_eq!(token_dot.span.start, 0);
        assert_eq!(token_dot.span.end, 1);
        assert_eq!(lexer_dot.cursor.peek(), Some('x'));

        let mut lexer_dotdot = Lexer::new("..x");
        let token_dotdot = lexer_dotdot
            .lex_special()
            .expect("token should be produced");
        assert!(matches!(
            token_dotdot.kind,
            TokenKind::Special(SpecialKind::DotDot)
        ));
        assert_eq!(token_dotdot.span.start, 0);
        assert_eq!(token_dotdot.span.end, 2);
        assert_eq!(lexer_dotdot.cursor.peek(), Some('x'));
    }

    #[test]
    fn lex_special_prefers_longest_match_for_ellipsis() {
        let mut lexer = Lexer::new("...");

        let token = lexer.lex_special().expect("token should be produced");

        assert!(matches!(
            token.kind,
            TokenKind::Special(SpecialKind::Ellipsis)
        ));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 3);
        assert_eq!(lexer.cursor.pos(), 3);
        assert_eq!(lexer.cursor.peek(), None);
    }

    #[test]
    fn lex_special_returns_none_on_non_special_start() {
        let mut lexer = Lexer::new("a->");

        let token = lexer.lex_special();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert_eq!(lexer.cursor.peek(), Some('a'));
    }
}

#[cfg(test)]
mod lexer_trial_parse_behavior_tests {
    use super::super::Lexer;

    #[test]
    fn lex_operator_or_assign_should_not_consume_on_failed_bang_prefix() {
        let mut lexer = Lexer::new("!a");

        let token = lexer.lex_operator_or_assign();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert_eq!(lexer.cursor.peek(), Some('!'));
    }

    #[test]
    fn lex_special_should_not_consume_on_failed_minus_prefix() {
        let mut lexer = Lexer::new("-a");

        let token = lexer.lex_special();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert_eq!(lexer.cursor.peek(), Some('-'));
    }
}

#[cfg(test)]
mod lexer_branch_coverage_tests {
    use super::super::Lexer;
    use super::super::token::{OperatorKind, SpecialKind, TokenKind};

    #[test]
    fn lex_operator_or_assign_returns_none_on_empty_input_branch() {
        let mut lexer = Lexer::new("");

        let token = lexer.lex_operator_or_assign();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert!(lexer.cursor.is_eof());
    }

    #[test]
    fn lex_operator_or_assign_falls_back_to_single_char_on_invalid_two_char_text() {
        let mut lexer = Lexer::new("+=rest");

        let token = lexer
            .lex_operator_or_assign()
            .expect("token should be produced");

        assert!(matches!(
            token.kind,
            TokenKind::Operator(OperatorKind::Plus)
        ));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 1);
        assert_eq!(lexer.cursor.pos(), 1);
        assert_eq!(lexer.cursor.peek(), Some('='));
    }

    #[test]
    fn lex_delimiter_returns_none_on_empty_input_branch() {
        let mut lexer = Lexer::new("");

        let token = lexer.lex_delimiter();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert!(lexer.cursor.is_eof());
    }

    #[test]
    fn lex_separator_returns_none_on_empty_input_branch() {
        let mut lexer = Lexer::new("");

        let token = lexer.lex_separator();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert!(lexer.cursor.is_eof());
    }

    #[test]
    fn lex_special_returns_none_on_empty_input_branch() {
        let mut lexer = Lexer::new("");

        let token = lexer.lex_special();

        assert!(token.is_none());
        assert_eq!(lexer.cursor.pos(), 0);
        assert!(lexer.cursor.is_eof());
    }

    #[test]
    fn lex_special_falls_back_to_dot_on_invalid_two_char_text() {
        let mut lexer = Lexer::new(".>x");

        let token = lexer.lex_special().expect("token should be produced");

        assert!(matches!(token.kind, TokenKind::Special(SpecialKind::Dot)));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 1);
        assert_eq!(lexer.cursor.pos(), 1);
        assert_eq!(lexer.cursor.peek(), Some('>'));
    }
}

#[cfg(test)]
mod lexer_trivia_tests {
    use super::super::Lexer;
    use super::super::error::LexErrorKind;
    use super::super::token::{KeywordKind, OperatorKind, TokenKind};

    #[test]
    fn skip_comment_skips_line_comment() {
        let mut lexer = Lexer::new("// hello world\nlet");

        assert!(lexer.skip_comment().expect("line comment should lex"));
        assert_eq!(lexer.cursor.peek(), Some('\n'));
    }

    #[test]
    fn skip_comment_skips_block_comment() {
        let mut lexer = Lexer::new("/* hello world */let");

        assert!(lexer.skip_comment().expect("block comment should lex"));
        assert_eq!(lexer.cursor.peek(), Some('l'));
    }

    #[test]
    fn skip_comment_skips_nested_block_comment() {
        let mut lexer = Lexer::new("/* outer /* inner */ outer */let");

        assert!(
            lexer
                .skip_comment()
                .expect("nested block comment should lex")
        );
        assert_eq!(lexer.cursor.peek(), Some('l'));
    }

    #[test]
    fn skip_comment_returns_error_for_unterminated_block_comment() {
        let mut lexer = Lexer::new("/* unterminated");

        let err = lexer
            .skip_comment()
            .expect_err("unterminated block comment should error");

        assert_eq!(err.kind, LexErrorKind::UnterminatedBlockComment);
        assert_eq!(err.span.start, 0);
        assert_eq!(err.span.end, "/* unterminated".len());
    }

    #[test]
    fn skip_comment_returns_error_for_unterminated_nested_block_comment() {
        let mut lexer = Lexer::new("/* outer /* inner */");

        let err = lexer
            .skip_comment()
            .expect_err("unterminated nested block comment should error");

        assert_eq!(err.kind, LexErrorKind::UnterminatedBlockComment);
        assert_eq!(err.span.start, 0);
        assert_eq!(err.span.end, "/* outer /* inner */".len());
    }

    #[test]
    fn skip_comment_does_not_consume_slash_operator() {
        let mut lexer = Lexer::new("/value");

        assert!(
            !lexer
                .skip_comment()
                .expect("slash operator is not a comment")
        );
        assert_eq!(lexer.cursor.pos(), 0);
        assert_eq!(lexer.cursor.peek(), Some('/'));
    }

    #[test]
    fn skip_trivia_skips_whitespace_and_comments() {
        let mut lexer = Lexer::new(" \n// comment\n/* block */let");

        lexer.skip_trivia().expect("trivia should lex");

        assert_eq!(lexer.cursor.peek(), Some('l'));
        assert_eq!(lexer.cursor.pos(), " \n// comment\n/* block */".len());
    }

    #[test]
    fn next_token_skips_line_comment_then_lexes_keyword() {
        let mut lexer = Lexer::new("// comment\nlet");

        let token = lexer.next_token().expect("token should lex");

        assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Let)));
    }

    #[test]
    fn next_token_skips_block_comment_then_lexes_keyword() {
        let mut lexer = Lexer::new("/* comment */return");

        let token = lexer.next_token().expect("token should lex");

        assert!(matches!(
            token.kind,
            TokenKind::Keyword(KeywordKind::Return)
        ));
    }

    #[test]
    fn next_token_returns_error_for_unterminated_block_comment() {
        let mut lexer = Lexer::new("/* comment");

        let err = lexer
            .next_token()
            .expect_err("unterminated block comment should error");

        assert_eq!(err.kind, LexErrorKind::UnterminatedBlockComment);
        assert_eq!(err.span.start, 0);
        assert_eq!(err.span.end, "/* comment".len());
    }

    #[test]
    fn next_token_treats_plain_slash_as_operator() {
        let mut lexer = Lexer::new("/value");

        let token = lexer.next_token().expect("token should lex");

        assert!(matches!(
            token.kind,
            TokenKind::Operator(OperatorKind::Slash)
        ));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 1);
    }
}

#[cfg(test)]
mod lexer_next_token_tests {
    use super::super::Lexer;
    use super::super::error::LexErrorKind;
    use super::super::token::{KeywordKind, LiteralKind, OperatorKind, Token, TokenKind};

    fn next_token(lexer: &mut Lexer) -> Token {
        lexer.next_token().expect("token should lex")
    }

    #[test]
    fn next_token_returns_eof_on_empty_input() {
        let mut lexer = Lexer::new("");

        let token = next_token(&mut lexer);

        assert!(matches!(token.kind, TokenKind::Eof));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 0);
    }

    #[test]
    fn next_token_skips_whitespace_then_lexes_keyword() {
        let mut lexer = Lexer::new(" \n\tlet");

        let token = next_token(&mut lexer);

        assert!(matches!(token.kind, TokenKind::Keyword(KeywordKind::Let)));
        assert_eq!(token.span.start, 3);
        assert_eq!(token.span.end, 6);
    }

    #[test]
    fn next_token_returns_eof_after_only_whitespace() {
        let mut lexer = Lexer::new(" \n\t  ");

        let token = next_token(&mut lexer);

        assert!(matches!(token.kind, TokenKind::Eof));
        assert_eq!(token.span.start, 5);
        assert_eq!(token.span.end, 5);
    }

    #[test]
    fn next_token_lexes_identifier() {
        let mut lexer = Lexer::new("name_1+");

        let token = next_token(&mut lexer);

        assert!(matches!(token.kind, TokenKind::Ident));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 6);
    }

    #[test]
    fn next_token_lexes_int_literal() {
        let mut lexer = Lexer::new("1234;");

        let token = next_token(&mut lexer);

        assert!(matches!(token.kind, TokenKind::Literal(LiteralKind::Int32)));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 4);
    }

    #[test]
    fn next_token_lexes_string_literal_with_escapes() {
        let mut lexer = Lexer::new("\"hello\\n\\\"world\\\"\"");

        let token = next_token(&mut lexer);

        assert!(matches!(
            token.kind,
            TokenKind::Literal(LiteralKind::String)
        ));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, "\"hello\\n\\\"world\\\"\"".len());
    }

    #[test]
    fn next_token_lexes_assign_and_operator() {
        let mut assign_lexer = Lexer::new("=");
        let assign = next_token(&mut assign_lexer);
        assert!(matches!(assign.kind, TokenKind::Assign));

        let mut op_lexer = Lexer::new("==");
        let op = next_token(&mut op_lexer);
        assert!(matches!(op.kind, TokenKind::Operator(OperatorKind::EqEq)));
    }

    #[test]
    fn next_token_lexes_delimiter_and_separator() {
        let mut delimiter_lexer = Lexer::new("(");
        let delimiter = next_token(&mut delimiter_lexer);
        assert!(matches!(delimiter.kind, TokenKind::Delimiter(_)));

        let mut separator_lexer = Lexer::new(";");
        let separator = next_token(&mut separator_lexer);
        assert!(matches!(separator.kind, TokenKind::Separator(_)));
    }

    #[test]
    fn next_token_returns_error_on_unknown_char() {
        let mut lexer = Lexer::new("@");

        let err = lexer
            .next_token()
            .expect_err("unknown character should produce a lex error");

        assert_eq!(err.kind, LexErrorKind::UnknownCharacter('@'));
        assert_eq!(err.span.start, 0);
        assert_eq!(err.span.end, 1);

        let eof = next_token(&mut lexer);
        assert!(matches!(eof.kind, TokenKind::Eof));
        assert_eq!(eof.span.start, 1);
        assert_eq!(eof.span.end, 1);
    }

    #[test]
    fn iterator_collect_terminates_after_unknown_char() {
        let results: Vec<_> = Lexer::new("@").collect();

        assert_eq!(results.len(), 1);
        assert!(matches!(
            &results[0],
            Err(err) if err.kind == LexErrorKind::UnknownCharacter('@')
                && err.span.start == 0
                && err.span.end == 1
        ));
    }

    #[test]
    fn iterator_collect_terminates_for_error1_like_input() {
        let results: Vec<_> = Lexer::new("let x = @;").collect();

        assert_eq!(results.len(), 5);
        assert!(
            matches!(&results[0], Ok(token) if matches!(&token.kind, TokenKind::Keyword(KeywordKind::Let)))
        );
        assert!(matches!(
            &results[1],
            Ok(token) if matches!(&token.kind, TokenKind::Ident)
        ));
        assert!(matches!(
            &results[2],
            Ok(token) if matches!(&token.kind, TokenKind::Assign)
        ));
        assert!(matches!(
            &results[3],
            Err(err) if err.kind == LexErrorKind::UnknownCharacter('@')
                && err.span.start == 8
                && err.span.end == 9
        ));
        assert!(matches!(
            &results[4],
            Ok(token) if matches!(&token.kind, TokenKind::Separator(_))
        ));
    }

    #[test]
    fn next_token_prefers_special_arrow_over_minus() {
        let mut lexer = Lexer::new("->");

        let token = next_token(&mut lexer);

        assert!(matches!(token.kind, TokenKind::Special(_)));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 2);
    }

    #[test]
    fn next_token_minus_falls_back_to_operator() {
        let mut lexer = Lexer::new("-x");

        let token = next_token(&mut lexer);

        assert!(matches!(
            token.kind,
            TokenKind::Operator(OperatorKind::Minus)
        ));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 1);
    }

    #[test]
    fn next_token_prefers_ellipsis_longest_match() {
        let mut lexer = Lexer::new("...x");

        let first = next_token(&mut lexer);
        let second = next_token(&mut lexer);

        assert!(matches!(
            first.kind,
            TokenKind::Special(super::super::token::SpecialKind::Ellipsis)
        ));
        assert_eq!(first.span.start, 0);
        assert_eq!(first.span.end, 3);
        assert!(matches!(second.kind, TokenKind::Ident));
        assert_eq!(second.span.start, 3);
        assert_eq!(second.span.end, 4);
    }

    #[test]
    fn next_token_prefers_ellipsis_over_dotdot() {
        let mut lexer = Lexer::new("...");

        let token = next_token(&mut lexer);

        assert!(matches!(
            token.kind,
            TokenKind::Special(super::super::token::SpecialKind::Ellipsis)
        ));
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 3);
    }

    #[test]
    fn next_token_reports_unterminated_string_literal() {
        let mut lexer = Lexer::new("\"hello");

        let err = lexer
            .next_token()
            .expect_err("unterminated string literal should error");

        assert_eq!(err.kind, LexErrorKind::UnterminatedStringLiteral);
        assert_eq!(err.span.start, 0);
        assert_eq!(err.span.end, "\"hello".len());
    }

    #[test]
    fn next_token_reports_invalid_string_escape() {
        let mut lexer = Lexer::new("\"bad\\x\"");

        let err = lexer
            .next_token()
            .expect_err("invalid string escape should error");

        assert_eq!(err.kind, LexErrorKind::InvalidStringEscape('x'));
        assert_eq!(err.span.start, 4);
        assert_eq!(err.span.end, 6);
    }

    #[test]
    fn next_token_program_like_sequence_matches_expected_kinds() {
        let mut lexer = Lexer::new("fn main(){ let x = 42; if x>=1 { x=x-1; } }\n");

        let mut kinds = Vec::new();
        for _ in 0..32 {
            let token = next_token(&mut lexer);
            let done = matches!(&token.kind, TokenKind::Eof);
            kinds.push(token.kind);
            if done {
                break;
            }
        }

        assert!(matches!(kinds[0], TokenKind::Keyword(KeywordKind::Fn)));
        assert!(matches!(kinds[1], TokenKind::Ident));
        assert!(matches!(kinds[2], TokenKind::Delimiter(_)));
        assert!(matches!(kinds[3], TokenKind::Delimiter(_)));
        assert!(matches!(kinds[4], TokenKind::Delimiter(_)));
        assert!(matches!(kinds[5], TokenKind::Keyword(KeywordKind::Let)));
        assert!(matches!(kinds[6], TokenKind::Ident));
        assert!(matches!(kinds[7], TokenKind::Assign));
        assert!(matches!(kinds[8], TokenKind::Literal(LiteralKind::Int32)));
        assert!(matches!(kinds[9], TokenKind::Separator(_)));
        assert!(matches!(kinds[10], TokenKind::Keyword(KeywordKind::If)));
        assert!(matches!(kinds[11], TokenKind::Ident));
        assert!(matches!(kinds[12], TokenKind::Operator(OperatorKind::Ge)));
        assert!(matches!(kinds[13], TokenKind::Literal(LiteralKind::Int32)));
        assert!(matches!(kinds[14], TokenKind::Delimiter(_)));
        assert!(matches!(kinds[15], TokenKind::Ident));
        assert!(matches!(kinds[16], TokenKind::Assign));
        assert!(matches!(kinds[17], TokenKind::Ident));
        assert!(matches!(
            kinds[18],
            TokenKind::Operator(OperatorKind::Minus)
        ));
        assert!(matches!(kinds[19], TokenKind::Literal(LiteralKind::Int32)));
        assert!(matches!(kinds[20], TokenKind::Separator(_)));
        assert!(matches!(kinds[21], TokenKind::Delimiter(_)));
        assert!(matches!(kinds[22], TokenKind::Delimiter(_)));
        assert!(matches!(kinds[23], TokenKind::Eof));
    }
}
