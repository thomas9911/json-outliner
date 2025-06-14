use std::{
    iter::{Enumerate, Peekable},
    str::Chars,
};

enum Either<T, S> {
    Left(T),
    Right(S),
}

fn left<T, S>(t: T) -> Either<T, S> {
    Either::Left(t)
}

fn right<T, S>(s: S) -> Either<T, S> {
    Either::Right(s)
}

#[derive(Debug, PartialEq)]
pub struct Span {
    start: usize,
    length: usize,
}

impl Span {
    pub fn as_range(&self) -> std::ops::Range<usize> {
        self.start..(self.start + self.length)
    }
}

impl Span {
    fn new(start: usize, end: usize) -> Span {
        Span {
            start,
            length: end - start,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TokenKind {
    StartMapping,
    EndMapping,
    StartArray,
    EndArray,
    Separator,
    KeySeparator,
    Spacing,
    TabSpacing,
    NewLine,
    String,
    Integer,
    Boolean,
    Float,
    Reference,
}

#[derive(Debug, PartialEq)]
pub struct Token<'a> {
    kind: TokenKind,
    span: Span,
    data: &'a str,
}

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    text: &'a str,
    chars: Peekable<Enumerate<Chars<'a>>>,
    position: usize,
    in_string: bool,
    string_escaped: bool,
    in_float: bool,
    in_number: bool,
    in_ref: bool,
    is_error: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(text: &'a str) -> Self {
        Lexer {
            text,
            chars: text.chars().enumerate().peekable(),
            position: 0,
            in_string: false,
            string_escaped: false,
            in_float: false,
            in_number: false,
            in_ref: false,
            is_error: false,
        }
    }

    fn new_span(&self, current_index: usize) -> Span {
        Span::new(self.position, current_index + 1)
    }

    fn new_token(&mut self, token_kind: TokenKind, current_index: usize) -> Token<'a> {
        let span = self.new_span(current_index);
        self.new_token_from_span(token_kind, span)
    }

    fn new_token_from_span(&mut self, token_kind: TokenKind, span: Span) -> Token<'a> {
        self.reset_flags();
        Token {
            kind: token_kind,
            data: &self.text[span.as_range()],
            span,
        }
    }

    fn reset_flags(&mut self) {
        self.in_float = false;
        self.in_number = false;
        self.in_string = false;
        self.string_escaped = false;
        self.in_ref = false;
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_error {
            return None;
        }

        while let Some((idx, ch)) = self.chars.next() {
            let mut item: Option<Either<_, (Token<'a>, usize)>> = None;

            match ch {
                '\\' if self.in_string == true => {
                    self.string_escaped = true;
                }
                '"' if self.string_escaped == true => {
                    self.string_escaped = false;
                }
                '"' if self.in_string == true => {
                    self.in_string = false;
                    item = Some(left(self.new_token(TokenKind::String, idx)))
                }
                '"' if self.in_string == false => {
                    self.in_string = true;
                }
                _ if self.in_string == true => {
                    continue;
                }
                // x if "0123456789".contains(x) && self.in_number == false => {
                //     self.in_number = true;
                //     continue;
                // }
                '.' if self.in_float == true => {
                    self.is_error = true;
                    return None;
                }
                '.' => {
                    self.in_float = true;
                }
                // x if "0123456789".contains(x) && self.chars.peek().is_none() => {
                //     let kind = if self.in_float {TokenKind::Float} else {TokenKind::Integer};
                //     item = Some(left(self.new_token(kind, idx)))
                // }
                x if self.in_ref == false
                    && "0123456789".contains(x)
                    && self
                        .chars
                        .peek()
                        .map(|(_, x)| !"e-.0123456789".contains(*x))
                        .unwrap_or(false) =>
                {
                    let kind = if self.in_float {
                        TokenKind::Float
                    } else {
                        TokenKind::Integer
                    };
                    item = Some(left(self.new_token(kind, idx)))
                }
                x if self.in_ref == false && "0123456789".contains(x) => {
                    self.in_number = true;
                }
                'e' if self.in_ref == false && self.in_number == true => {
                    self.in_float = true;
                }
                't' if self.in_ref == false => {
                    let span = self.new_span(self.position + 3);
                    if let Some("true") = self.text.get(span.as_range()) {
                        let (pos, _) = self.chars.nth(2).unwrap();
                        item = Some(right((
                            self.new_token_from_span(TokenKind::Boolean, span),
                            pos,
                        )));
                    }
                }
                'f' if self.in_ref == false => {
                    let span = self.new_span(self.position + 4);
                    if let Some("false") = self.text.get(span.as_range()) {
                        let (pos, _) = self.chars.nth(3).unwrap();
                        item = Some(right((
                            self.new_token_from_span(TokenKind::Boolean, span),
                            pos,
                        )));
                    }
                }
                x if is_snakecase(x)
                    && self
                        .chars
                        .peek()
                        .map(|(_, c)| !is_snakecase(*c))
                        .unwrap_or(false) =>
                {
                    item = Some(left(self.new_token(TokenKind::Reference, idx)))
                }
                x if is_snakecase(x) => {
                    self.in_ref = true;
                }
                '[' => item = Some(left(self.new_token(TokenKind::StartArray, idx))),
                ']' => item = Some(left(self.new_token(TokenKind::EndArray, idx))),
                '{' => item = Some(left(self.new_token(TokenKind::StartMapping, idx))),
                '}' => item = Some(left(self.new_token(TokenKind::EndMapping, idx))),
                ',' => item = Some(left(self.new_token(TokenKind::Separator, idx))),
                ':' => item = Some(left(self.new_token(TokenKind::KeySeparator, idx))),
                ' ' => item = Some(left(self.new_token(TokenKind::Spacing, idx))),
                '\t' => item = Some(left(self.new_token(TokenKind::TabSpacing, idx))),
                '\n' => item = Some(left(self.new_token(TokenKind::NewLine, idx))),

                _ => {}
            }

            match item {
                Some(Either::Left(item)) => {
                    self.position = idx + 1;
                    return Some(item);
                }
                Some(Either::Right((item, pos))) => {
                    self.position = pos + 1;
                    return Some(item);
                }
                None => {}
            }
        }

        None
    }
}

fn is_snakecase(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_string() {
        let text = r#""data \"123\" ""#;
        let mut lexer = Lexer::new(text);
        let token = lexer.next().unwrap();
        assert_eq!(
            token,
            Token {
                kind: TokenKind::String,
                data: r#""data \"123\" ""#,
                span: Span {
                    start: 0,
                    length: 15
                }
            }
        )
    }

    #[test]
    fn lexer_bool() {
        let text = "[true,false]";
        let lexer = Lexer::new(text);
        let tokens: Vec<_> = lexer.clone().collect();
        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::StartArray,
                    data: "[",
                    span: Span {
                        start: 0,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Boolean,
                    data: "true",
                    span: Span {
                        start: 1,
                        length: 4
                    }
                },
                Token {
                    kind: TokenKind::Separator,
                    data: ",",
                    span: Span {
                        start: 5,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Boolean,
                    data: "false",
                    span: Span {
                        start: 6,
                        length: 5
                    }
                },
                Token {
                    kind: TokenKind::EndArray,
                    data: "]",
                    span: Span {
                        start: 11,
                        length: 1
                    }
                }
            ]
        );
    }

    #[test]
    fn lexer_number() {
        let text = "[123456]";
        let lexer = Lexer::new(text);
        let tokens: Vec<_> = lexer.clone().collect();
        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::StartArray,
                    data: "[",
                    span: Span {
                        start: 0,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Integer,
                    data: "123456",
                    span: Span {
                        start: 1,
                        length: 6
                    }
                },
                Token {
                    kind: TokenKind::EndArray,
                    data: "]",
                    span: Span {
                        start: 7,
                        length: 1
                    }
                }
            ]
        );
    }

    #[test]
    fn lexer_simple_float() {
        let text = "[123.456,3e-19,-2]";
        let lexer = Lexer::new(text);
        let tokens: Vec<_> = lexer.clone().collect();
        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::StartArray,
                    data: "[",
                    span: Span {
                        start: 0,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Float,
                    data: "123.456",
                    span: Span {
                        start: 1,
                        length: 7
                    }
                },
                Token {
                    kind: TokenKind::Separator,
                    data: ",",
                    span: Span {
                        start: 8,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Float,
                    data: "3e-19",
                    span: Span {
                        start: 9,
                        length: 5
                    }
                },
                Token {
                    kind: TokenKind::Separator,
                    data: ",",
                    span: Span {
                        start: 14,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Integer,
                    data: "-2",
                    span: Span {
                        start: 15,
                        length: 2
                    }
                },
                Token {
                    kind: TokenKind::EndArray,
                    data: "]",
                    span: Span {
                        start: 17,
                        length: 1
                    }
                }
            ]
        );
    }

    #[test]
    fn lexer_simple_map() {
        let text = r#"{"a": 123.456, "b": "c"}"#;
        let lexer = Lexer::new(text);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::StartMapping,
                    data: "{",
                    span: Span {
                        start: 0,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::String,
                    data: r#""a""#,
                    span: Span {
                        start: 1,
                        length: 3
                    }
                },
                Token {
                    kind: TokenKind::KeySeparator,
                    data: ":",
                    span: Span {
                        start: 4,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Spacing,
                    data: " ",
                    span: Span {
                        start: 5,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Float,
                    data: "123.456",
                    span: Span {
                        start: 6,
                        length: 7
                    }
                },
                Token {
                    kind: TokenKind::Separator,
                    data: ",",
                    span: Span {
                        start: 13,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Spacing,
                    data: " ",
                    span: Span {
                        start: 14,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::String,
                    data: r#""b""#,
                    span: Span {
                        start: 15,
                        length: 3
                    }
                },
                Token {
                    kind: TokenKind::KeySeparator,
                    data: ":",
                    span: Span {
                        start: 18,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Spacing,
                    data: " ",
                    span: Span {
                        start: 19,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::String,
                    data: r#""c""#,
                    span: Span {
                        start: 20,
                        length: 3
                    }
                },
                Token {
                    kind: TokenKind::EndMapping,
                    data: "}",
                    span: Span {
                        start: 23,
                        length: 1
                    }
                }
            ]
        );
    }

    #[test]
    fn lexer_reference() {
        let text = r#"[my_reference_name]"#;
        let lexer = Lexer::new(text);
        let tokens: Vec<_> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::StartArray,
                    data: "[",
                    span: Span {
                        start: 0,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Reference,
                    data: "my_reference_name",
                    span: Span {
                        start: 1,
                        length: 17
                    }
                },
                Token {
                    kind: TokenKind::EndArray,
                    data: "]",
                    span: Span {
                        start: 18,
                        length: 1
                    }
                }
            ]
        );
    }

    #[test]
    fn lexer_reference_ending_in_number() {
        let text = r#"[my_reference_name_12]"#;
        let lexer = Lexer::new(text);
        let tokens: Vec<_> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::StartArray,
                    data: "[",
                    span: Span {
                        start: 0,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Reference,
                    data: "my_reference_name_12",
                    span: Span {
                        start: 1,
                        length: 20
                    }
                },
                Token {
                    kind: TokenKind::EndArray,
                    data: "]",
                    span: Span {
                        start: 21,
                        length: 1
                    }
                }
            ]
        );
    }

    #[test]
    fn lexer_reference_ending_in_boolean() {
        let text = r#"[my_reference_name_true, my_reference_name_false]"#;
        let lexer = Lexer::new(text);
        let tokens: Vec<_> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::StartArray,
                    data: "[",
                    span: Span {
                        start: 0,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Reference,
                    data: "my_reference_name_true",
                    span: Span {
                        start: 1,
                        length: 22
                    }
                },
                Token {
                    kind: TokenKind::Separator,
                    data: ",",
                    span: Span {
                        start: 23,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Spacing,
                    data: " ",
                    span: Span {
                        start: 24,
                        length: 1
                    }
                },
                Token {
                    kind: TokenKind::Reference,
                    data: "my_reference_name_false",
                    span: Span {
                        start: 25,
                        length: 23
                    }
                },
                Token {
                    kind: TokenKind::EndArray,
                    data: "]",
                    span: Span {
                        start: 48,
                        length: 1
                    }
                }
            ]
        );
    }
}
