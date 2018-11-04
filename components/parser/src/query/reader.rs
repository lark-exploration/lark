use codespan::{ByteIndex, CodeMap, ColumnIndex, FileMap, FileName, LineIndex};
use crate::prelude::*;
use crate::StringId;
use map::FxIndexSet;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

salsa::query_group! {
    pub trait ReaderDatabase: crate::HasParserState + salsa::Database {
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

/// Trait that the `ReaderDatabase` relies on; exposes the internal
/// storage we use to track files and create spans, which is currently
/// a `CodeMap`. Also exposes some helper functions that will mutate
/// that storage and set the `paths` and `source` inputs
/// appropriately for doing higher-level operations like adding a new file
/// into the system (or overwriting the text of an existing file with new text).
pub trait HasReaderState {
    fn reader_state(&self) -> &ReaderState;

    fn initialize_reader(&mut self)
    where
        Self: ReaderDatabase,
    {
        self.query_mut(Paths)
            .set((), Arc::new(FxIndexSet::default()))
    }

    fn add_file(&mut self, path: &str, source: impl Into<String>) -> Arc<File>
    where
        Self: ReaderDatabase,
    {
        let path_id = self.intern_string(path);
        let source = source.into();
        let data = self.reader_state().data.clone();

        let mut paths = (*self.paths()).clone();
        paths.insert(path_id);
        self.query_mut(Paths).set((), Arc::new(paths));

        let file = data
            .write()
            .insert(&path_id, codespan::FileName::Real(path.into()), source);
        self.query_mut(Source).set(path_id, file.clone());

        file
    }
}

/// The internal state for the reader queries. For a new database,
/// create fresh state using `ReaderState::default`; to fork off a
/// thread, simple clone.
#[derive(Clone, Debug, Default)]
pub struct ReaderState {
    data: Arc<RwLock<ReaderStateData>>,
}

/// The actual data for the `ReaderState`; this is private to this
/// module. It is held behind a rw-lock to permit parallel access.
///
/// Note: this permits us to mutate which could subvert the
/// incremental system. It is important that we only do not offer
/// operations that "read" the present state -- e.g., do not offer a
/// way to check *if* something is interned, only offer a way to
/// intern something (which returns an equivalent result whether or
/// not something was interned already).
#[derive(Debug, Default)]
struct ReaderStateData {
    codemap: CodeMap,
    files: HashMap<StringId, Arc<File>>,
}

impl ReaderStateData {
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
