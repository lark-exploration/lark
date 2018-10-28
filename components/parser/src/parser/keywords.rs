use crate::parser::Token;

tokens! {
    type Token = Token;

    declare KEYWORDS {
        "struct" => KeywordStruct,
        "own"    => KeywordOwn,
        "def"    => KeywordDef,
        "let"    => KeywordLet
    }

    declare SIGILS {
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
