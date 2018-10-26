use crate::AstDatabase;
use intern::Intern;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::ItemKind;
use lark_entity::MemberKind;
use lark_error::or_return_sentinel;
use lark_error::{ErrorReported, WithError};
use parser::ast;
use parser::pos::{HasSpan, Span};
use parser::StringId;
use std::sync::Arc;

crate fn ast_of_file(
    db: &impl AstDatabase,
    path: StringId,
) -> WithError<Result<Arc<ast::Module>, ErrorReported>> {
    let input_text = db.input_text(path).unwrap_or_else(|| {
        panic!("no input text for path `{}`", db.untern_string(path));
    });

    match db.parser_state().parse(path, &input_text) {
        Ok(module) => WithError::ok(Ok(Arc::new(module))),
        Err(parse_error) => WithError {
            value: Err(ErrorReported::at_span(parse_error.span)),
            errors: vec![parse_error.span],
        },
    }
}

crate fn items_in_file(db: &impl AstDatabase, input_file: StringId) -> Arc<Vec<Entity>> {
    let ast_of_file = or_return_sentinel!(db, db.ast_of_file(input_file).into_value());

    let input_file_id = EntityData::InputFile { file: input_file }.intern(db);

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

        EntityData::Error(span) => Err(ErrorReported::at_span(span)),

        d => panic!("ast-of-item invoked with non-field {:?}", d),
    }
}

crate fn entity_span(db: &impl AstDatabase, entity: Entity) -> Option<Span> {
    match entity.untern(db) {
        EntityData::ItemName { .. } => match db.ast_of_item(entity) {
            Ok(ast) => Some(ast.span()),
            Err(err) => Some(err.some_span()),
        },

        EntityData::Error(span) => Some(span),

        EntityData::LangItem(_) => None,

        EntityData::InputFile { .. } => unimplemented!("span for an input file"),

        EntityData::MemberName {
            kind: MemberKind::Field,
            ..
        } => match db.ast_of_field(entity) {
            Ok(field) => Some(field.span()),
            Err(err) => Some(err.some_span()),
        },

        EntityData::MemberName {
            kind: MemberKind::Method,
            ..
        } => unimplemented!("span for a method"),
    }
}
