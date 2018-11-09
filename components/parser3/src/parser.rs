use crate::lexer::definition::LexerState;
use crate::lexer::token::LexToken;
use crate::lexer::tools::Tokenizer;
use crate::macros::MacroDefinition;
use crate::parsed_entity::ErrorParsedEntity;
use crate::parsed_entity::ParsedEntity;
use crate::span::CurrentFile;
use crate::span::Span;
use intern::Intern;
use lark_entity::EntityData;
use lark_entity::EntityTables;
use lark_error::WithError;
use lark_string::global::GlobalIdentifier;
use lark_string::global::GlobalIdentifierTables;
use lark_string::text::Text;
use map::FxIndexMap;
use std::sync::Arc;

crate struct Parser<'me> {
    global_identifier_tables: &'me GlobalIdentifierTables,
    entity_tables: &'me EntityTables,
    macro_definitions: &'me FxIndexMap<GlobalIdentifier, Arc<dyn MacroDefinition>>,
    input: &'me Text,
    tokenizer: Tokenizer<'me, LexerState>,
}

impl Parser<'me> {
    crate fn new(
        db: &'me (impl AsRef<GlobalIdentifierTables> + AsRef<EntityTables>),
        macro_definitions: &'me FxIndexMap<GlobalIdentifier, Arc<dyn MacroDefinition>>,
        input: &'me Text,
    ) -> Self {
        let tokenizer = Tokenizer::new(input);
        Parser {
            global_identifier_tables: db.as_ref(),
            entity_tables: db.as_ref(),
            macro_definitions,
            input,
            tokenizer,
        }
    }

    crate fn parse_all_entities(&mut self) -> WithError<Vec<ParsedEntity>> {
        let mut entities = vec![];
        let mut errors = vec![];
        while let Some(entity) = self.parse_entity() {
            entities.push(entity.accumulate_errors_into(&mut errors));
        }
        WithError {
            value: entities,
            errors,
        }
    }

    fn parse_entity(&mut self) -> Option<WithError<ParsedEntity>> {
        loop {
            match self.tokenizer.next()? {
                Ok(token) => match token.value {
                    LexToken::Identifier => {
                        let global_id =
                            self.input[token.span].intern(self.global_identifier_tables);

                        let macro_definition = match self.macro_definitions.get(&global_id) {
                            Some(m) => m.clone(),
                            None => {
                                // FIXME -- scan end-to-end

                                return self.error_entity(
                                    format!("no macro named `{}`", &self.input[token.span]),
                                    token.span,
                                );
                            }
                        };

                        return Some(macro_definition.parse(self));
                    }

                    // Skip whitespace, newlines and comments.
                    LexToken::Whitespace | LexToken::Newline | LexToken::Comment => continue,

                    // Things that can't start entities.
                    LexToken::EOF | LexToken::String | LexToken::Sigil => {
                        return self.error_entity(format!("unexpeced token"), token.span);
                    }
                },

                Err(span) => {
                    return self.error_entity(format!("unexpected token"), span);
                }
            }
        }
    }

    fn error_entity(
        &self,
        message: String,
        span: Span<CurrentFile>,
    ) -> Option<WithError<ParsedEntity>> {
        let diagnostic = crate::diagnostic(message, span);
        let entity = EntityData::Error(diagnostic.clone()).intern(self.entity_tables);

        Some(WithError {
            value: ParsedEntity::new(entity, span, Arc::new(ErrorParsedEntity)),

            errors: vec![diagnostic],
        })
    }
}
