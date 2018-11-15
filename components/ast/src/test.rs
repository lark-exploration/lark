#![cfg(test)]

use crate::AstDatabase;
use debug::DebugWith;
use lark_entity::EntityTables;
use lark_string::global::GlobalIdentifierTables;
use salsa::Database;

#[derive(Default)]
struct TestDatabaseImpl {
    runtime: salsa::Runtime<TestDatabaseImpl>,
    item_id_tables: EntityTables,
}

salsa::database_storage! {
    pub struct TestDatabaseImplStorage for TestDatabaseImpl {
        impl AstDatabase {
            fn entity_span() for crate::EntitySpanQuery;
        }
    }
}

impl Database for TestDatabaseImpl {
    fn salsa_runtime(&self) -> &salsa::Runtime<TestDatabaseImpl> {
        &self.runtime
    }
}

impl AsRef<GlobalIdentifierTables> for TestDatabaseImpl {
    fn as_ref(&self) -> &GlobalIdentifierTables {
        (&self.parser_state).as_ref()
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
    // db.query_mut(parser::Files)
    //     .set((), Arc::new(RwLock::new(SourceFiles::default())));

    let path1 = db.intern_string("path1");
    db.add_file("path1", "XXX");

    assert!(!db.ast_of_file(path1).errors.is_empty());
}

#[test]
fn parse_ok() {
    let mut db = TestDatabaseImpl::default();

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
