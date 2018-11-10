use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_string::text::Text;
use std::fmt::Debug;

pub trait SpanFile: Copy + Debug + Eq {}
impl<T: Copy + Debug + Eq> SpanFile for T {}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span<File: SpanFile> {
    file: File,
    start: usize,
    end: usize,
}

/// Relative to the "current file", which must be known.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurrentFile;

/// Relative to the "current entity", which must be known.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurrentEntity;

impl<File: SpanFile> Span<File> {
    pub fn new(file: File, start: usize, end: usize) -> Self {
        assert!(end >= start);
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

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn in_file<NewFile: SpanFile>(self, file: NewFile) -> Span<NewFile> {
        Span::new(file, self.start, self.end)
    }

    pub fn contains(self, span: Span<File>) -> bool {
        self.start >= span.start && self.end < span.end
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn relative_to_entity(self, entity_span: Span<File>) -> Span<CurrentEntity> {
        assert!(entity_span.contains(self));
        let len = self.len();
        let start = self.start - entity_span.start;
        Span::new(CurrentEntity, start, start + len)
    }
}

#[derive(Copy, Clone, Debug, DebugWith, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span<CurrentFile>,
}

impl<T> Spanned<T> {
    pub fn map<U>(self, value: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            value: value(self.value),
            span: self.span,
        }
    }
}

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl std::ops::Index<Span<CurrentFile>> for Text {
    type Output = str;

    fn index(&self, span: Span<CurrentFile>) -> &str {
        let s: &str = self;
        &s[span.start..span.end]
    }
}
