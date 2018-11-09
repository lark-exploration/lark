use lark_debug_derive::DebugWith;

/// The different kinds of token our lexer can distinguish. Note that
/// you can recover the full text of the token using its span.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq)]
pub enum LexToken {
    Whitespace,
    Identifier,
    Sigil,
    Comment,
    String,
    Newline,
    EOF,
}
