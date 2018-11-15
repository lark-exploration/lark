use crate::Span;

use lark_debug_derive::DebugWith;
use lark_string::{GlobalIdentifier, Text};
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
