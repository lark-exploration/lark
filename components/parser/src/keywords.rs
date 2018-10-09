use crate::lexer::TokenMatches;
use crate::Token;

use lazy_static::lazy_static;
use std::fmt;

use derive_new::new;

tokens! {
    type Token = Token;

    declare(pub) KEYWORDS {
        "struct" => KeywordStruct,
        "own"    => KeywordOwn,
        "def"    => KeywordDef,
        "let"    => KeywordLet
    }

    declare(pub) SIGILS {
        "{"  => CurlyBraceOpen,
        "}"  => CurlyBraceClose,
        "("  => ParenOpen,
        ":"  => Colon,
        ")"  => ParenClose,
        ":"  => Colon,
        ","  => Comma,
        "->" => ThinArrow,
        "="  => Equals,
        "+"  => OpAdd,
        "\n" => Newline
    }
}
