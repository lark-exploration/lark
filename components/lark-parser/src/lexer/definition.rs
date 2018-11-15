use crate::lexer::token::LexToken;
use crate::lexer::tools::consume;
use crate::lexer::tools::consume_n;
use crate::lexer::tools::reconsume;
use crate::lexer::tools::LexerDelegateTrait;
use crate::lexer::tools::LexerNext;

use lark_span::{CurrentFile, Span};
use unicode_xid::UnicodeXID;

#[derive(Debug, Copy, Clone)]
crate enum LexerState {
    Top,
    Whitespace,
    StartIdent,
    ContinueIdent,
    StringLiteral,
    Sigil,
    Comment(u32),
}

impl LexerDelegateTrait for LexerState {
    type Token = LexToken;

    fn top() -> LexerState {
        LexerState::Top
    }

    fn next<'input>(
        &self,
        c: Option<char>,
        rest: &'input str,
    ) -> Result<LexerNext<Self>, Span<CurrentFile>> {
        use self::LexerState::*;

        let out = match self {
            LexerState::Top => match c {
                None => LexerNext::EOF,
                Some(c) => match c {
                    c if UnicodeXID::is_xid_start(c) => LexerNext::begin(StartIdent),
                    c if is_delimiter_sigil_char(c) => {
                        consume().and_emit(LexToken::Sigil).and_remain()
                    }
                    c if is_sigil_char(c) => {
                        LexerNext::begin(Sigil)
                        // LexerNext::dynamic_sigil(Token::Sigil)
                    }
                    '"' => consume().and_transition(StringLiteral),
                    '\n' => LexerNext::sigil(LexToken::Newline),
                    c if c.is_whitespace() => LexerNext::begin(Whitespace),
                    _ if rest.starts_with("/*") => consume_n(2).and_push(Comment(1)),
                    c => LexerNext::Error(Some(c)),
                },
            },

            LexerState::Sigil => match c {
                None => reconsume()
                    .and_emit(LexToken::Sigil)
                    .and_transition(LexerState::Top),
                Some(c) if is_sigil_char(c) => consume().and_remain(),
                _ => reconsume()
                    .and_emit(LexToken::Sigil)
                    .and_transition(LexerState::Top),
            },

            LexerState::StringLiteral => match c {
                None => LexerNext::Error(c),
                Some(c) => match c {
                    '"' => consume()
                        .and_emit(LexToken::String)
                        .and_transition(LexerState::Top),
                    _ => consume().and_remain(),
                },
            },

            LexerState::StartIdent => match c {
                None => LexerNext::emit(LexToken::Identifier, LexerState::Top),
                Some(c) => match c {
                    c if UnicodeXID::is_xid_continue(c) => {
                        consume().and_transition(LexerState::ContinueIdent)
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
                    c if UnicodeXID::is_xid_continue(c) => consume().and_remain(),
                    _ => reconsume()
                        .and_emit(LexToken::Identifier)
                        .and_transition(LexerState::Top),
                },
            },

            LexerState::Whitespace => match c {
                None => LexerNext::EOF,
                Some(c) => match c {
                    '\n' => reconsume().and_discard().and_transition(LexerState::Top),
                    c if c.is_whitespace() => consume().and_remain(),
                    _ => reconsume()
                        .and_emit(LexToken::Whitespace)
                        .and_transition(LexerState::Top),
                },
            },

            LexerState::Comment(1) => {
                if rest.starts_with("/*") {
                    consume_n(2)
                        .and_continue()
                        .and_transition(LexerState::Comment(2))
                } else if rest.starts_with("*/") {
                    consume_n(2)
                        .and_emit(LexToken::Comment)
                        .and_transition(LexerState::Top)
                } else {
                    consume().and_remain()
                }
            }

            LexerState::Comment(n) => {
                if rest.starts_with("/*") {
                    consume_n(2)
                        .and_continue()
                        .and_transition(LexerState::Comment(n + 1))
                } else if rest.starts_with("*/") {
                    consume_n(2)
                        .and_continue()
                        .and_transition(LexerState::Comment(n - 1))
                } else {
                    consume().and_remain()
                }
            }
        };

        Ok(out)
    }
}

fn is_sigil_char(c: char) -> bool {
    match c {
        '{' | '}' | '(' | ')' | '+' | '-' | '*' | '/' | ':' | ',' | '>' | '<' | '=' => true,
        _ => false,
    }
}

fn is_delimiter_sigil_char(c: char) -> bool {
    match c {
        '{' | '}' | '(' | ')' | '>' | '<' => true,
        _ => false,
    }
}
