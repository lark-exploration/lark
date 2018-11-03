use crate::StringId;

use codespan::{CodeMap, FileMap, FileName};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Debug)]
pub struct SourceFiles {
    codemap: CodeMap,
    files: HashMap<StringId, Arc<File>>,
}

impl SourceFiles {
    pub fn set(&mut self, path: &StringId, path_name: FileName, source: String) {
        let file = self.codemap.add_filemap(path_name, source);
        self.files.insert(*path, Arc::new(File(file.clone())));
    }

    pub fn get(&self, path: &StringId) -> Option<Arc<File>> {
        self.files.get(path).clone().map(|p| p.clone())
    }

    pub fn paths(&self) -> Vec<StringId> {
        self.files.keys().cloned().collect()
    }
}

#[derive(Debug)]
pub struct File(Arc<FileMap>);

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Path(FileName);

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
