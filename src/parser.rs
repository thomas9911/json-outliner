use std::collections::HashMap;

use crate::{
    lexer::{Lexer, Token, TokenKind},
    value::ValueRef,
};

#[derive(Debug, PartialEq)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub fn invalid_token() -> Error {
        Error {
            kind: ErrorKind::InvalidToken,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    Lexer,
    InvalidToken,
    InvalidInteger,
    InvalidBoolean,
    InvalidNumber,
    DoubleSeparators,
    None,
}

#[derive(Debug)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    pub fn from_str(text: &'a str) -> Self {
        Parser {
            lexer: Lexer::new(text),
        }
    }

    pub fn from_lexer(lexer: Lexer<'a>) -> Self {
        Parser { lexer }
    }

    pub fn to_value(&mut self) -> Result<ValueRef<'a>, Error> {
        if self.lexer.is_error {
            return Err(Error {
                kind: ErrorKind::Lexer,
            });
        }

        self.to_value_inner(None)
    }

    fn to_value_inner(&mut self, prev_token: Option<Token<'a>>) -> Result<ValueRef<'a>, Error> {
        let mut item = None;

        if let Some(token) = prev_token {
            self.inner_value_loop(&token, &mut item)?;
            if let Some(item) = item {
                return Ok(item);
            }
        }

        while let Some(token) = self.lexer.next() {
            self.inner_value_loop(&token, &mut item)?;
            // if let Some(item) = item {
            //     return Ok(item);
            // }
        }

        if let Some(item) = item {
            Ok(item)
        } else {
            Err(Error {
                kind: ErrorKind::None,
            })
        }
    }

    fn inner_value_loop(
        &mut self,
        token: &Token<'a>,
        item: &mut Option<ValueRef<'a>>,
    ) -> Result<(), Error> {
        match token.kind {
            TokenKind::TabSpacing | TokenKind::NewLine | TokenKind::Spacing => {}
            TokenKind::StartMapping => *item = Some(self.value_mapping()?),
            TokenKind::EndMapping => return Err(Error::invalid_token()),
            TokenKind::StartArray => *item = Some(self.value_array()?),
            TokenKind::EndArray => return Err(Error::invalid_token()),
            TokenKind::Separator => return Err(Error::invalid_token()),
            TokenKind::KeySeparator => return Err(Error::invalid_token()),
            TokenKind::String => *item = Some(Self::value_string(&token)?),
            TokenKind::Integer => *item = Some(Self::value_integer(&token)?),
            TokenKind::Boolean => *item = Some(Self::value_boolean(&token)?),
            TokenKind::Float => *item = Some(Self::value_float(&token)?),
            TokenKind::Reference => *item = Some(Self::value_reference(&token)?),
        };

        Ok(())
    }

    fn value_string(token: &Token<'a>) -> Result<ValueRef<'a>, Error> {
        Ok(ValueRef::String(&token.data.trim_matches('"')))
    }

    fn value_reference(token: &Token<'a>) -> Result<ValueRef<'a>, Error> {
        Ok(ValueRef::Reference(&token.data))
    }

    fn value_integer(token: &Token<'a>) -> Result<ValueRef<'a>, Error> {
        token
            .data
            .parse()
            .map(ValueRef::Integer)
            .map_err(|_| Error {
                kind: ErrorKind::InvalidInteger,
            })
    }

    fn value_boolean(token: &Token<'a>) -> Result<ValueRef<'a>, Error> {
        token
            .data
            .parse()
            .map(ValueRef::Boolean)
            .map_err(|_| Error {
                kind: ErrorKind::InvalidBoolean,
            })
    }

    fn value_float(token: &Token<'a>) -> Result<ValueRef<'a>, Error> {
        token.data.parse().map(ValueRef::Number).map_err(|_| Error {
            kind: ErrorKind::InvalidNumber,
        })
    }

    fn value_array(&mut self) -> Result<ValueRef<'a>, Error> {
        let mut array = Vec::new();
        let mut seperator = false;

        loop {
            let item = self.lexer.next();
            match item {
                Some(Token {
                    kind: TokenKind::Separator,
                    ..
                }) if seperator == true => {
                    return Err(Error {
                        kind: ErrorKind::DoubleSeparators,
                    });
                }
                Some(Token {
                    kind: TokenKind::Separator,
                    ..
                }) if seperator == false => {
                    seperator = true;
                }
                Some(token)
                    if (token.is_value(true)
                        || token.kind == TokenKind::StartMapping
                        || token.kind == TokenKind::StartArray) =>
                {
                    let value = self.to_value_inner(Some(token))?;
                    array.push(value);
                    seperator = false;
                }
                Some(Token {
                    kind: TokenKind::EndArray,
                    ..
                }) => {
                    return Ok(ValueRef::Array(array));
                }
                Some(token) if token.is_whitespace() => {}
                _ => return Err(Error::invalid_token()),
            }
        }
    }

    fn value_mapping(&mut self) -> Result<ValueRef<'a>, Error> {
        let mut map = HashMap::new();

        let mut key = None;
        let mut key_seperator = false;
        // let mut seperator = false;

        loop {
            let item = self.lexer.next();
            // dbg!((&item, key, key_seperator));
            match item {
                Some(Token {
                    kind: TokenKind::String,
                    data,
                    ..
                }) => {
                    key = Some(data.trim_matches('"'));
                }
                Some(Token {
                    kind: TokenKind::KeySeparator,
                    ..
                }) if key.is_some() => {
                    key_seperator = true;
                }
                Some(token)
                    if key_seperator == true
                        && (token.is_value(true)
                            || token.kind == TokenKind::StartMapping
                            || token.kind == TokenKind::StartArray)
                        && key.is_some() =>
                {
                    let value = self.to_value_inner(Some(token))?;
                    map.insert(key.unwrap(), value);
                }
                Some(Token {
                    kind: TokenKind::Separator,
                    ..
                }) => {
                    key_seperator = false;
                    key = None;
                }
                Some(Token {
                    kind: TokenKind::EndMapping,
                    ..
                }) => {
                    return Ok(ValueRef::Object(map));
                }
                Some(token) if token.is_whitespace() => {}
                _ => return Err(Error::invalid_token()),
            }
        }
    }
}

#[test]
fn parse_integer() {
    let mut parser = Parser::from_str("1234");

    assert_eq!(parser.to_value(), Ok(ValueRef::Integer(1234)))
}

#[test]
fn parse_simple_map() {
    let mut parser = Parser::from_str(r#"{"a": 1234}"#);
    let expected = HashMap::from_iter(vec![("a", ValueRef::Integer(1234))]);

    assert_eq!(parser.to_value(), Ok(ValueRef::Object(expected)))
}

#[test]
fn parse_simple_array() {
    let mut parser = Parser::from_str(r#"["test", 1, true, false, 912.21]"#);
    let expected = vec![
        ValueRef::String("test"),
        ValueRef::Integer(1),
        ValueRef::Boolean(true),
        ValueRef::Boolean(false),
        ValueRef::Number(912.21),
    ];

    assert_eq!(parser.to_value(), Ok(ValueRef::Array(expected)))
}

#[test]
fn parse_map() {
    let mut parser = Parser::from_str(r#"{"a": 1234, "b": true, "c": {"d": false}}"#);
    let expected = HashMap::from_iter(vec![
        ("a", ValueRef::Integer(1234)),
        ("b", ValueRef::Boolean(true)),
        (
            "c",
            ValueRef::Object(HashMap::from_iter(vec![("d", ValueRef::Boolean(false))])),
        ),
    ]);

    assert_eq!(parser.to_value(), Ok(ValueRef::Object(expected)))
}
