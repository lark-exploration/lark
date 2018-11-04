use crate::prelude::*;

use crate::StringId;

use codespan::{ByteIndex, CodeMap, ColumnIndex, FileMap, FileName, LineIndex};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct SourceFiles {
    codemap: CodeMap,
    files: HashMap<StringId, Arc<File>>,
}

impl SourceFiles {
    pub fn insert(&mut self, path: &StringId, path_name: FileName, source: String) {
        let file = self.codemap.add_filemap(path_name, source);
        self.files.insert(*path, Arc::new(File(file.clone())));
    }

    pub fn find(&self, path: &StringId) -> Option<Arc<File>> {
        self.files.get(path).clone().map(|p| p.clone())
    }

    pub fn paths(&self) -> Vec<StringId> {
        self.files.keys().cloned().collect()
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
