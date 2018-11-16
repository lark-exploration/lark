use crate::lexer::token::LexToken;
use crate::lexer::tools::consume;
use crate::lexer::tools::consume_str;
use crate::lexer::tools::reconsume;
use crate::lexer::tools::LexerDelegateTrait;
use crate::lexer::tools::LexerNext;

use unicode_xid::UnicodeXID;

#[derive(Debug, Copy, Clone)]
crate enum LexerState {
    Top,
    Whitespace,
    StartIdent,
    ContinueIdent,
    StringLiteral,
    Sigil,
    Number,
    Comment(u32),
}

impl LexerDelegateTrait for LexerState {
    type Token = LexToken;

    fn top() -> LexerState {
        LexerState::Top
    }

    fn next<'input>(&self, c: Option<char>, rest: &'input str) -> LexerNext<Self> {
        use self::LexerState::*;

        let out = match self {
            LexerState::Top => match c {
                None => LexerNext::EOF,
                Some(c) => match c {
                    c if UnicodeXID::is_xid_start(c) => LexerNext::begin(StartIdent),
                    c if is_delimiter_sigil_char(c) => {
                        consume(c).and_emit(LexToken::Sigil).and_remain()
                    }
                    c if is_sigil_char(c) => {
                        LexerNext::begin(Sigil)
                        // LexerNext::dynamic_sigil(Token::Sigil)
                    }
                    '0'..='9' => LexerNext::begin(Number),
                    '"' => consume(c).and_transition(StringLiteral),
                    '\n' => LexerNext::sigil(LexToken::Newline),
                    c if c.is_whitespace() => LexerNext::begin(Whitespace),
                    _ if rest.starts_with("/*") => consume_str("/*").and_push(Comment(1)),
                    _ => consume(c).and_emit(LexToken::Error).and_remain(),
                },
            },

            LexerState::Sigil => match c {
                None => reconsume()
                    .and_emit(LexToken::Sigil)
                    .and_transition(LexerState::Top),
                Some(c) if is_sigil_char(c) => consume(c).and_remain(),
                _ => reconsume()
                    .and_emit(LexToken::Sigil)
                    .and_transition(LexerState::Top),
            },

            LexerState::Number => match c {
                None => reconsume()
                    .and_emit(LexToken::Integer)
                    .and_transition(LexerState::Top),
                Some(c @ '0'..='9') => consume(c).and_remain(),
                Some(c @ '_') => consume(c).and_remain(),
                Some(_) => reconsume()
                    .and_emit(LexToken::Integer)
                    .and_transition(LexerState::Top),
            },

            LexerState::StringLiteral => match c {
                None => reconsume()
                    .and_emit(LexToken::Error)
                    .and_transition(LexerState::Top),
                Some(c) => match c {
                    '"' => consume(c)
                        .and_emit(LexToken::String)
                        .and_transition(LexerState::Top),
                    _ => consume(c).and_remain(),
                },
            },

            LexerState::StartIdent => match c {
                None => LexerNext::emit(LexToken::Identifier, LexerState::Top),
                Some(c) => match c {
                    c if UnicodeXID::is_xid_continue(c) => {
                        consume(c).and_transition(LexerState::ContinueIdent)
                    }

                    // TODO: Should this be a pop, so we don't have to reiterate
                    // the state name?
                    _ => reconsume()
                        .and_emit(LexToken::Identifier)
                        .and_transition(LexerState::Top),
                },
            },

            LexerState::ContinueIdent => match c {
                None => LexerNext::emit(LexToken::Identifier, LexerState::Top),
                Some(c) => match c {
                    c if UnicodeXID::is_xid_continue(c) => consume(c).and_remain(),
                    _ => reconsume()
                        .and_emit(LexToken::Identifier)
                        .and_transition(LexerState::Top),
                },
            },

            LexerState::Whitespace => match c {
                None => LexerNext::EOF,
                Some(c) => match c {
                    '\n' => reconsume().and_discard().and_transition(LexerState::Top),
                    c if c.is_whitespace() => consume(c).and_remain(),
                    _ => reconsume()
                        .and_emit(LexToken::Whitespace)
                        .and_transition(LexerState::Top),
                },
            },

            LexerState::Comment(1) => {
                if rest.starts_with("/*") {
                    consume_str("/*")
                        .and_continue()
                        .and_transition(LexerState::Comment(2))
                } else if rest.starts_with("*/") {
                    consume_str("*/")
                        .and_emit(LexToken::Comment)
                        .and_transition(LexerState::Top)
                } else {
                    match c {
                        Some(c) => consume(c).and_remain(),
                        None => reconsume()
                            .and_emit(LexToken::Error)
                            .and_transition(LexerState::Top),
                    }
                }
            }

            LexerState::Comment(n) => {
                if rest.starts_with("/*") {
                    consume_str("/*")
                        .and_continue()
                        .and_transition(LexerState::Comment(n + 1))
                } else if rest.starts_with("*/") {
                    consume_str("*/")
                        .and_continue()
                        .and_transition(LexerState::Comment(n - 1))
                } else {
                    match c {
                        Some(c) => consume(c).and_remain(),
                        None => reconsume()
                            .and_emit(LexToken::Error)
                            .and_transition(LexerState::Comment(n - 1)),
                    }
                }
            }
        };

        out
    }
}

fn is_sigil_char(c: char) -> bool {
    match c {
        '{' | '}' | '(' | ')' | '+' | '-' | '*' | '/' | ':' | ',' | '>' | '<' | '=' | '.' => true,
        _ => false,
    }
}

fn is_delimiter_sigil_char(c: char) -> bool {
    match c {
        '{' | '}' | '(' | ')' | '>' | '<' => true,
        _ => false,
    }
}
