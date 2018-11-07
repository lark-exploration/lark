#![cfg(test)]

use ast::AstDatabase;
use crate::HirDatabase;
use lark_entity::EntityTables;
use parser::{HasParserState, HasReaderState, ParserState, ReaderState};
use salsa::Database;
use std::sync::Arc;

#[derive(Default)]
struct LarkDatabase {
    runtime: salsa::Runtime<LarkDatabase>,
    parser_state: ParserState,
    reader_state: ReaderState,
    item_id_tables: EntityTables,
    declaration_tables: lark_ty::declaration::DeclarationTables,
}

salsa::database_storage! {
    pub struct LarkDatabaseStorage for LarkDatabase {
        impl parser::ReaderDatabase {
            fn paths_trigger() for parser::PathsTrigger;
            fn paths() for parser::Paths;
            fn source() for parser::Source;
        }
        impl ast::AstDatabase {
            fn ast_of_file() for ast::AstOfFileQuery;
            fn items_in_file() for ast::ItemsInFileQuery;
            fn ast_of_item() for ast::AstOfItemQuery;
            fn ast_of_field() for ast::AstOfFieldQuery;
            fn entity_span() for ast::EntitySpanQuery;
        }
        impl crate::HirDatabase {
            fn fn_body() for crate::FnBodyQuery;
            fn members() for crate::MembersQuery;
            fn member_entity() for crate::MemberEntityQuery;
            fn subentities() for crate::SubentitiesQuery;
            fn ty() for crate::TyQuery;
            fn signature() for crate::SignatureQuery;
            fn generic_declarations() for crate::GenericDeclarationsQuery;
            fn resolve_name() for crate::ResolveNameQuery;
        }
    }
}

impl Database for LarkDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<LarkDatabase> {
        &self.runtime
    }
}

impl AsRef<lark_ty::declaration::DeclarationTables> for LarkDatabase {
    fn as_ref(&self) -> &lark_ty::declaration::DeclarationTables {
        &self.declaration_tables
    }
}

impl HasParserState for LarkDatabase {
    fn parser_state(&self) -> &ParserState {
        &self.parser_state
    }
}

// FIXME: This whole "indirect through `LookupStringId` thing" is a
// workaround for the fact that I don't want to be touching the parser
// module very much right now.
impl parser::LookupStringId for LarkDatabase {
    fn lookup(&self, id: parser::StringId) -> Arc<String> {
        self.parser_state.untern_string(id)
    }
}

impl parser::HasReaderState for LarkDatabase {
    fn reader_state(&self) -> &ReaderState {
        &self.reader_state
    }
}

impl AsRef<EntityTables> for LarkDatabase {
    fn as_ref(&self) -> &EntityTables {
        &self.item_id_tables
    }
}

fn run_test(text: &str, span: &str) {
    let mut db = LarkDatabase::default();

    let path1_str = "path1";
    let path1_interned = db.intern_string("path1");

    db.add_file(path1_str, text);

    let items_in_file = db.items_in_file(path1_interned);
    assert_eq!(items_in_file.len(), 1, "input with more than one item");

    let entity = items_in_file[0];
    let hir_with_errors = db.fn_body(entity);

    assert_eq!(
        hir_with_errors.errors.len(),
        1,
        "input with more than one error"
    );

    // total hack: we know that the byte index will be relative to the
    // start of the string
    let expected_range = format!("{}..{}", span.find('~').unwrap() + 1, span.len() + 1);

    let error_span = hir_with_errors.errors[0].span;
    assert_eq!(expected_range, format!("{:?}", error_span));
}

#[test]
fn bad_identifier() {
    run_test(
        "def new(msg: bool,) -> bool { msg1 }",
        "                              ~~~~",
    );
}
