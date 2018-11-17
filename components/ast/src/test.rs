#![cfg(test)]

use crate::AstDatabase;
use debug::DebugWith;
use intern::Intern;
use lark_entity::EntityTables;
use lark_parser::{IntoFileName, ParserDatabase, ParserDatabaseExt};
use lark_seq::Seq;
use lark_span::FileName;
use lark_string::GlobalIdentifierTables;
use salsa::Database;

#[derive(Default)]
struct TestDatabaseImpl {
    runtime: salsa::Runtime<TestDatabaseImpl>,
    string_id_tables: GlobalIdentifierTables,
    item_id_tables: EntityTables,
}

impl TestDatabaseImpl {
    pub fn new() -> TestDatabaseImpl {
        let mut db = TestDatabaseImpl::default();
        db.query_mut(lark_parser::FileNamesQuery)
            .set((), Seq::default());
        db
    }
}

salsa::database_storage! {
    pub struct TestDatabaseImplStorage for TestDatabaseImpl {
        impl ParserDatabase {
            fn file_names() for lark_parser::FileNamesQuery;
            fn file_text() for lark_parser::FileTextQuery;
            fn line_offsets() for lark_parser::LineOffsetsQuery;
            fn location() for lark_parser::LocationQuery;
            fn byte_index() for lark_parser::ByteIndexQuery;
            fn file_tokens() for lark_parser::FileTokensQuery;
            fn parsed_file() for lark_parser::ParsedFileQuery;
            fn child_parsed_entities() for lark_parser::ChildParsedEntitiesQuery;
            fn parsed_entity() for lark_parser::ParsedEntityQuery;
            fn child_entities() for lark_parser::ChildEntitiesQuery;
            fn uhir_of_entity() for lark_parser::UhirOfEntityQuery;
            fn uhir_of_field() for lark_parser::UhirOfFieldQuery;
        }
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

impl ParserDatabaseExt for TestDatabaseImpl {}

impl AsRef<GlobalIdentifierTables> for TestDatabaseImpl {
    fn as_ref(&self) -> &GlobalIdentifierTables {
        (&self.string_id_tables).as_ref()
    }
}

impl AsRef<EntityTables> for TestDatabaseImpl {
    fn as_ref(&self) -> &EntityTables {
        &self.item_id_tables
    }
}

#[test]
fn parse_error() {
    let mut db = TestDatabaseImpl::new();
    // db.query_mut(parser::Files)
    //     .set((), Arc::new(RwLock::new(SourceFiles::default())));

    let path1 = "path1".into_file_name(&db);
    db.add_file(path1, "XXX");

    assert!(!db.parsed_file(path1).errors.is_empty());
}

#[test]
fn parse_ok() {
    let mut db = TestDatabaseImpl::new();

    let file_name = FileName {
        id: "path1".intern(&db),
    };
    let text1_str = "struct Diagnostic { msg: own String, level: String, }

def new(msg: own String, level: String) -> Diagnostic {
  Diagnostic { mgs, level }
}";

    db.add_file(file_name, text1_str);

    assert!(
        db.parsed_file(file_name).errors.is_empty(),
        "{:?}",
        db.parsed_file(file_name).errors,
    );

    let parsed_file = db.parsed_file(file_name);
    assert_eq!(
        format!("{:#?}", parsed_file.value.debug_with(&db)),
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
