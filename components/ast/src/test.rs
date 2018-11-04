#![cfg(test)]

use crate::AstDatabase;
use debug::DebugWith;
use lark_entity::EntityTables;
use parking_lot::RwLock;
use parser::{HasParserState, HasReaderState, ParserState, ReaderState};
use salsa::Database;
use std::sync::Arc;

#[derive(Default)]
struct TestDatabaseImpl {
    runtime: salsa::Runtime<TestDatabaseImpl>,
    parser_state: ParserState,
    reader_state: Arc<RwLock<ReaderState>>,
    item_id_tables: EntityTables,
}

salsa::database_storage! {
    pub struct TestDatabaseImplStorage for TestDatabaseImpl {
        impl parser::ReaderDatabase {
            fn paths() for parser::Paths;
            fn source() for parser::Source;
        }
        impl AstDatabase {
            fn ast_of_file() for crate::AstOfFileQuery;
            fn items_in_file() for crate::ItemsInFileQuery;
            fn ast_of_item() for crate::AstOfItemQuery;
            fn ast_of_field() for crate::AstOfFieldQuery;
            fn entity_span() for crate::EntitySpanQuery;
        }
    }
}

impl Database for TestDatabaseImpl {
    fn salsa_runtime(&self) -> &salsa::Runtime<TestDatabaseImpl> {
        &self.runtime
    }
}

impl HasParserState for TestDatabaseImpl {
    fn parser_state(&self) -> &ParserState {
        &self.parser_state
    }
}

// FIXME: This whole "indirect through `LookupStringId` thing" is a
// workaround for the fact that I don't want to be touching the parser
// module very much right now.
impl parser::LookupStringId for TestDatabaseImpl {
    fn lookup(&self, id: parser::StringId) -> Arc<String> {
        self.parser_state.untern_string(id)
    }
}

impl parser::HasReaderState for TestDatabaseImpl {
    fn reader_state(&self) -> &Arc<RwLock<ReaderState>> {
        &self.reader_state
    }
}

impl AsRef<EntityTables> for TestDatabaseImpl {
    fn as_ref(&self) -> &EntityTables {
        &self.item_id_tables
    }
}

#[test]
fn parse_error() {
    let mut db = TestDatabaseImpl::default();
    db.initialize_reader();
    // db.query_mut(parser::Files)
    //     .set((), Arc::new(RwLock::new(SourceFiles::default())));

    let path1 = db.intern_string("path1");
    db.add_file("path1", "XXX");

    assert!(!db.ast_of_file(path1).errors.is_empty());
}

#[test]
fn parse_ok() {
    let mut db = TestDatabaseImpl::default();
    db.initialize_reader();

    let path1_str = "path1";
    let path1_interned = db.intern_string("path1");
    let text1_str = "struct Diagnostic { msg: own String, level: String, }

def new(msg: own String, level: String) -> Diagnostic {
  Diagnostic { mgs, level }
}";

    db.add_file(path1_str, text1_str);

    assert!(
        db.ast_of_file(path1_interned).errors.is_empty(),
        "{:?}",
        db.ast_of_file(path1_interned).errors,
    );

    let items_in_file = db.items_in_file(path1_interned);
    assert_eq!(
        format!("{:#?}", items_in_file.debug_with(&db)),
        r#"[
    ItemName {
        base: InputFile {
            file: "path1"
        },
        kind: Struct,
        id: "Diagnostic"
    },
    ItemName {
        base: InputFile {
            file: "path1"
        },
        kind: Function,
        id: "new"
    }
]"#
    );
}
