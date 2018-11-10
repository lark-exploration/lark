use debug::DebugWith;
use std::ops::Range;
use std::sync::Arc;

/// A "Text" is like a string except that it can be cheaply cloned.
/// You can also "extract" subtexts quite cheaply. You can also deref
/// an `&Text` into a `&str` for interoperability.
///
/// Used to represent the value of an input file.
#[derive(Clone, PartialEq, Eq, Hash)]
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

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        let end = text.len();
        Self {
            text: Arc::new(text),
            start: 0,
            end,
        }
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Self {
        let end = text.len();
        Self {
            text: Arc::new(text.to_string()),
            start: 0,
            end,
        }
    }
}

impl std::borrow::Borrow<str> for Text {
    fn borrow(&self) -> &str {
        &self.text
    }
}

impl std::ops::Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <str as std::fmt::Display>::fmt(self, fmt)
    }
}

impl std::fmt::Debug for Text {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <str as std::fmt::Debug>::fmt(self, fmt)
    }
}

impl DebugWith for Text {
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <str as DebugWith>::fmt_with(self, cx, fmt)
    }
}

impl PartialEq<str> for Text {
    fn eq(&self, other: &str) -> bool {
        let this: &str = self;
        this == other
    }
}

impl PartialEq<String> for Text {
    fn eq(&self, other: &String) -> bool {
        let this: &str = self;
        let other: &str = other;
        this == other
    }
}

impl PartialEq<Text> for str {
    fn eq(&self, other: &Text) -> bool {
        other == self
    }
}

impl PartialEq<Text> for String {
    fn eq(&self, other: &Text) -> bool {
        other == self
    }
}

impl<T: ?Sized> PartialEq<&T> for Text
where
    Text: PartialEq<T>,
{
    fn eq(&self, other: &&T) -> bool {
        self == *other
    }
}
