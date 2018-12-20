use crate::lexer::token::LexToken;
use crate::macros::EntityMacroDefinition;
use crate::syntax::NonEmptySyntax;
use crate::syntax::Syntax;

use lark_collections::{FxIndexMap, Seq};
use lark_debug_with::DebugWith;
use lark_entity::EntityTables;
use lark_error::{Diagnostic, ErrorReported, WithError};
use lark_span::{FileName, Span, Spanned};
use lark_string::{GlobalIdentifier, GlobalIdentifierTables, Text};
use std::sync::Arc;

pub struct Parser<'parse> {
    /// The source file name for the file being parsed; used in error reporting
    file_name: FileName,

    /// Tables for interning global identifiers; extracted from the database.
    global_identifier_tables: &'parse GlobalIdentifierTables,

    /// Tables for interning entities; extracted from the database.
    entity_tables: &'parse EntityTables,

    /// Set of macro definitions in scope.
    entity_macro_definitions: &'parse FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>>,

    /// Complete input; needed to extract the full text of tokens.
    input: &'parse Text,

    /// List of all tokens.
    tokens: &'parse Seq<Spanned<LexToken, FileName>>,

    /// Index of the token *after* the current token.
    next_lookahead_token: usize,

    /// Span of the last consumed token (ignoring whitespace and
    /// comments); see the `last_span()` method below.
    last_span: Span<FileName>,

    /// Current lookahead token.
    lookahead_token: Spanned<LexToken, FileName>,

    /// Errors reported during parsing; these will be converted into
    /// the final `WithError` result
    errors: Vec<Diagnostic>,
}

impl Parser<'parse> {
    crate fn new(
        file_name: FileName,
        db: &'parse (impl AsRef<GlobalIdentifierTables> + AsRef<EntityTables> + ?Sized),
        entity_macro_definitions: &'parse FxIndexMap<
            GlobalIdentifier,
            Arc<dyn EntityMacroDefinition>,
        >,
        input: &'parse Text,
        tokens: &'parse Seq<Spanned<LexToken, FileName>>,
        start_token: usize,
    ) -> Self {
        // Subtle: the start token may be whitespace etc. So we actually have to invoke
        // `advance_next_token` to advance.
        let mut next_lookahead_token = start_token;
        let lookahead_token =
            advance_next_token(input, tokens, &mut next_lookahead_token, file_name);

        Parser {
            file_name,
            global_identifier_tables: db.as_ref(),
            entity_tables: db.as_ref(),
            entity_macro_definitions,
            input,
            tokens,
            next_lookahead_token,
            lookahead_token,
            errors: vec![],
            last_span: Span::initial(file_name),
        }
    }

    /// Clones the parser to produce a "checkpoint". You can go on
    /// using this checkpoint, but any changes to the current token
    /// (as well as any reported errors!) will be ignored and will not
    /// affect the main parser. This is intended to enable "limited
    /// lookahead" of more than one token, e.g. skipping upcoming
    /// newlines.
    crate fn checkpoint(&self) -> Self {
        Parser {
            errors: vec![],
            ..*self
        }
    }

    /// Parse all the instances of `syntax` that we can, stopping only
    /// at EOF. Returns a vector of the results plus any parse errors
    /// we encountered.
    crate fn parse_until_eof<S>(mut self, mut syntax: S) -> WithError<Seq<S::Data>>
    where
        S: NonEmptySyntax<'parse>,
    {
        let mut entities = vec![];
        loop {
            self.skip_newlines();

            if self.is(LexToken::EOF) {
                break;
            }

            if self.test(&mut syntax) {
                match self.expect(&mut syntax) {
                    Ok(e) => entities.push(e),
                    Err(ErrorReported(_)) => (),
                }
            } else {
                let Spanned { span, .. } = self.shift();
                self.report_error("unexpected character", span);
            }
        }

        self.into_with_error(Seq::from(entities))
    }

    crate fn into_with_error<T>(self, value: T) -> WithError<T> {
        WithError {
            value,
            errors: self.errors,
        }
    }

    /// Consume the current token and load the next one.  Return the
    /// old token.
    crate fn shift(&mut self) -> Spanned<LexToken, FileName> {
        assert!(!self.is(LexToken::EOF));

        self.last_span = self.lookahead_token.span;
        let last_token = self.lookahead_token;

        self.lookahead_token = advance_next_token(
            self.input,
            self.tokens,
            &mut self.next_lookahead_token,
            self.file_name,
        );

        log::trace!(
            "shift: new lookahead token = {}, consumed token = {}",
            self.lookahead_token.debug_with(self),
            last_token.debug_with(self),
        );

        last_token
    }

    /// Extract the complete input
    crate fn input(&self) -> &'parse Text {
        self.input
    }

    /// Extract the complete input
    crate fn entity_macro_definitions(
        &self,
    ) -> &'parse FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>> {
        self.entity_macro_definitions
    }

    /// Peek at the index of the current lookahead token in the token
    /// list. If this is the EOF token, then the index returned is the
    /// length of the token list.
    crate fn peek_index(&self) -> usize {
        self.next_lookahead_token - 1
    }

    /// Peek at the current lookahead token.
    crate fn peek(&self) -> Spanned<LexToken, FileName> {
        self.lookahead_token
    }

    /// Span covering the space *in between* the previous token
    /// and the current token. This is the span where something
    /// elided would go.
    crate fn elided_span(&self) -> Span<FileName> {
        // FIXME -- what should we do regarding whitespace etc?
        Span::new(
            self.file_name,
            self.last_span.end(),
            self.peek_span().start(),
        )
    }

    /// Span of the current lookahead token.
    crate fn peek_span(&self) -> Span<FileName> {
        self.peek().span
    }

    /// Span of the last consumed token, ignoring whitespace and
    /// comments. This is very handy when constructing the span of
    /// things we are looking at.  You basically consume tokens until
    /// the lookahead tells you that you are at the end, and then you
    /// can look at the `last_span`
    crate fn last_span(&self) -> Span<FileName> {
        self.last_span
    }

    /// Peek at the string reprsentation of the current token.
    crate fn peek_str(&self) -> &'parse str {
        &self.input[self.peek_span()]
    }

    /// Test if the current token is of the given kind.
    crate fn is(&self, kind: LexToken) -> bool {
        kind == self.lookahead_token.value
    }

    /// If at EOF, returns `None`. Otherwise, shifts all remaining
    /// tokens out and returns the span that covers them.
    crate fn parse_extra_input(&mut self) -> Option<Span<FileName>> {
        if self.is(LexToken::EOF) {
            return None;
        }

        let start = self.shift();
        while !self.is(LexToken::EOF) {
            self.shift();
        }
        let span = start.span.extended_until_end_of(self.peek_span());
        return Some(span);
    }

    /// Consumes all subsequent newline characters, returning true if
    /// at least one newline was found.
    crate fn skip_newlines(&mut self) -> bool {
        let mut count = 0;
        while self.is(LexToken::Newline) {
            self.shift();
            count += 1;
        }
        count > 0
    }

    /// Tests whether the syntax applies at the current point.
    crate fn test(&self, mut syntax: impl Syntax<'parse>) -> bool {
        log::trace!(
            "test({}) at token `{}({})`",
            syntax.debug_with(self),
            self.lookahead_token.value.debug_with(self),
            self.peek_str().debug_with(self),
        );

        if syntax.test(self) {
            log::trace!("test: passed");
            true
        } else {
            false
        }
    }

    /// Parses a `T` if we can and returns true if so; otherwise,
    /// reports an error and returns false.
    crate fn expect<T>(&'s mut self, mut syntax: T) -> Result<T::Data, ErrorReported>
    where
        T: Syntax<'parse>,
    {
        log::debug!(
            "expect({}) at token `{}({})`",
            syntax.debug_with(self),
            self.lookahead_token.value.debug_with(self),
            self.peek_str().debug_with(self),
        );

        syntax.expect(self)
    }

    /// Parse a piece of syntax (if it is present), otherwise returns
    /// `None`. A combination of `test` and `expect`.
    crate fn parse_if_present<T>(&mut self, mut syntax: T) -> Option<Result<T::Data, ErrorReported>>
    where
        T: Syntax<'parse>,
    {
        log::trace!(
            "eat({}) at token `{}({})`",
            syntax.debug_with(self),
            self.lookahead_token.value.debug_with(self),
            self.peek_str().debug_with(self),
        );

        if self.test(&mut syntax) {
            Some(self.expect(syntax))
        } else {
            None
        }
    }

    /// Report an error with the given message at the given span.
    crate fn report_error(
        &mut self,
        message: impl Into<String>,
        span: Span<FileName>,
    ) -> ErrorReported {
        report_error(&mut self.errors, message, span)
    }
}

impl AsRef<GlobalIdentifierTables> for Parser<'_> {
    fn as_ref(&self) -> &GlobalIdentifierTables {
        self.global_identifier_tables
    }
}

impl AsRef<EntityTables> for Parser<'_> {
    fn as_ref(&self) -> &EntityTables {
        self.entity_tables
    }
}

fn advance_next_token(
    input: &Text,
    tokens: &[Spanned<LexToken, FileName>],
    next_token: &mut usize,
    file_name: FileName,
) -> Spanned<LexToken, FileName> {
    loop {
        if *next_token >= tokens.len() {
            *next_token = tokens.len() + 1;
            return Spanned {
                value: LexToken::EOF,
                span: Span::eof(file_name, input),
            };
        }

        let token = tokens[*next_token];

        // Advance to the next token, unless we are at EOF.
        *next_token += 1;

        // Skip over whitespace/comments automatically (but not
        // newlines).
        match token.value {
            LexToken::Whitespace | LexToken::Comment => continue,
            _ => return token,
        }
    }
}

fn report_error(
    errors: &mut Vec<Diagnostic>,
    message: impl Into<String>,
    span: Span<FileName>,
) -> ErrorReported {
    let message: String = message.into();
    let diagnostic = crate::diagnostic(message, span);
    errors.push(diagnostic);
    ErrorReported::at_diagnostic(errors.last().unwrap())
}
