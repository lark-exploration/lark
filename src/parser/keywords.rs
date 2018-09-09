use crate::parser::Token;
use lazy_static::lazy_static;

lazy_static! {
    pub(crate) static ref KEYWORDS: Matchers =
        { Matchers::keywords(&[("struct", Token::KeywordStruct), ("own", Token::KeywordOwn)]) };
    pub(crate) static ref SIGILS: Matchers = {
        Matchers::keywords(&[
            ("{", Token::CurlyBraceOpen),
            ("}", Token::CurlyBraceClose),
            (":", Token::Colon),
            (",", Token::Comma),
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
