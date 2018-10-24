use crate::parser::ast::{DebugModuleTable, Debuggable};
use crate::parser::keywords::{KEYWORDS, SIGILS};
use crate::parser::lexer_helpers::{begin, consume, consume_n, reconsume};
use crate::parser::lexer_helpers::{
    LexerAccumulate, LexerAction, LexerDelegateTrait, LexerNext, LexerToken, ParseError,
    Tokenizer as GenericTokenizer,
};
use crate::parser::program::StringId;
use crate::parser::{ModuleTable, Span, Spanned};
use crate::parser2::LexToken;

use codespan::ByteOffset;
use derive_new::new;
use lazy_static::lazy_static;
use log::{trace, warn};
use std::fmt;
use unicode_xid::UnicodeXID;

pub type Tokenizer<'table> = GenericTokenizer<'table, LexerState>;

// impl Tokenizer<'table> {
//     fn tokens(self) -> Result<Vec<Spanned<Token>>, ParseError> {
//         self.map(|result| result.map(|(start, tok, end)| Spanned::from(tok, start, end)))
//             .collect()
//     }
// }

#[derive(Debug, Copy, Clone)]
pub enum LexerState {
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
    ) -> Result<LexerNext<Self>, ParseError> {
        use self::LexerState::*;

        let out = match self {
            LexerState::Top => match c {
                None => LexerNext::EOF,
                Some(c) => match c {
                    c if UnicodeXID::is_xid_start(c) => LexerNext::begin(StartIdent),
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
                    .and_emit_dynamic(LexToken::sigil)
                    .and_transition(LexerState::Top),
                Some(c) if is_sigil_char(c) => consume().and_remain(),
                _ => reconsume()
                    .and_emit_dynamic(LexToken::sigil)
                    .and_transition(LexerState::Top),
            },

            LexerState::StringLiteral => match c {
                None => LexerNext::Error(c),
                Some(c) => match c {
                    '"' => consume()
                        .and_emit_dynamic(LexToken::String)
                        .and_transition(LexerState::Top),
                    _ => consume().and_remain(),
                },
            },

            LexerState::StartIdent => match c {
                None => LexerNext::emit_dynamic(LexToken::Identifier, LexerState::Top),
                Some(c) => match c {
                    c if UnicodeXID::is_xid_continue(c) => {
                        consume().and_transition(LexerState::ContinueIdent)
                    }

                    // TODO: Should this be a pop, so we don't have to reiterate
                    // the state name?
                    _ => reconsume()
                        .and_emit_dynamic(LexToken::Identifier)
                        .and_transition(LexerState::Top),
                },
            },

            LexerState::ContinueIdent => match c {
                None => LexerNext::emit_dynamic(LexToken::Identifier, LexerState::Top),
                Some(c) => match c {
                    c if UnicodeXID::is_xid_continue(c) => consume().and_remain(),
                    _ => reconsume()
                        .and_emit_dynamic(LexToken::Identifier)
                        .and_transition(LexerState::Top),
                },
            },

            LexerState::Whitespace => match c {
                None => LexerNext::EOF,
                Some(c) => match c {
                    '\n' => reconsume().and_discard().and_transition(LexerState::Top),
                    c if c.is_whitespace() => consume().and_remain(),
                    _ => reconsume()
                        .and_emit_dynamic(LexToken::Whitespace)
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
                        .and_emit_dynamic(LexToken::Comment)
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

#[cfg(test)]
mod tests {
    use crate::LexToken;
    use crate::parser::ast::DebuggableVec;
    use crate::parser::lexer_helpers::ParseError;
    use crate::parser::{Span, Spanned};
    use crate::parser2::test_helpers::{process, Annotations, Position};
    use super::Tokenizer;

    use log::trace;
    use unindent::unindent;

    #[test]
    fn test_quicklex() -> Result<(), ParseError> {
        crate::init_logger();

        let source = unindent(
            r##"
            struct Diagnostic {
            ^^^^^^~^^^^^^^^^^~^ @struct@ ws @Diagnostic@ ws #{#
              msg: String,
              ^^^~^~~~~~~^ @msg@ #:# ws @String@ #,#
              level: String,
              ^^^^^~^~~~~~~^ @level@ #:# ws @String@ #,#
            }
            ^ #}#
            "##,
        );

        let (source, mut ann) = process(&source);

        let filemap = ann.codemap().add_filemap("test".into(), source.clone());
        let start = filemap.span().start().0;

        let lexed = Tokenizer::new(ann.table(), &source, start);

        let tokens: Result<Vec<Spanned<LexToken>>, ParseError> = lexed
            .map(|result| result.map(|(start, tok, end)| Spanned::from(tok, start, end)))
            .collect();

        trace!("{:#?}", DebuggableVec::from(&tokens.clone()?, ann.table()));

        Ok(())
    }

}
