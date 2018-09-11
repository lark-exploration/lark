use crate::parser::Token;
use lazy_static::lazy_static;
use std::fmt;

use derive_new::new;

lazy_static! {
    pub(crate) static ref KEYWORDS: Matchers = {
        Matchers::keywords(&[
            ("struct", Token::KeywordStruct),
            ("own", Token::KeywordOwn),
            ("def", Token::KeywordDef),
        ])
    };
    pub(crate) static ref SIGILS: Matchers = {
        Matchers::keywords(&[
            ("{", Token::CurlyBraceOpen),
            ("}", Token::CurlyBraceClose),
            ("(", Token::ParenOpen),
            (":", Token::Colon),
            (")", Token::ParenClose),
            (":", Token::Colon),
            (",", Token::Comma),
            ("->", Token::ThinArrow),
        ])
    };
}

crate struct Matchers {
    keywords: Keywords,
}

impl Matchers {
    crate fn keywords(keywords: &[(&'static str, Token)]) -> Matchers {
        Matchers {
            keywords: Keywords::new(keywords.into()),
        }
    }

    crate fn match_keyword(&self, rest: &str) -> Option<(Token, u32)> {
        self.keywords.match_keyword(rest)
    }
}

crate struct Keywords {
    tokens: Vec<(&'static str, Token, u32)>,
}

impl Keywords {
    crate fn new(strings: Vec<(&'static str, Token)>) -> Keywords {
        let tokens = strings
            .iter()
            .map(|(s, t)| (*s, *t, s.len() as u32))
            .collect();
        Keywords { tokens }
    }

    crate fn match_keyword(&self, rest: &str) -> Option<(Token, u32)> {
        for (string, token, len) in &self.tokens {
            if rest.starts_with(string) {
                return Some((*token, *len));
            }
        }

        None
    }
}

#[derive(Debug, new)]
crate struct KeywordList {
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
            })).finish()
    }
}

#[derive(new)]
crate struct DisplayAdapter<T: fmt::Display> {
    inner: T,
}

impl<T: fmt::Display> fmt::Debug for DisplayAdapter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.inner)
    }
}
