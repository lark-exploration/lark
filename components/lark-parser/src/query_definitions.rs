use crate::ir::ParsedFile;
use crate::lexer::definition::LexerState;
use crate::lexer::token::LexToken;
use crate::lexer::tools::Tokenizer;
use crate::parser::Parser;
use crate::syntax::entity::{EntitySyntax, ParsedEntity, ParsedEntityThunk};
use crate::syntax::skip_newline::SkipNewline;
use crate::HoverTarget;
use crate::HoverTargetKind;
use crate::ParserDatabase;

use lark_collections::Seq;
use lark_debug_with::DebugWith;
use lark_entity::MemberKind;
use lark_entity::{Entity, EntityData};
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_error::WithError;
use lark_hir as hir;
use lark_intern::{Intern, Untern};
use lark_span::{ByteIndex, FileName, Location, Span, Spanned};
use lark_string::GlobalIdentifier;
use std::sync::Arc;

crate fn file_tokens(
    db: &impl ParserDatabase,
    file_name: FileName,
) -> WithError<Seq<Spanned<LexToken, FileName>>> {
    let input = db.file_text(file_name);
    let mut tokenizer: Tokenizer<'_, LexerState> = Tokenizer::new(&input);
    let mut errors = vec![];
    let mut tokens = vec![];
    while let Some(token) = tokenizer.next() {
        match token {
            Ok(t) => tokens.push(t.in_file_named(file_name)),
            Err(span) => errors.push(crate::diagnostic(
                "unrecognized token",
                span.in_file_named(file_name),
            )),
        }
    }

    // Note: the EOF token is constructed "on the fly" by the parser
    // when the end of the current sequence of tokens is reached.

    WithError {
        value: Seq::from(tokens),
        errors,
    }
}

crate fn parsed_file(db: &impl ParserDatabase, file_name: FileName) -> WithError<ParsedFile> {
    log::debug!("parsed_file({})", file_name.debug_with(db));

    let file_entity = EntityData::InputFile { file: file_name }.intern(db);
    let entity_macro_definitions = crate::macro_definitions(&db, file_entity);
    let input = &db.file_text(file_name);
    let tokens = &db.file_tokens(file_name).into_value();
    let parser = Parser::new(file_name, db, &entity_macro_definitions, input, tokens, 0);
    parser
        .parse_until_eof(SkipNewline(EntitySyntax::new(file_entity)))
        .map(|entities| ParsedFile::new(file_name, entities, Span::new(file_name, 0, input.len())))
}

crate fn child_parsed_entities(
    db: &impl ParserDatabase,
    entity: Entity,
) -> WithError<Seq<ParsedEntity>> {
    log::debug!("child_parsed_entities({})", entity.debug_with(db));

    match entity.untern(db) {
        EntityData::InputFile { file } => WithError::ok(db.parsed_file(file).into_value().entities),

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
        EntityData::InputFile { file } => {
            let parsed_file = db.parsed_file(file).into_value();
            ParsedEntity {
                entity: entity,
                full_span: parsed_file.span,
                characteristic_span: parsed_file.span,
                thunk: ParsedEntityThunk::new(parsed_file),
            }
        }

        EntityData::ItemName { base, .. } | EntityData::MemberName { base, .. } => {
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

        EntityData::Error { .. } | EntityData::LangItem(_) => {
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

crate fn fn_body(db: &impl ParserDatabase, entity: Entity) -> WithError<Arc<hir::FnBody>> {
    db.parsed_entity(entity)
        .thunk
        .parse_fn_body(entity, db)
        .map(Arc::new)
}

crate fn entity_span(db: &impl ParserDatabase, entity: Entity) -> Span<FileName> {
    db.parsed_entity(entity).full_span.in_file_named(
        entity
            .input_file(db)
            .expect("Unexpected entity_span for LangItem or Error"),
    )
}

crate fn characteristic_entity_span(db: &impl ParserDatabase, entity: Entity) -> Span<FileName> {
    db.parsed_entity(entity).characteristic_span.in_file_named(
        entity
            .input_file(db)
            .expect("Unexpected entity_span for LangItem or Error"),
    )
}

crate fn line_offsets(db: &impl ParserDatabase, id: FileName) -> Seq<usize> {
    let text: &str = &db.file_text(id);
    let mut accumulator = 0;
    text.lines()
        .map(|line_text| {
            let line_start = accumulator;
            accumulator += line_text.len();
            if text[accumulator..].starts_with("\r\n") {
                accumulator += 2;
            } else if text[accumulator..].starts_with("\n") {
                accumulator += 1;
            }
            line_start
        })
        .chain(std::iter::once(text.len()))
        .collect()
}

crate fn location(db: &impl ParserDatabase, id: FileName, index: ByteIndex) -> Location {
    let line_offsets = db.line_offsets(id);
    match line_offsets.binary_search(&index.to_usize()) {
        Ok(line) => {
            // Found the start of a line directly:
            return Location::new(line, 0, index);
        }
        Err(next_line) => {
            let line = next_line - 1;

            // Found something in the middle.
            let line_start = line_offsets[line];

            // count utf-8 characters to find column
            let text: &str = &db.file_text(id);
            let column = text[line_start..index.to_usize()].chars().count();

            Location::new(line, column, index)
        }
    }
}

crate fn byte_index(db: &impl ParserDatabase, id: FileName, line: u64, column: u64) -> ByteIndex {
    let line = line as usize;
    let column = column as usize;
    let line_offsets = db.line_offsets(id);
    let line_start = line_offsets[line];
    ByteIndex::from(line_start + column)
}

crate fn descendant_entities(db: &impl ParserDatabase, root: Entity) -> Seq<Entity> {
    let mut entities = vec![root];

    // Go over each thing added to entities and add any nested
    // entities.
    let mut index = 0;
    while let Some(&entity) = entities.get(index) {
        index += 1;
        entities.extend(db.child_entities(entity).iter());
    }

    Seq::from(entities)
}

crate fn members(
    db: &impl ParserDatabase,
    owner: Entity,
) -> Result<Seq<hir::Member>, ErrorReported> {
    // Really this query should perhaps go away, or else maybe be
    // redirected to the `ParsedEntity` -- e.g., one goal was to allow
    // us to know when there were errors in the field list (i.e., by
    // returning `Err`) to suppress derived errors.  This setup won't
    // permit that.
    Ok(db
        .child_entities(owner)
        .iter()
        .cloned()
        .filter_map(|child_entity| match child_entity.untern(db) {
            EntityData::MemberName { id, kind, .. } => Some(hir::Member {
                name: id,
                kind,
                entity: child_entity,
            }),

            _ => None,
        })
        .collect())
}

crate fn member_entity(
    db: &impl ParserDatabase,
    owner: Entity,
    kind: MemberKind,
    name: GlobalIdentifier,
) -> Option<Entity> {
    match db.members(owner) {
        Err(report) => Some(Entity::error_sentinel(db, report)),

        Ok(members) => members
            .iter()
            .filter_map(|member| {
                if member.kind == kind && member.name == name {
                    Some(member.entity)
                } else {
                    None
                }
            })
            .next(),
    }
}

crate fn hover_targets(
    db: &impl ParserDatabase,
    file: FileName,
    index: ByteIndex,
) -> Seq<HoverTarget> {
    let file_entity = EntityData::InputFile { file }.intern(db);

    let mut targets: Vec<_> = db
        .descendant_entities(file_entity)
        .iter()
        .flat_map(|&entity| {
            let entity_span = db.entity_span(entity);

            if !entity_span.contains_index(index) {
                return vec![];
            }

            let mut targets = vec![HoverTarget {
                span: entity_span,
                kind: HoverTargetKind::Entity(entity),
            }];

            if entity.untern(db).has_fn_body() {
                let fn_body = db.fn_body(entity).into_value();
                targets.extend(fn_body.tables.spans.iter().filter_map(|(&mi, &mi_span)| {
                    if mi_span.contains_index(index) {
                        Some(HoverTarget {
                            span: mi_span,
                            kind: HoverTargetKind::MetaIndex(entity, mi),
                        })
                    } else {
                        None
                    }
                }));
            }

            targets
        })
        .collect();

    // If we assume that all the targets contain one another,
    // then sorting by their *start spans* first (and inversely by
    // *end spans* in case of ties...)  should give in
    // "outermost-to-innermost" order.
    //
    // Example:
    //
    // foo { bar { } }
    //       ^^^       2
    //       ^^^^^^^   1
    // ^^^^^^^^^^^^^^^ 0
    targets.sort_by_key(|target| {
        let start = target.span.start();
        let end = std::usize::MAX - target.span.end().to_usize();
        (start, end)
    });

    assert!(!targets.is_empty());
    Seq::from(targets)
}
