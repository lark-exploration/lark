use lark_debug_derive::DebugWith;
use std::ops::Range;
use std::sync::Arc;

/// A "Text" is like a string except that it can be cheaply cloned.
/// You can also "extract" subtexts quite cheaply. You can also deref
/// an `&Text` into a `&str` for interoperability.
///
/// Used to represent the value of an input file.
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct Text {
    text: Arc<String>,
    start: usize,
    end: usize,
}

impl Text {
    /// Modifies this restrict to a subset of its current range.
    pub fn select(&mut self, range: Range<usize>) {
        let len = range.end - range.start;
        let new_start = self.start + range.start;
        let new_end = new_start + len;
        assert!(new_end <= self.end);

        self.start = new_start;
        self.end = new_end;
    }

    /// Extract a new `Text` that is a subset of an old `Text`
    /// -- `text.extract(1..3)` is similar to `&foo[1..3]` except that
    /// it gives back an owned value instead of a borrowed value.
    pub fn extract(&self, range: Range<usize>) -> Self {
        let mut result = self.clone();
        result.select(range);
        result
    }
}

impl From<Arc<String>> for Text {
    fn from(text: Arc<String>) -> Self {
        let end = text.len();
        Self {
            text,
            start: 0,
            end,
        }
    }
}

impl std::ops::Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text[self.start..self.end]
    }
}
