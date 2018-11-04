use crate::query::files::{File, SourceFiles};
use crate::StringId;

use map::FxIndexSet;
use std::sync::{Arc, RwLock};

salsa::query_group! {
    pub trait ReaderDatabase: crate::HasParserState + salsa::Database {
        fn files() -> Arc<RwLock<SourceFiles>> {
            type Files;
            storage input;
        }

        fn paths() -> Arc<FxIndexSet<StringId>> {
            type Paths;
            storage input;
        }

        fn source(key: StringId) -> Arc<File> {
            type Source;
        }
    }
}

pub fn initialize_reader(db: &mut impl ReaderDatabase) {
    db.query_mut(Files)
        .set((), Arc::new(RwLock::new(SourceFiles::default())));
}

pub fn add_file(db: &mut impl ReaderDatabase, path: &str, source: impl Into<String>) {
    let path_id = db.intern_string(path);
    let source = source.into();

    let files = db.files();

    let mut paths = (*db.paths()).clone();
    paths.insert(path_id);
    db.query_mut(Paths).set((), Arc::new(paths));

    files
        .write()
        .unwrap()
        .insert(&path_id, codespan::FileName::Real(path.into()), source);
}

fn source(db: &impl ReaderDatabase, key: StringId) -> Arc<File> {
    let files = db.files();
    let files = files.read().unwrap();
    files.find(&key).unwrap_or_else(|| {
        panic!("no input text for path `{}`", db.untern_string(key));
    })
}
