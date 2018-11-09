use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_string::text::Text;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, new)]
pub struct Location<File> {
    file: File,
    start: usize,
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span<File> {
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

impl<File: Copy> Span<File> {
    pub fn new(file: File, start: usize, end: usize) -> Self {
        assert!(end >= start);
        Span { file, start, end }
    }

    pub fn at(location: Location<File>) -> Self {
        let end = location.start + 1;
        Self::new(location.file, location.start, end)
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn in_file<NewFile: Copy>(self, file: NewFile) -> Span<NewFile> {
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
