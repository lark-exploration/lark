use codespan::{ByteIndex, CodeMap, ColumnIndex, FileMap, LineIndex};
use crate::prelude::*;
use crate::StringId;
use map::FxIndexSet;
use parking_lot::RwLock;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

salsa::query_group! {
    pub trait ReaderDatabase: crate::HasParserState + HasReaderState + salsa::Database {
        fn paths() -> Arc<FxIndexSet<StringId>> {
            type Paths;
        }

        fn source(key: StringId) -> File {
            type Source;
            storage input;
        }

        fn paths_trigger() -> () {
            type PathsTrigger;

            // This "pseudo-input" is written to each time the set of
            // paths changes. It's a hack to make the derived `paths`
            // query below re-execute as needed.
            storage input;
        }
    }
}

fn paths(db: &impl ReaderDatabase) -> Arc<FxIndexSet<StringId>> {
    // Register a read of the `paths_trigger` input:
    db.paths_trigger();

    db.reader_state().data.read().paths.clone()
}

/// Trait that the `ReaderDatabase` relies on; exposes the internal
/// storage we use to track files and create spans, which is currently
/// a `CodeMap`. Also exposes some helper functions that will mutate
/// that storage and set the `paths` and `source` inputs
/// appropriately for doing higher-level operations like adding a new file
/// into the system (or overwriting the text of an existing file with new text).
pub trait HasReaderState {
    fn reader_state(&self) -> &ReaderState;

    /// Adds a new file (or overwrites an existing file) into our
    /// reader database. Returns the `File` you can use to talk
    /// about it, but that file is also available via
    /// `self.source(path_id)` (where `path_id` is the interned
    /// version of `path`).
    fn add_file(&mut self, path: &str, source: impl Into<String>) -> File
    where
        Self: ReaderDatabase,
    {
        let path_id = self.intern_string(path);
        let source = source.into();

        let mut trigger_path = false;
        let file = {
            // Acquire the write-lock on the reader state and create
            // the `file` in the codemap.
            let mut data = self.reader_state().data.write();

            // Update the full set of all paths if necessary.
            if !data.paths.contains(&path_id) {
                Arc::make_mut(&mut data.paths).insert(path_id);
                trigger_path = true;
            }

            // Insert new file into the codemap.
            let codemap_path_name = codespan::FileName::Real(path.into());
            let filemap = data.codemap.add_filemap(codemap_path_name, source);
            File(filemap.clone())
        };

        if trigger_path {
            self.query_mut(PathsTrigger).set((), ());
        }
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
    /// The codemap that we use to store all of our inputs.
    codemap: CodeMap,

    /// The full set of paths.
    paths: Arc<FxIndexSet<StringId>>,
}

/// Represents (one version of) a file that has been added into the
/// reader database. You can obtain one of these via the `source` query.
#[derive(Clone, Debug)]
pub struct File(Arc<FileMap>);

impl File {
    /// Get access to the full text of the file.
    pub fn source(&self) -> &str {
        self.0.src()
    }

    /// Returns the full span of the file.
    pub fn span(&self) -> Span {
        Span::Real(self.0.span())
    }

    /// Given a line and column, returns the codespan
    /// `ByteIndex`. This is useful for bridging to the language
    /// server.
    ///
    /// TODO: Perhaps we want to just take languageserver_types::Position?
    pub fn byte_index(&self, line: u64, column: u64) -> Result<ByteIndex, codespan::LocationError> {
        self.0
            .byte_index(LineIndex(line as u32), ColumnIndex(column as u32))
    }

    /// Given a byte-index, convert back to a line and column
    /// number. This is useful for bridging to the language server.
    ///
    /// TODO: Perhaps we want to just return
    /// `languageserver_types::Position`?
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

impl Hash for File {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.src().hash(state)
    }
}
