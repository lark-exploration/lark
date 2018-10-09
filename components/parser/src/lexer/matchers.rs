#![allow(unused_variables)]

use derive_new::new;
use std::fmt;

pub struct TokenMatches<Token> {
    tokens: Vec<(&'static str, Token, u32)>,
}

impl<Token: Copy> TokenMatches<Token> {
    pub fn new(strings: Vec<(&'static str, Token)>) -> TokenMatches<Token> {
        let tokens = strings
            .iter()
            .map(|(s, t)| (*s, *t, s.len() as u32))
            .collect();
        TokenMatches { tokens }
    }

    pub fn match_token(&self, rest: &str) -> Option<(Token, u32)> {
        for (string, token, len) in &self.tokens {
            if rest.starts_with(string) {
                return Some((*token, *len));
            }
        }

        None
    }
}

#[derive(Debug, new)]
pub struct KeywordList {
    vec: Vec<String>,
}

impl fmt::Display for KeywordList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entries(self.vec.iter().map(|i| match i {
                s if s == "\"if\"" => DisplayAdapter::new("if"),
                s if s == "\"else\"" => DisplayAdapter::new("else"),
                s if s == "\"for\"" => DisplayAdapter::new("for"),
                other => DisplayAdapter::new(&i[..]),
            }))
            .finish()
    }
}

#[derive(new)]
pub struct DisplayAdapter<T: fmt::Display> {
    inner: T,
}

impl<T: fmt::Display> fmt::Debug for DisplayAdapter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.inner)
    }
}

#[macro_export]
macro_rules! tokens {
    {
        type Token = $token:ident;

        $(
        declare($($v:tt)*) $name:ident {
            $($str:tt => $id:ident),*
        }
        )*
    } => {
        lazy_static::lazy_static! {
            $(
                $($v)* static ref $name: $crate::lexer::TokenMatches<$token> = {
                    $crate::lexer::TokenMatches::new(vec![
                        $(
                            ( $str, $token::$id )
                        ),*
                    ])
                };
            )*
        }
    };
}
