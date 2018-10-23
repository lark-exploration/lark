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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PairedDelimiter {
    Curly,
    Round,
    Square,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ShapeStart {
    Identifier(Spanned<StringId>),
    Macro(Spanned<StringId>),
    PairedDelimiter(PairedDelimiter),
    String,
    Prefix(StringId),
    EOF,
}

#[derive(Debug, Copy, Clone)]
pub enum ShapeContinue {
    Identifier(StringId),
    Macro(StringId),
    Sigil(StringId),
    Operator(StringId),
    PairedDelimiter(PairedDelimiter),
    Newline,
    EOF,
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
    ExpectedId(ExpectedId),
    ExpectedSigil(ExpectedSigil),
}

impl From<ExpectedSigil> for Expected {
    fn from(from: ExpectedSigil) -> Expected {
        Expected::ExpectedSigil(from)
    }
}

impl From<ExpectedId> for Expected {
    fn from(from: ExpectedId) -> Expected {
        Expected::ExpectedId(from)
    }
}

impl Expected {
    fn matches(&self, token: &LexToken) -> bool {
        match self {
            Expected::ExpectedId(id) => id.matches(token),
            Expected::ExpectedSigil(sigil) => sigil.matches(token),
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
pub enum ExpectedSigil {
    AnySigil,
    Sigil(StringId),
}

pub trait IntoExpectedSigil: DebugModuleTable {
    fn into_expected_sigil(&self, table: &ModuleTable) -> ExpectedSigil;
}

impl IntoExpectedSigil for &str {
    fn into_expected_sigil(&self, table: &ModuleTable) -> ExpectedSigil {
        let id = table
            .get(self)
            .expect(&format!("Unexpected missing sigil {:?}", self));
        ExpectedSigil::Sigil(id)
    }
}

impl IntoExpectedSigil for StringId {
    fn into_expected_sigil(&self, table: &ModuleTable) -> ExpectedSigil {
        ExpectedSigil::Sigil(*self)
    }
}

impl IntoExpectedSigil for ExpectedSigil {
    fn into_expected_sigil(&self, _table: &ModuleTable) -> ExpectedSigil {
        *self
    }
}

impl DebugModuleTable for ExpectedSigil {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            ExpectedSigil::AnySigil => write!(f, "<any sigil>"),
            ExpectedSigil::Sigil(id) => write!(f, "#{:?}#", Debuggable::from(id, table)),
        }
    }
}

impl ExpectedSigil {
    fn matches(&self, token: &LexToken) -> bool {
        match self {
            ExpectedSigil::AnySigil => token.is_sigil(),
            ExpectedSigil::Sigil(s) => token.is_sigil_named(*s),
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

pub const EOF: Spanned<LexToken> = Spanned(LexToken::EOF, Span::EOF);

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

    pub fn tokens(&self) -> (&[Spanned<LexToken>], usize) {
        let tokens = &self.tokens[..];
        let pos = self.pos();

        (tokens, pos)
    }

    pub fn tree(&mut self) -> &mut TokenTree {
        &mut self.tree
    }

    fn has_macro(&self, id: &StringId) -> bool {
        self.macros.has(&id)
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

    pub fn expect_paired_delimiters(
        &mut self,
        allow: AllowPolicy,
        delimiters: PairedDelimiter,
    ) -> Result<(), ParseResult> {
    }

    pub fn expect_id_until(
        &mut self,
        allow: AllowPolicy,
        expected: ExpectedId,
        terminator: impl Into<Expected>,
    ) -> Result<MaybeTerminator, ParseError> {
        trace!(target: "lark::reader", "expect_id_until");

        let next = self.consume_next_token(allow)?;

        match next {
            Spanned(LexToken::EOF, ..) => Ok(MaybeTerminator::Token(EOF)),
            token @ Spanned(..) => match token.node() {
                id if terminator.into().matches(&id) => Ok(MaybeTerminator::Terminator(token)),
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

    pub fn sigil(&self, sigil: &str) -> ExpectedSigil {
        let id = self.table.get(&sigil).expect(&format!(
            "Expected sigil {}, but none was registered",
            sigil
        ));

        ExpectedSigil::Sigil(id)
    }

    pub fn any_sigil(&self) -> ExpectedSigil {
        ExpectedSigil::AnySigil
    }

    pub fn consume_start_expr(
        &mut self,
        shape: ShapeStart,
        allow: AllowPolicy,
    ) -> Result<(), ParseError> {
        // TODO: Validate ShapeStart
        self.consume_next_token(allow).map(|_| ())
    }

    pub fn consume_continue_expr(
        &mut self,
        shape: ShapeContinue,
        allow: AllowPolicy,
    ) -> Result<(), ParseError> {
        // TODO: Validate ShapeStart
        self.consume_next_token(allow).map(|_| ())
    }

    pub fn peek_start_expr(&self, allow: AllowPolicy) -> Result<ShapeStart, ParseError> {
        if self.tree.is_done() {
            if allow.has(ALLOW_EOF) {
                return Ok(ShapeStart::EOF);
            } else {
                return Err(ParseError::new(format!("Unexpected EOF"), Span::EOF));
            }
        }

        let mut pos = self.pos();

        loop {
            let token = self.tokens[pos];
            pos += 1;

            match token.node() {
                LexToken::EOF => unreachable!(),
                LexToken::Newline if allow.has(ALLOW_NEWLINE) => continue,
                LexToken::Newline => {
                    return Err(ParseError::new(format!("Unexpected newline"), token.span()))
                }
                LexToken::Comment(_) => continue,
                LexToken::String(_) => {
                    return Err(ParseError::new(format!("Unexpected string"), token.span()))
                }
                LexToken::Whitespace(_) => continue,
                LexToken::Identifier(id) => {
                    if self.has_macro(&id) {
                        return Ok(ShapeStart::Macro(token.copy(*id)));
                    } else {
                        return Ok(ShapeStart::Identifier(token.copy(*id)));
                    }
                }
                sigil @ LexToken::Sigil(_) => match sigil {
                    _ if self.sigil("{").matches(sigil) => {
                        return Ok(ShapeStart::PairedDelimiter(PairedDelimiter::Curly))
                    }
                    _ if self.sigil("(").matches(sigil) => {
                        return Ok(ShapeStart::PairedDelimiter(PairedDelimiter::Round))
                    }
                    _ if self.sigil("[").matches(sigil) => {
                        return Ok(ShapeStart::PairedDelimiter(PairedDelimiter::Square))
                    }
                    sigil => {
                        return Err(ParseError::new(
                            format!(
                                "Unexpected sigil {:?}",
                                Debuggable::from(&token, self.table()),
                            ),
                            token.span(),
                        ))
                    }
                },
            }
        }
    }

    pub fn peek_continue_expr(&self, allow: AllowPolicy) -> Result<ShapeContinue, ParseError> {
        if self.tree.is_done() {
            if allow.has(ALLOW_EOF) {
                return Ok(ShapeContinue::EOF);
            } else {
                return Err(ParseError::new(format!("Unexpected EOF"), Span::EOF));
            }
        }

        let token = self.tokens[self.pos()];

        loop {
            match token.node() {
                LexToken::EOF => unreachable!(),

                // TODO: Leading `.` on next line. Requires an extra lookahead through whitespace
                LexToken::Newline if allow.has(ALLOW_NEWLINE) => return Ok(ShapeContinue::Newline),
                LexToken::Newline => {
                    return Err(ParseError::new(format!("Unexpected newline"), token.span()))
                }
                LexToken::Comment(_) => continue,
                LexToken::String(_) => {
                    return Err(ParseError::new(
                        format!("Unimplemented string in continuation position"),
                        token.span(),
                    ))
                }
                LexToken::Whitespace(_) => continue,
                LexToken::Identifier(id) => {
                    return Ok(ShapeContinue::Identifier(*id));
                }
                sigil @ LexToken::Sigil(_) => match sigil {
                    _ if self.sigil("{").matches(sigil) => {
                        return Ok(ShapeContinue::PairedDelimiter(PairedDelimiter::Curly))
                    }
                    _ if self.sigil("(").matches(sigil) => {
                        return Ok(ShapeContinue::PairedDelimiter(PairedDelimiter::Round))
                    }
                    _ if self.sigil("[").matches(sigil) => {
                        return Ok(ShapeContinue::PairedDelimiter(PairedDelimiter::Square))
                    }
                    sigil => {
                        return Err(ParseError::new(
                            format!(
                                "Unimplemented operators {:?}",
                                Debuggable::from(&token, self.table()),
                            ),
                            token.span(),
                        ))
                    }
                },
            }
        }
    }

    pub fn expect_paired(&mut self, open: PairedDelimiter) -> Result<(), ParseError> {
        unimplemented!()
    }

    pub fn expect_id(&mut self, allow: AllowPolicy) -> Result<Spanned<StringId>, ParseError> {
        trace!(target: "lark::reader", "expect_id");
        let id_token = self.consume_next_id(allow)?;

        id_token.as_id()
    }

    pub fn expect_type(&mut self, whitespace: AllowPolicy) -> Result<Handle, ParseError> {
        trace!(target: "lark::reader", "expect_type");
        self.tree.start("type");
        self.tree.mark_type();
        self.consume_next_id(whitespace)?;
        let handle = self.tree.end("type");

        Ok(handle)
    }

    pub fn maybe_sigil(
        &mut self,
        sigil: impl IntoExpectedSigil,
        allow: AllowPolicy,
    ) -> Result<Result<Spanned<LexToken>, Spanned<LexToken>>, ParseError> {
        self.tree.mark_backtrack_point("maybe_sigil");
        let next = self.consume_next_token(allow)?;
        let sigil = sigil.into_expected_sigil(self.table());

        match next.node() {
            LexToken::Sigil(s) if sigil.matches(&next) => {
                self.tree.commit("maybe_sigil");
                Ok(Ok(next))
            }
            _ => {
                self.tree.backtrack("maybe_sigil");
                Ok(Err(next))
            }
        }
    }

    pub fn expect_sigil(
        &mut self,
        sigil: impl IntoExpectedSigil,
        allow: AllowPolicy,
    ) -> Result<(), ParseError> {
        trace!(target: "lark::reader", "expect_sigil {:?}", Debuggable::from(&sigil, self.table()));

        match self.maybe_sigil(sigil, allow)? {
            Ok(_) => Ok(()),
            Err(token) => Err(ParseError::new(
                format!("Unexpected {:?}", *token),
                token.span(),
            )),
        }
    }

    pub fn current_span(&self) -> Span {
        let token = self.tokens[self.pos()];
        token.span()
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

    fn mark_backtrack_point(&mut self) {}

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
        self.tree.backtrack(debug_from);
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
