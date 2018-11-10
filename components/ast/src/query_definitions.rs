use crate::AstDatabase;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::ItemKind;
use lark_entity::MemberKind;
use lark_error::or_return_sentinel;
use lark_error::{Diagnostic, ErrorReported, WithError};
use lark_string::global::GlobalIdentifier;
use parser::ast;
use parser::pos::{HasSpan, Span};
use std::sync::Arc;

crate fn ast_of_file(
    db: &impl AstDatabase,
    path: GlobalIdentifier,
) -> WithError<Result<Arc<ast::Module>, ErrorReported>> {
    let input_text = db.source(path);

    match db.parser_state().parse(input_text.source()) {
        Ok(module) => WithError::ok(Ok(Arc::new(module))),
        Err(parse_error) => {
            let diagnostic = Diagnostic::new(parse_error.description, parse_error.span);
            log::error!("parse error for {}: {:?}", path.debug_with(db), diagnostic);
            WithError {
                value: Err(ErrorReported::at_diagnostic(diagnostic.clone())),
                errors: vec![diagnostic],
            }
        }
    }
}

crate fn items_in_file(db: &impl AstDatabase, input_file: GlobalIdentifier) -> Arc<Vec<Entity>> {
    log::debug!("items_in_file(input_file={})", input_file.debug_with(db));

    let ast_of_file = or_return_sentinel!(db, db.ast_of_file(input_file).into_value());

    log::debug!("items_in_file: ast_of_file={:?}", ast_of_file);

    let input_file_id = EntityData::InputFile { file: input_file }.intern(db);

    log::debug!(
        "items_in_file: input_file_id={}",
        input_file_id.debug_with(db)
    );

    let items: Vec<_> = ast_of_file
        .items
        .iter()
        .map(|item| {
            let kind = match **item {
                ast::Item::Struct(_) => ItemKind::Struct,
                ast::Item::Def(_) => ItemKind::Function,
            };
            EntityData::ItemName {
                base: input_file_id,
                kind,
                id: item.name(),
            }
            .intern(db)
        })
        .collect();

    Arc::new(items)
}

crate fn ast_of_item(
    db: &impl AstDatabase,
    item_id: Entity,
) -> Result<Arc<ast::Item>, ErrorReported> {
    match item_id.untern(db) {
        EntityData::ItemName {
            base,
            kind: _,
            id: path_id,
        } => {
            match base.untern(db) {
                EntityData::InputFile { file: input_file } => {
                    // Base case: root item in a file

                    let module = db.ast_of_file(input_file).into_value()?;

                    for item in &module.items {
                        if item.name() == path_id {
                            return Ok(item.clone());
                        }
                    }

                    panic!("no such item")
                }

                _ => unimplemented!("nested items -- too lazy"),
            }
        }

        d => panic!("ast-of-item invoked with non-item {:?}", d),
    }
}

crate fn ast_of_field(db: &impl AstDatabase, item_id: Entity) -> Result<ast::Field, ErrorReported> {
    match item_id.untern(db) {
        EntityData::MemberName {
            base,
            kind: MemberKind::Field,
            id,
        } => match &*db.ast_of_item(base)? {
            ast::Item::Struct(s) => match s.fields.iter().find(|f| *f.name == id) {
                Some(field) => Ok(field.clone()),

                None => panic!("no such field"),
            },

            ast => panic!("field of invalid entity {:?}", ast),
        },

        EntityData::Error(diagnostic) => Err(ErrorReported::at_diagnostic(diagnostic)),

        d => panic!("ast-of-item invoked with non-field {:?}", d),
    }
}

crate fn entity_span(db: &impl AstDatabase, entity: Entity) -> Option<Span> {
    match entity.untern(db) {
        EntityData::ItemName { .. } => match db.ast_of_item(entity) {
            Ok(ast) => Some(ast.span()),
            Err(err) => Some(err.some_diagnostic().span),
        },

        EntityData::Error(diagnostic) => Some(diagnostic.span),

        EntityData::LangItem(_) => None,

        EntityData::InputFile { file: filename } => {
            let file = db.source(filename);
            Some(file.span())
        }

        EntityData::MemberName {
            kind: MemberKind::Field,
            ..
        } => match db.ast_of_field(entity) {
            Ok(field) => Some(field.span()),
            Err(err) => Some(err.some_diagnostic().span),
        },

        EntityData::MemberName {
            kind: MemberKind::Method,
            ..
        } => unimplemented!("span for a method"),
    }
}
