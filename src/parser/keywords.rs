use crate::lexer::TokenMatches;
use crate::parser::Token;

use lazy_static::lazy_static;
use std::fmt;

use derive_new::new;

tokens! {
    type Token = Token;

    declare KEYWORDS {
        "struct" => KeywordStruct,
        "own" => KeywordOwn,
        "def" => KeywordDef
    }

    declare SIGILS {
        "{" => CurlyBraceOpen,
        "}" => CurlyBraceClose,
        "(" => ParenOpen,
        ":" => Colon,
        ")" => ParenClose,
        ":" => Colon,
        "," => Comma,
        "->"=> ThinArrow
    }
}
