use crate::query::files::{File, SourceFiles};
use crate::StringId;

use std::sync::Arc;

salsa::query_group! {
    pub trait ReaderDatabase: crate::HasParserState + salsa::Database {
        fn files() -> Arc<SourceFiles> {
            type Files;
            storage input;
        }

        fn paths() -> Arc<Vec<StringId>> {
            type Paths;
        }

        fn source(key: StringId) -> Arc<File> {
            type Source;
        }
    }
}

fn paths(db: &impl ReaderDatabase) -> Arc<Vec<StringId>> {
    let files = db.files();
    Arc::new(files.paths())
}

fn source(db: &impl ReaderDatabase, key: StringId) -> Arc<File> {
    let files = db.files();
    files.get(&key).unwrap_or_else(|| {
        panic!("no input text for path `{}`", db.untern_string(key));
    })
}
