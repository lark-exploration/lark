#![cfg(test)]

use crate::AstDatabase;
use crate::AstOfFile;
use crate::AstOfItem;
use crate::HasParserState;
use crate::InputFiles;
use crate::InputText;
use crate::ItemsInFile;
use crate::ParserState;
use debug::DebugWith;
use lark_entity::EntityTables;
use salsa::Database;
use std::sync::Arc;

#[derive(Default)]
struct TestDatabaseImpl {
    runtime: salsa::Runtime<TestDatabaseImpl>,
    parser_state: ParserState,
    item_id_tables: EntityTables,
}

salsa::database_storage! {
    pub struct TestDatabaseImplStorage for TestDatabaseImpl {
        impl AstDatabase {
            fn input_files() for InputFiles;
            fn input_text() for InputText;
            fn ast_of_file() for AstOfFile;
            fn items_in_file() for ItemsInFile;
            fn ast_of_item() for AstOfItem;
        }
    }
}

impl Database for TestDatabaseImpl {
    fn salsa_runtime(&self) -> &salsa::Runtime<TestDatabaseImpl> {
        &self.runtime
    }
}

impl crate::HasParserState for TestDatabaseImpl {
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

impl AsRef<EntityTables> for TestDatabaseImpl {
    fn as_ref(&self) -> &EntityTables {
        &self.item_id_tables
    }
}

#[test]
fn parse_error() {
    let db = TestDatabaseImpl::default();

    let path1 = db.intern_string("path1");
    db.query(InputFiles).set((), Arc::new(vec![path1]));
    let text1 = db.intern_string("XXX");
    db.query(InputText).set(path1, Some(text1));

    assert!(db.ast_of_file(path1).is_err());
}

#[test]
fn parse_ok() {
    let db = TestDatabaseImpl::default();

    let path1 = db.intern_string("path1");
    db.query(InputFiles).set((), Arc::new(vec![path1]));
    let text1 = db.intern_string(
        "struct Diagnostic {
  msg: own String,
  level: String,
}

def new(msg: own String, level: String) -> Diagnostic {
  Diagnostic { mgs, level }
}",
    );
    db.query(InputText).set(path1, Some(text1));

    assert!(
        db.ast_of_file(path1).is_ok(),
        "{:?}",
        db.ast_of_file(path1).unwrap_err(),
    );

    let items_in_file = db.items_in_file(path1);
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
