use crate::{FileName, Location, OutOfBounds, SpanFile};

use lark_debug_derive::DebugWith;
use lark_string::Text;
use std::ops::Index;

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteIndex(crate usize);

impl ByteIndex {
    pub fn to_usize(self) -> usize {
        self.0
    }
}

impl From<usize> for ByteIndex {
    fn from(u: usize) -> ByteIndex {
        ByteIndex(u)
    }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteSize(crate usize);

impl<File: SpanFile> Index<Span<File>> for str {
    type Output = str;

    fn index(&self, range: Span<File>) -> &str {
        &self[range.start.0..range.end.0]
    }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span<File: SpanFile> {
    file: File,
    crate start: ByteIndex,
    crate end: ByteIndex,
}

/// Relative to the "current file", which must be known.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurrentFile;

/// Relative to the "current entity", which must be known.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurrentEntity;

impl<File: SpanFile> Span<File> {
    pub fn new(file: File, start: impl Into<ByteIndex>, end: impl Into<ByteIndex>) -> Self {
        let start = start.into();
        let end = end.into();

        assert!(end.0 >= start.0);

        Span { file, start, end }
    }

    /// Gives an empty span at the start of `file`.
    pub fn initial(file: File) -> Self {
        Span::new(file, 0, 0)
    }

    /// Gives the "EOF" span for a file with the given text.  This is
    /// an empty span pointing at the end.
    pub fn eof(file: File, text: &Text) -> Self {
        let len = text.len();
        Span::new(file, len, len)
    }

    /// Returns a span beginning at the start of this span but ending
    /// at the end of `other_span` (which must be within the same
    /// file).
    pub fn extended_until_end_of(self, other_span: Span<File>) -> Span<File> {
        assert_eq!(self.file, other_span.file);
        assert!(self.start <= other_span.end);
        Span::new(self.file, self.start, other_span.end)
    }

    pub fn start(&self) -> ByteIndex {
        self.start
    }

    pub fn end(&self) -> ByteIndex {
        self.end
    }

    pub fn in_file<NewFile: SpanFile>(self, file: NewFile) -> Span<NewFile> {
        Span::new(file, self.start, self.end)
    }

    pub fn in_file_named(self, file: FileName) -> Span<FileName> {
        Span::new(file, self.start, self.end)
    }

    pub fn contains(self, span: Span<File>) -> bool {
        self.start >= span.start && self.end < span.end
    }

    pub fn contains_index(self, index: ByteIndex) -> bool {
        self.start >= index && self.end < index
    }

    pub fn len(&self) -> ByteSize {
        ByteSize(self.end.0 - self.start.0)
    }

    pub fn relative_to_entity(self, entity_span: Span<File>) -> Span<CurrentEntity> {
        assert!(entity_span.contains(self));
        let len = self.len();
        let start = self.start.0 - entity_span.start.0;
        Span::new(CurrentEntity, start, start + len.0)
    }

    pub fn to_range(&self, s: &str) -> Result<languageserver_types::Range, OutOfBounds> {
        let left = Location::from_index(s, self.start)?.as_position();
        let right = Location::from_index(s, self.end)?.as_position();

        Ok(languageserver_types::Range::new(left, right))
    }
}
