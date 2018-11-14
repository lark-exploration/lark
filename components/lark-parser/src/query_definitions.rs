use crate::lexer::definition::LexerState;
use crate::lexer::token::LexToken;
use crate::lexer::tools::Tokenizer;
use crate::macros;
use crate::parser::Parser;
use crate::syntax::entity::EntitySyntax;
use crate::syntax::entity::ParsedEntity;
use crate::syntax::skip_newline::SkipNewline;
use crate::FileName;
use crate::ParserDatabase;

use debug::DebugWith;
use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData};
use lark_error::WithError;
use lark_seq::Seq;
use lark_span::Spanned;

crate fn file_tokens(
    db: &impl ParserDatabase,
    file_name: FileName,
) -> WithError<Seq<Spanned<LexToken>>> {
    let input = db.file_text(file_name);
    let mut tokenizer: Tokenizer<'_, LexerState> = Tokenizer::new(&input);
    let mut errors = vec![];
    let mut tokens = vec![];
    while let Some(token) = tokenizer.next() {
        match token {
            Ok(t) => tokens.push(t),
            Err(span) => errors.push(crate::diagnostic("unrecognized token", span)),
        }
    }

    // Note: the EOF token is constructed "on the fly" by the parser
    // when the end of the current sequence of tokens is reached.

    WithError {
        value: Seq::from(tokens),
        errors,
    }
}

crate fn child_parsed_entities(
    db: &impl ParserDatabase,
    entity: Entity,
) -> WithError<Seq<ParsedEntity>> {
    log::debug!("child_parsed_entities({})", entity.debug_with(db));

    match entity.untern(db) {
        EntityData::InputFile { file } => {
            let file_name = FileName { id: file };
            let entity_macro_definitions = &macros::default_entity_macros(db);
            let input = &db.file_text(file_name);
            let tokens = &db.file_tokens(file_name).into_value();
            let parser = Parser::new(db, entity_macro_definitions, input, tokens, 0);
            let file_entity = EntityData::InputFile { file: file_name.id }.intern(db);
            parser.parse_until_eof(SkipNewline(EntitySyntax::new(file_entity)))
        }

        EntityData::ItemName { .. } => db
            .parsed_entity(entity)
            .thunk
            .parse_children(entity, db)
            .map(Seq::from),

        EntityData::Error { .. } | EntityData::MemberName { .. } | EntityData::LangItem(_) => {
            WithError::ok(Seq::default())
        }
    }
}

crate fn parsed_entity(db: &impl ParserDatabase, entity: Entity) -> ParsedEntity {
    match entity.untern(db) {
        EntityData::ItemName { base, .. } => {
            let siblings = db.child_parsed_entities(base).into_value();
            siblings
                .iter()
                .find(|p| p.entity == entity)
                .unwrap_or_else(|| {
                    panic!(
                        "parsed_entity({}): entity not found amongst its siblings `{:?}`",
                        entity.debug_with(db),
                        siblings.debug_with(db),
                    )
                })
                .clone()
        }

        EntityData::Error { .. }
        | EntityData::InputFile { .. }
        | EntityData::MemberName { .. }
        | EntityData::LangItem(_) => {
            panic!(
                "cannot compute: `parsed_entity({:?})`",
                entity.debug_with(db),
            );
        }
    }
}

crate fn child_entities(db: &impl ParserDatabase, entity: Entity) -> Seq<Entity> {
    db.child_parsed_entities(entity)
        .into_value()
        .iter()
        .map(|parsed_entity| parsed_entity.entity)
        .collect()
}
