use crate::prelude::*;

use crate::intern::ModuleTable;
use crate::parser2::allow::{AllowPolicy, ALLOW_EOF, ALLOW_NEWLINE};
use crate::parser2::builtins::ExprParser;
use crate::parser2::builtins::Paired;
use crate::parser2::entity_tree::{Entities, EntitiesBuilder, EntityKind};
use crate::parser2::macros::{MacroRead, Macros, Term};
use crate::parser2::token;
use crate::parser2::token_tree::{Handle, TokenNode, TokenPos, TokenTree};
use crate::LexToken;

use codespan::CodeMap;
use log::{debug, trace};
use std::fmt;
use std::sync::Arc;

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
    Identifier(Spanned<StringId>),
    Macro(Spanned<StringId>),
    Sigil(Spanned<StringId>),
    Operator(Spanned<StringId>),
    PairedDelimiter(Spanned<PairedDelimiter>),
    Newline,
    EOF,
}

#[derive(Debug)]
pub struct Reader<'codemap> {
    tokens: Vec<Spanned<LexToken>>,
    macros: Macros,
    table: ModuleTable,
    codemap: &'codemap CodeMap,
    entity_tree: EntitiesBuilder,
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
            entity_tree: EntitiesBuilder::new(),
            tree: TokenTree::new(len),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum RelativePosition {
    #[allow(unused)]
    Hoist,
    #[allow(unused)]
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
            Expected::ExpectedSigil(sigil) => sigil.matches_token(token),
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
    fn into_expected_sigil(&self, _table: &ModuleTable) -> ExpectedSigil {
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
    fn matches_token(&self, token: &LexToken) -> bool {
        match self {
            ExpectedSigil::AnySigil => token.is_sigil(),
            ExpectedSigil::Sigil(s) => token.is_sigil_named(*s),
        }
    }

    fn matches(&self, sigil: &token::Sigil) -> bool {
        match self {
            ExpectedSigil::AnySigil => true,
            ExpectedSigil::Sigil(s) => *s == sigil.0,
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

#[derive(Debug)]
pub struct ParseResult {
    node_tree: Vec<TokenNode>,
    entity_tree: Entities,
}

pub const EOF: Spanned<LexToken> = Spanned(LexToken::EOF, Span::EOF);

impl Reader<'codemap> {
    pub fn process(mut self) -> Result<ParseResult, ParseError> {
        while !self.tree().is_done() {
            debug!(target: "lark::reader", "LOOPING!");
            self.process_macro()?;
        }

        Ok(ParseResult {
            node_tree: self.tree.finalize(),
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
        trace!(target: "lark::reader", "# process_macro");

        let token = self.consume_next_id(ALLOW_NEWLINE | ALLOW_EOF)?;
        self.tree.start_at(self.tree.last_non_ws(), "macro");

        if let LexToken::EOF = token.node() {
            return Ok(());
        } else if token.is_id() {
            debug!(target: "lark::reader",
                "Processing macro {:?}",
                Debuggable::from(&token, self.table())
            );
            let macro_def = self.get_macro(token.as_id().unwrap())?;

            macro_def.extent(self)?;

            self.tree.end_at(self.tree.last_non_ws(), "macro");

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
        start: Spanned<PairedDelimiter>,
    ) -> Result<(), ParseError> {
        trace!(target: "lark::reader", "# paired delimiters; start={:?} @ {:?}", start.node(), start.span());
        let mut paired = Paired::start(self, start);

        let end = paired.process()?;

        self.tree.fast_forward(end, end - 1);

        Ok(())
    }

    pub fn expect_id_until(
        &mut self,
        allow: AllowPolicy,
        expected: ExpectedId,
        terminator: impl Into<Expected>,
    ) -> Result<MaybeTerminator, ParseError> {
        trace!(target: "lark::reader", "# expect_id_until");

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
        _shape: ShapeStart,
        allow: AllowPolicy,
    ) -> Result<(), ParseError> {
        // TODO: Validate ShapeStart
        self.consume_next_token(allow).map(|_| ())
    }

    pub fn consume_continue_expr(
        &mut self,
        _shape: ShapeContinue,
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
                LexToken::Sigil(sigil) => match sigil {
                    _ if self.sigil("{").matches(sigil) => {
                        return Ok(ShapeStart::PairedDelimiter(PairedDelimiter::Curly))
                    }
                    _ if self.sigil("(").matches(sigil) => {
                        return Ok(ShapeStart::PairedDelimiter(PairedDelimiter::Round))
                    }
                    _ if self.sigil("[").matches(sigil) => {
                        return Ok(ShapeStart::PairedDelimiter(PairedDelimiter::Square))
                    }
                    _ => {
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
        trace!(target: "lark::reader", "# peek_continue_expr");

        if self.tree.is_done() {
            if allow.has(ALLOW_EOF) {
                return Ok(ShapeContinue::EOF);
            } else {
                return Err(ParseError::new(format!("Unexpected EOF"), Span::EOF));
            }
        }

        let mut pos = self.pos();

        loop {
            let token = self.tokens[pos];
            pos += 1;

            trace!(target: "lark::reader", "peeked {:?}", Debuggable::from(&token, self.table()));

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
                    return Ok(ShapeContinue::Identifier(token.copy(*id)));
                }
                LexToken::Sigil(sigil) => match sigil {
                    _ if self.sigil("{").matches(sigil) => {
                        return Ok(ShapeContinue::PairedDelimiter(
                            token.copy(PairedDelimiter::Curly),
                        ))
                    }
                    _ if self.sigil("(").matches(sigil) => {
                        return Ok(ShapeContinue::PairedDelimiter(
                            token.copy(PairedDelimiter::Round),
                        ))
                    }
                    _ if self.sigil("[").matches(sigil) => {
                        return Ok(ShapeContinue::PairedDelimiter(
                            token.copy(PairedDelimiter::Square),
                        ))
                    }
                    _ => {
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

    pub fn next_non_ws(&self, from: usize) -> TokenPos {
        let mut pos = from;
        loop {
            let token = self.tokens[pos];
            pos += 1;

            match token.node() {
                LexToken::Whitespace(..) => continue,
                LexToken::Newline => continue,
                _ => return TokenPos(pos - 1),
            }
        }
    }

    pub fn expect_id(&mut self, allow: AllowPolicy) -> Result<Spanned<StringId>, ParseError> {
        trace!(target: "lark::reader", "# expect_id");
        let id_token = self.consume_next_id(allow)?;

        id_token.as_id()
    }

    pub fn expect_type(&mut self, whitespace: AllowPolicy) -> Result<Handle, ParseError> {
        trace!(target: "lark::reader", "# expect_type");
        self.tree.start("type");
        self.tree.mark_type();
        self.consume_next_id(whitespace)?;
        let handle = self.tree.end("type");

        Ok(handle)
    }

    pub fn maybe_sigil(
        &mut self,
        expected: impl IntoExpectedSigil,
        allow: AllowPolicy,
    ) -> Result<Result<Spanned<token::Sigil>, Spanned<LexToken>>, ParseError> {
        self.tree.mark_backtrack_point("maybe_sigil");
        let next = self.consume_next_token(allow)?;
        let expected = expected.into_expected_sigil(self.table());

        match next.node() {
            LexToken::Sigil(sigil) if expected.matches(&sigil) => {
                self.tree.commit("maybe_sigil");
                Ok(Ok(next.copy(*sigil)))
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
    ) -> Result<Spanned<token::ClassifiedSigil>, ParseError> {
        trace!(target: "lark::reader", "# expect_sigil {:?}", Debuggable::from(&sigil, self.table()));

        match self.maybe_sigil(sigil, allow)? {
            Ok(token) => Ok(token.copy(token.classify(self.table()))),
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

    pub fn start_entity(&mut self, name: &StringId, kind: EntityKind) {
        self.entity_tree
            .push(name, TokenPos(self.tree.last_non_ws()), kind);
    }

    pub fn end_entity(&mut self, term: Box<dyn Term>) {
        self.entity_tree
            .finish(TokenPos(self.tree.last_non_ws()), term);
    }

    fn consume(&mut self) -> Spanned<LexToken> {
        if self.tree().is_done() {
            trace!(target: "lark::reader", "in token=EOF")
        } else {
            trace!(
                target: "lark::reader",
                "in token={:?} pos={:?}",
                Debuggable::from(&self.tokens[self.pos()], self.table()), self.pos()
            )
        }

        let token = self.tokens[self.pos()];

        self.tick(&token, "consume");

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
            let token = self.maybe_consume();
            let token = match token {
                None if allow.has(ALLOW_EOF) => {
                    return Ok(Spanned::wrap_span(LexToken::EOF, Span::EOF))
                }
                None => return Err(ParseError::new(format!("Unexpected EOF"), Span::EOF)),
                Some(token) => token,
            };

            trace!(target: "lark::reader", "token = {:?}", Debuggable::from(&token, self.table()));

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

    fn tick(&mut self, token: &LexToken, debug_from: &str) {
        trace!(target: "lark::reader",
            "tick: processed token: {:?} (from: {})",
            Debuggable::from(&self.tokens[self.pos()], self.table()),
            debug_from,
        );

        if !token.is_whitespace() {
            self.tree.tick_non_ws();
        }

        self.tree.tick();
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    use super::{ParseResult, Reader};

    use crate::parser2::macros::macros;
    use crate::parser2::quicklex::Tokenizer;
    use crate::parser2::test_helpers::process;
    use crate::parser2::token::token_pos_at;
    use crate::print_parse_error;

    use derive_new::new;
    use log::debug;
    use unindent::unindent;

    #[test]
    fn test_reader() {
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
            def new(msg: String, level: String) -> Diagnostic {
            ^^^~^^^~^^^~^~~~~~~^~^^^^^~^~~~~~~^~^^~^^^^^^^^^^~^ @def@ ws @new@ #(# @msg@ #:# ws @String@ #,# ws @level@ #:# ws @String@ #)# ws #-># ws @Diagnostic@ ws #{#
              Diagnostic { msg, level }
              ^^^^^^^^^^~^~^^^~^~~~~~^~ @Diagnostic@ ws #{# ws @msg@ #,# ws @level@ ws #}#
            }
            ^ #}#
            def main() {
            ^^^~^^^^~^~^ @def@ ws @main@ #(# #)# ws #{#
              let var_name = "variable"
              ^^^~^^^^^^^^~^~^^^^^^^^^^ @let@ ws @var_name@ ws #=# ws "variable"
              let s = "variable is unused" + var_name
              ^^^~^~^~^^^^^^^^^^^^^^^^^^^^~^~^^^^^^^^ @let@ ws @s@ ws #=# ws "variable is unused" ws #+# ws @var_name@
              new(s, "warning")
              ^^^~^~^~~~~~~~~~^ @new@ #(# @s@ #,# ws "warning" #)#
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

        for (i, token) in tokens.clone().iter().enumerate() {
            debug!(target: "lark::reader::test", "{}. {:?}", i, Debuggable::from(token, ann.table()));
        }

        let builtin_macros = macros(ann.table());

        let parser = Reader::new(
            tokens.clone(),
            builtin_macros,
            ann.table().clone(),
            ann.codemap(),
        );

        let result = match parser.process() {
            Ok(result) => {
                debug!(
                    "{:#?}",
                    result.entity_tree.debug(ann.table(), &tokens.clone())
                );
                result
            }
            Err(e) => print_parse_error(e, ann.codemap()),
        };

        let ParseResult {
            entity_tree,
            node_tree: _,
        } = result;

        assert_eq!(
            entity_tree.len(),
            3,
            "There are three entities in the parse"
        );

        assert_eq!(
            entity_tree.str_keys(ann.table()),
            vec!["Diagnostic", "new", "main"]
        );

        let assert = AssertEntities::new(&entity_tree, ann.table(), &tokens);

        assert.entities(&[
            ("Diagnostic", (1, 2), (4, 0)),
            ("new", (5, 2), (7, 0)),
            ("main", (8, 2), (12, 0)),
        ]);
    }

    #[derive(Debug, new)]
    struct AssertEntities<'test> {
        tree: &'test crate::Entities,
        table: &'test crate::intern::ModuleTable,
        tokens: &'test [Spanned<crate::LexToken>],
    }

    impl AssertEntities<'test> {
        fn entities(&self, entities: &[(&str, (usize, usize), (usize, usize))]) {
            assert_eq!(
                self.tree.len(),
                entities.len(),
                "There are {} entities in the parse",
                entities.len()
            );

            assert_eq!(
                self.tree.str_keys(self.table),
                entities.iter().map(|i| i.0).collect::<Vec<_>>(),
                "the entities are as expected"
            );

            for (name, start, end) in entities {
                self.entity(name, *start, *end)
            }
        }

        fn entity(&self, name: &str, start: (usize, usize), end: (usize, usize)) {
            assert_entity(self.tree, name, start, end, self.table, self.tokens)
        }
    }

    fn assert_entity(
        tree: &crate::Entities,
        name: &str,
        start: (usize, usize),
        end: (usize, usize),
        table: &crate::ModuleTable,
        tokens: &[Spanned<crate::LexToken>],
    ) {
        let struct_entity = tree.get_entity_by(table, name);

        assert_eq!(
            struct_entity.start(),
            token_pos_at(start.0, start.1, &tokens),
            "struct start is correct"
        );

        assert_eq!(
            struct_entity.end(),
            token_pos_at(end.0, end.1, &tokens),
            "struct end is correct"
        );
    }
}
