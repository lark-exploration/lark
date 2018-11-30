use crate::Span;
use lark_debug_derive::DebugWith;
use lark_intern::{Intern, Untern};
use lark_string::{GlobalIdentifier, GlobalIdentifierTables, Text};
use std::fmt::Debug;

pub trait SpanFile: Copy + Debug + Eq + Ord {}
impl<T: Copy + Debug + Eq + Ord> SpanFile for T {}

impl<File: SpanFile> std::ops::Index<Span<File>> for Text {
    type Output = str;

    fn index(&self, span: Span<File>) -> &str {
        let s: &str = self;
        &s[span]
    }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct FileName {
    pub id: GlobalIdentifier,
}

impl FileName {
    pub fn untern(self, db: &dyn AsRef<GlobalIdentifierTables>) -> Text {
        self.id.untern(db)
    }
}

pub trait IntoFileName {
    fn into_file_name(&self, db: &dyn AsRef<GlobalIdentifierTables>) -> FileName;
}

impl IntoFileName for FileName {
    fn into_file_name(&self, _db: &dyn AsRef<GlobalIdentifierTables>) -> FileName {
        *self
    }
}

impl IntoFileName for &str {
    fn into_file_name(&self, db: &dyn AsRef<GlobalIdentifierTables>) -> FileName {
        FileName {
            id: self.intern(db),
        }
    }
}

impl IntoFileName for GlobalIdentifier {
    fn into_file_name(&self, _db: &dyn AsRef<GlobalIdentifierTables>) -> FileName {
        FileName { id: *self }
    }
}
