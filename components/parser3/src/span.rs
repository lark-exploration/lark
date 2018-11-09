use derive_new::new;
use lark_debug_derive::DebugWith;

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

impl<File> Span<File> {
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

    pub fn in_file<NewFile>(self, file: NewFile) -> Span<NewFile> {
        Span::new(file, self.start, self.end)
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
