use crate::lexer::definition::LexerState;
use crate::lexer::token::LexToken;
use crate::lexer::tools::Tokenizer;
use crate::macros::type_reference::NamedTypeReference;
use crate::macros::type_reference::ParsedTypeReference;
use crate::macros::EntityMacroDefinition;
use crate::parsed_entity::ErrorParsedEntity;
use crate::parsed_entity::ParsedEntity;
use crate::span::CurrentFile;
use crate::span::Span;
use crate::span::Spanned;
use intern::Intern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::EntityTables;
use lark_error::Diagnostic;
use lark_error::WithError;
use lark_string::global::GlobalIdentifier;
use lark_string::global::GlobalIdentifierTables;
use lark_string::text::Text;
use map::FxIndexMap;
use std::sync::Arc;

crate struct Parser<'me> {
    global_identifier_tables: &'me GlobalIdentifierTables,
    entity_tables: &'me EntityTables,
    entity_macro_definitions: &'me FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>>,
    input: &'me Text,
    tokenizer: Tokenizer<'me, LexerState>,
    errors: Vec<Diagnostic>,

    /// Current lookahead token.
    token: Spanned<LexToken>,

    /// The span of the last token that we consumed (i.e., the one
    /// immediately before `self.token`).
    last_span: Span<CurrentFile>,
}

impl Parser<'me> {
    crate fn new(
        db: &'me (impl AsRef<GlobalIdentifierTables> + AsRef<EntityTables>),
        entity_macro_definitions: &'me FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>>,
        input: &'me Text,
    ) -> Self {
        let mut tokenizer = Tokenizer::new(input);
        let mut errors = vec![];
        let token = next_token(&mut tokenizer, &mut errors, input);
        Parser {
            global_identifier_tables: db.as_ref(),
            entity_tables: db.as_ref(),
            entity_macro_definitions,
            input,
            tokenizer,
            errors,
            last_span: Span::initial(CurrentFile),
            token,
        }
    }

    crate fn parse_all_entities(mut self, parent_entity: Entity) -> WithError<Vec<ParsedEntity>> {
        let mut entities = vec![];
        while let Some(entity) = self.parse_entity(parent_entity) {
            entities.push(entity);
        }

        WithError {
            value: entities,
            errors: self.errors,
        }
    }

    /// Consume the current token and load the next one.
    /// Return the old token.
    crate fn shift(&mut self) -> Spanned<LexToken> {
        self.last_span = self.token.span;
        std::mem::replace(
            &mut self.token,
            next_token(&mut self.tokenizer, &mut self.errors, self.input),
        )
    }

    /// Peek at the current lookahead token.
    crate fn peek(&self) -> Spanned<LexToken> {
        self.token
    }

    /// Span of the current lookahead token.
    crate fn peek_span(&self) -> Span<CurrentFile> {
        self.token.span
    }

    /// Span of the last consumed token.
    crate fn last_span(&self) -> Span<CurrentFile> {
        self.token.span
    }

    /// Peek at the string reprsentation of the current token.
    crate fn peek_str(&self) -> Spanned<&'me str> {
        let text = &self.input[self.token.span];
        Spanned {
            value: text,
            span: self.token.span,
        }
    }

    crate fn eat_global_identifier(&self) -> Option<Spanned<GlobalIdentifier>> {
        if self.is(LexToken::Identifier) {
            Some(self.peek_str().map(|value| value.intern(self)))
        } else {
            None
        }
    }

    crate fn is(&self, kind: LexToken) -> bool {
        kind == self.token.value
    }

    /// Consumes all subsequent newline characters, returning true if
    /// at least one newline was found.
    crate fn eat_newlines(&mut self) -> bool {
        let mut count = 0;
        while self.is(LexToken::Newline) {
            self.shift();
            count += 1;
        }
        count > 0
    }

    crate fn eat_sigil(&mut self, text: &str) -> Option<Spanned<LexToken>> {
        if self.is_sigil(text) {
            Some(self.shift())
        } else {
            None
        }
    }

    crate fn is_sigil(&self, text: &str) -> bool {
        if let LexToken::Sigil = self.token.value {
            &self.input[self.token.span] == text
        } else {
            false
        }
    }

    crate fn parse_type(&mut self) -> Option<ParsedTypeReference> {
        self.eat_global_identifier()
            .map(|identifier| ParsedTypeReference::Named(NamedTypeReference { identifier }))
    }

    crate fn parse_entity(&mut self, parent_entity: Entity) -> Option<ParsedEntity> {
        let macro_name = self.eat_global_identifier()?;
        let macro_definition = match self.entity_macro_definitions.get(&macro_name.value) {
            Some(m) => m.clone(),
            None => {
                // FIXME -- scan end-to-end

                return Some(self.error_entity("no macro with this name", macro_name.span));
            }
        };
        Some(macro_definition.parse(self, parent_entity, macro_name))
    }

    crate fn report_error(&mut self, message: impl Into<String>, span: Span<CurrentFile>) {
        report_error(&mut self.errors, message, span);
    }

    crate fn error_entity(
        &mut self,
        message: impl Into<String>,
        span: Span<CurrentFile>,
    ) -> ParsedEntity {
        let message: String = message.into();
        self.report_error(message.clone(), span);

        let diagnostic = crate::diagnostic(message, span);
        let entity = EntityData::Error(diagnostic.clone()).intern(self.entity_tables);
        ParsedEntity::new(entity, span, span, Arc::new(ErrorParsedEntity))
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

fn next_token(
    tokenizer: &mut Tokenizer<'_, LexerState>,
    errors: &mut Vec<Diagnostic>,
    input: &'me Text,
) -> Spanned<LexToken> {
    loop {
        let new_token = tokenizer.next().unwrap_or_else(|| {
            Ok(Spanned {
                value: LexToken::EOF,
                span: Span::eof(CurrentFile, input),
            })
        });

        // Skip over whitespace/comments automatically (but not
        // newlines).
        match new_token {
            Ok(token) => match token.value {
                LexToken::Whitespace | LexToken::Comment => {
                    continue;
                }

                _ => {
                    return token;
                }
            },

            Err(span) => {
                report_error(errors, "unrecognized token", span);
                continue;
            }
        }
    }
}

fn report_error(errors: &mut Vec<Diagnostic>, message: impl Into<String>, span: Span<CurrentFile>) {
    let message: String = message.into();
    let diagnostic = crate::diagnostic(message, span);
    errors.push(diagnostic);
}
