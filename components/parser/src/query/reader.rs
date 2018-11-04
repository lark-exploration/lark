use codespan::{ByteIndex, CodeMap, ColumnIndex, FileMap, FileName, LineIndex};
use crate::prelude::*;
use crate::StringId;
use map::FxIndexSet;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
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
            storage input;
        }
    }
}

#[derive(Debug, Default)]
pub struct SourceFiles {
    codemap: CodeMap,
    files: HashMap<StringId, Arc<File>>,
}

impl SourceFiles {
    fn insert(&mut self, path: &StringId, path_name: FileName, source: String) -> Arc<File> {
        let filemap = self.codemap.add_filemap(path_name, source);
        let file = Arc::new(File(filemap.clone()));
        self.files.insert(*path, file.clone());
        file
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Path(FileName);

#[derive(Debug)]
pub struct File(Arc<FileMap>);

impl File {
    pub fn source(&self) -> &str {
        self.0.src()
    }

    pub fn span(&self) -> Span {
        Span::Real(self.0.span())
    }

    // TODO: Take languageserver_types::Position?
    pub fn byte_index(&self, line: u64, column: u64) -> Result<ByteIndex, codespan::LocationError> {
        self.0
            .byte_index(LineIndex(line as u32), ColumnIndex(column as u32))
    }

    // TODO: Return languageserver_types::Position?
    pub fn location(&self, pos: ByteIndex) -> (u64, u64) {
        self.0
            .location(pos)
            .map(|(line, col)| (line.to_usize() as u64, col.to_usize() as u64))
            .unwrap()
    }
}

impl PartialEq for File {
    fn eq(&self, other: &File) -> bool {
        self.0.src() == other.0.src()
    }
}

impl Eq for File {}

impl Hash for Path {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.0 {
            FileName::Real(path) => {
                0.hash(state);
                path.hash(state);
            }

            FileName::Virtual(name) => {
                1.hash(state);
                name.hash(state);
            }
        }
    }
}

impl Hash for File {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.src().hash(state)
    }
}

pub fn initialize_reader(db: &mut impl ReaderDatabase) {
    db.query_mut(Files)
        .set((), Arc::new(RwLock::new(SourceFiles::default())));

    db.query_mut(Paths).set((), Arc::new(FxIndexSet::default()))
}

pub fn add_file(db: &mut impl ReaderDatabase, path: &str, source: impl Into<String>) -> Arc<File> {
    let path_id = db.intern_string(path);
    let source = source.into();

    let files = db.files();

    let mut paths = (*db.paths()).clone();
    paths.insert(path_id);
    db.query_mut(Paths).set((), Arc::new(paths));

    let mut files = files.write().unwrap();
    let file = files.insert(&path_id, codespan::FileName::Real(path.into()), source);
    db.query_mut(Source).set(path_id, file.clone());

    file
}
