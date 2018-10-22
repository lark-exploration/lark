use crate::prelude::*;

use crate::parser::{ModuleTable, ParseError, Span, Spanned, StringId};
use crate::parser2::allow::{AllowPolicy, ALLOW_EOF, ALLOW_NEWLINE};
use crate::parser2::builtins::{self, ExprParser};
use crate::parser2::entity_tree::{EntityTree, EntityTreeBuilder};
use crate::parser2::macros::{macros, MacroRead, Macros};
use crate::parser2::quicklex::Token as LexToken;
use crate::parser2::token_tree::{Handle, TokenPos, TokenSpan, TokenTree};

use bimap::BiMap;
use codespan::CodeMap;
use log::{debug, trace};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use derive_new::new;

#[derive(Debug, Copy, Clone)]
enum NextAction {
    Top,
    Macro(StringId),
}

#[derive(Debug)]
pub struct Reader<'codemap> {
    tokens: Vec<Spanned<LexToken>>,
    macros: Macros,
    table: ModuleTable,
    codemap: &'codemap CodeMap,
    entity_tree: EntityTreeBuilder,
    tree: TokenTree,
}

impl Reader<'codemap> {
    pub fn new(
        tokens: Vec<Spanned<LexToken>>,
        macros: Macros,
        table: ModuleTable,
        codemap: &'codemap CodeMap,
    ) -> Reader<'codemap> {
        let len = tokens.len();

        Reader {
            tokens,
            macros,
            table,
            codemap,
            entity_tree: EntityTreeBuilder::new(),
            tree: TokenTree::new(len),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum RelativePosition {
    Hoist,
    After,
}

#[derive(Debug, Copy, Clone)]
pub enum Expected {
    AnyIdentifier,
    Identifier(StringId),
    Sigil(StringId),
}

impl Expected {
    fn matches(&self, token: &LexToken) -> bool {
        match self {
            Expected::AnyIdentifier => token.is_id(),
            Expected::Identifier(s) => token.is_id_named(*s),
            Expected::Sigil(s) => token.is_sigil_named(*s),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ExpectedId {
    AnyIdentifier,
    Identifier(StringId),
}

impl ExpectedId {
    fn matches(&self, token: &LexToken) -> bool {
        match self {
            ExpectedId::AnyIdentifier => token.is_id(),
            ExpectedId::Identifier(s) => token.is_id_named(*s),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MaybeTerminator {
    Token(Spanned<LexToken>),
    Terminator(Spanned<LexToken>),
}

impl DebugModuleTable for MaybeTerminator {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            MaybeTerminator::Token(token) => token.debug(f, table),
            MaybeTerminator::Terminator(token) => token.debug(f, table),
        }
    }
}

pub struct ParseResult {
    tree: TokenTree,
    entity_tree: EntityTree,
}

const EOF: Spanned<LexToken> = Spanned(LexToken::EOF, Span::EOF);

impl Reader<'codemap> {
    pub fn process(mut self) -> Result<ParseResult, ParseError> {
        while !self.tree().is_done() {
            debug!(target: "lark::reader", "LOOPING!");
            self.process_macro()?;
        }

        Ok(ParseResult {
            tree: self.tree,
            entity_tree: self.entity_tree.finalize(),
        })
    }

    pub fn table(&self) -> &ModuleTable {
        &self.table
    }

    pub fn tree(&mut self) -> &mut TokenTree {
        &mut self.tree
    }

    fn get_macro(&mut self, id: Spanned<StringId>) -> Result<Arc<MacroRead>, ParseError> {
        self.macros.get(*id).ok_or_else(|| {
            ParseError::new(
                format!("No macro in scope {:?}", Debuggable::from(&id, &self.table)),
                id.span(),
            )
        })
    }

    fn pos(&self) -> usize {
        self.tree.pos()
    }

    fn process_macro(&mut self) -> Result<(), ParseError> {
        trace!(target: "lark::reader", "processing macro");
        let token = self.consume_next_id(ALLOW_NEWLINE | ALLOW_EOF)?;

        if let LexToken::EOF = token.node() {
            return Ok(());
        } else if token.is_id() {
            debug!(target: "lark::reader",
                "Processing macro {:?}",
                Debuggable::from(&token, self.table())
            );
            let macro_def = self.get_macro(token.as_id().unwrap())?;

            macro_def.extent(self)?;

            Ok(())
        } else {
            Err(ParseError::new(
                format!(
                    "Expected identifier, found {:?}",
                    Debuggable::from(&token, self.table())
                ),
                token.span(),
            ))
        }
    }

    pub fn expect_id_until(
        &mut self,
        allow: AllowPolicy,
        expected: ExpectedId,
        terminator: Expected,
    ) -> Result<MaybeTerminator, ParseError> {
        trace!(target: "lark::reader", "expect_id_until");

        let next = self.consume_next_token(allow)?;

        match next {
            Spanned(LexToken::EOF, ..) => Ok(MaybeTerminator::Token(EOF)),
            token @ Spanned(..) => match token.node() {
                id if terminator.matches(&id) => Ok(MaybeTerminator::Terminator(token)),
                id if expected.matches(&id) => Ok(MaybeTerminator::Token(token)),
                other => {
                    return Err(ParseError::new(
                        format!("Unexpected token {:?}", other),
                        next.span(),
                    ))
                }
            },
        }
    }

    pub fn sigil(&self, sigil: &str) -> Expected {
        let id = self.table.get(&sigil).expect(&format!(
            "Expected sigil {}, but none was registered",
            sigil
        ));

        Expected::Sigil(id)
    }

    pub fn expect_id(&mut self, allow: AllowPolicy) -> Result<Spanned<StringId>, ParseError> {
        trace!(target: "lark::reader", "expect_id");
        let id_token = self.consume_next_id(allow)?;

        id_token.as_id()
    }

    pub fn expect_type(&mut self, whitespace: AllowPolicy) -> Result<Handle, ParseError> {
        trace!(target: "lark::reader", "expect_type");
        self.tree.start();
        self.tree.mark_type();
        self.consume_next_id(whitespace)?;
        let handle = self.tree.end();

        Ok(handle)
    }

    pub fn maybe_sigil(
        &mut self,
        sigil: &str,
        allow: AllowPolicy,
    ) -> Result<(bool, Spanned<LexToken>), ParseError> {
        let id = self.table.get(&sigil);

        match id {
            None => unimplemented!(),

            Some(id) => match self.consume_next_token(allow)? {
                eof @ Spanned(LexToken::EOF, ..) => Ok((true, eof)),

                Spanned(LexToken::Sigil(sigil), span) if sigil == id => {
                    Ok((true, Spanned::wrap_span(LexToken::Sigil(sigil), span)))
                }

                other => {
                    self.backtrack("maybe_sigil");
                    Ok((false, other))
                }
            },
        }
    }

    pub fn expect_sigil(&mut self, sigil: &str, allow: AllowPolicy) -> Result<(), ParseError> {
        trace!(target: "lark::reader", "expect_sigil #{}#", sigil);

        match self.maybe_sigil(sigil, allow)? {
            (true, _) => Ok(()),
            (false, token) => Err(ParseError::new(
                format!("Unexpected {:?}", *token),
                token.span(),
            )),
        }
    }

    pub fn expect_expr(&mut self) -> Result<Handle, ParseError> {
        ExprParser.extent(self)
    }

    pub fn start_entity(&mut self, name: &StringId) {
        self.entity_tree.push(*name, TokenPos(self.pos()));
    }

    pub fn end_entity(&mut self) {
        self.entity_tree.finish(TokenPos(self.pos()));
    }

    fn consume(&mut self) -> Spanned<LexToken> {
        if self.tree().is_done() {
            trace!(target: "lark::reader", "in token=EOF")
        } else {
            trace!(
                target: "lark::reader",
                "in token={:?}",
                Debuggable::from(&self.tokens[self.pos()], self.table())
            )
        }

        let token = self.tokens[self.pos()];

        self.tick("consume");

        token
    }

    fn maybe_consume(&mut self) -> Option<Spanned<LexToken>> {
        if self.tree().is_done() {
            None
        } else {
            Some(self.consume())
        }
    }

    fn consume_next_token(&mut self, allow: AllowPolicy) -> Result<Spanned<LexToken>, ParseError> {
        trace!(target: "lark::reader", "consume_next_token");

        loop {
            trace!(target: "lark::reader", "consume_next_token ->");
            let token = self.maybe_consume();

            let token = match token {
                None if allow.has(ALLOW_EOF) => {
                    return Ok(Spanned::wrap_span(LexToken::EOF, Span::EOF))
                }
                None => return Err(ParseError::new(format!("Unexpected EOF"), Span::EOF)),
                Some(token) => token,
            };

            match *token {
                LexToken::Whitespace(..) => {}
                LexToken::Newline if allow.has(ALLOW_NEWLINE) => {}
                _ => return Ok(token),
            }
        }
    }

    fn consume_next_id(&mut self, allow: AllowPolicy) -> Result<Spanned<LexToken>, ParseError> {
        let next = self.consume_next_token(allow)?;

        let token = match *next {
            LexToken::EOF if allow.has(ALLOW_EOF) => return Ok(EOF),
            LexToken::EOF => {
                return Err(ParseError::new(
                    "Unexpected EOF in macro expansion, TODO".to_string(),
                    Span::EOF,
                ))
            }
            _ => next.expect_id()?,
        };

        Ok(token)
    }

    fn tick(&mut self, debug_from: &str) {
        trace!(target: "lark::reader",
            "from: {}, processed token: {:?}",
            debug_from,
            Debuggable::from(&self.tokens[self.pos()], self.table())
        );
        self.tree.tick();
    }

    fn backtrack(&mut self, debug_from: &str) {
        trace!(target: "lark::reader",
            "from: {}, backtracked token: {:?}",
            debug_from,
            Debuggable::from(&self.tokens[self.pos()], self.table())
        );
        self.tree.backtrack();
    }
}

fn expect(
    token: Option<Spanned<LexToken>>,
    condition: impl FnOnce(LexToken) -> bool,
) -> Result<Spanned<LexToken>, ParseError> {
    match token {
        None => Err(ParseError::new(format!("Unexpected EOF"), Span::EOF)),

        Some(token) if condition(*token) => Ok(token),

        Some(other) => Err(ParseError::new(
            format!("Unexpected {:?}", other),
            other.span(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::Reader;

    use crate::parser::ast::DebuggableVec;
    use crate::parser::lexer_helpers::ParseError;
    use crate::parser::reporting::print_parse_error;
    use crate::parser::{Span, Spanned};
    use crate::parser2::macros::{macros, Macros};
    use crate::parser2::quicklex::{Token, Tokenizer};
    use crate::parser2::test_helpers::{process, Annotations, Position};

    use log::trace;
    use std::collections::HashMap;
    use unindent::unindent;

    #[test]
    fn test_reader() {
        crate::init_logger();

        return;

        // let source = unindent(
        //     r##"
        //     struct Diagnostic {
        //     ^^^^^^~^^^^^^^^^^~^ @struct@ ws @Diagnostic@ ws #{#
        //       msg: String,
        //       ^^^~^~~~~~~^ @msg@ #:# ws @String@ #,#
        //       level: String,
        //       ^^^^^~^~~~~~~^ @level@ #:# ws @String@ #,#
        //     }
        //     ^ #}#
        //     "##,
        // );

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
            def new(msg: String, level: String) -> Diagnostic {
            ^^^~^^^~^^^~^~~~~~~^~^^^^^~^~~~~~~^~^^~^^^^^^^^^^~^ @def@ ws @new@ #(# @msg@ #:# ws @String@ #,# ws @level@ #:# ws @String@ #)# ws #-># ws @Diagnostic@ ws #{#
              Diagnostic { msg, level }
              ^^^^^^^^^^~^~^^^~^~~~~~^~ @Diagnostic@ ws #{# ws @msg@ #,# ws @level@ ws #}#
            }
            ^ #}#
            "##,
        );

        let (source, mut ann) = process(&source);

        let filemap = ann.codemap().add_filemap("test".into(), source.clone());
        let start = filemap.span().start().0;

        let tokens = match Tokenizer::new(ann.table(), &source, start).tokens() {
            Ok(tokens) => tokens,
            Err(e) => print_parse_error(e, ann.codemap()),
        };

        // let tokens: Result<Vec<Spanned<Token>>, ParseError> = lexed
        //     .map(|result| result.map(|(start, tok, end)| Spanned::from(tok, start, end)))
        //     .collect();

        trace!(target: "lark::reader::test", "{:#?}", DebuggableVec::from(&tokens.clone(), ann.table()));

        let builtin_macros = macros(ann.table());

        let parser = Reader::new(tokens, builtin_macros, ann.table().clone(), ann.codemap());

        match parser.process() {
            Ok(_) => {}
            Err(e) => print_parse_error(e, ann.codemap()),
        };
    }
}
