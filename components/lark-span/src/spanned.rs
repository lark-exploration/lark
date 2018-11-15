use crate::{FileName, Span, SpanFile};

use derive_new::new;
use lark_debug_derive::DebugWith;

#[derive(Copy, Clone, Debug, DebugWith, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Spanned<T, File: SpanFile> {
    pub value: T,
    pub span: Span<File>,
}

impl<T, File: SpanFile> Spanned<T, File> {
    pub fn map<U>(self, value: impl FnOnce(T) -> U) -> Spanned<U, File> {
        Spanned {
            value: value(self.value),
            span: self.span,
        }
    }

    pub fn in_file_named(self, file_name: FileName) -> Spanned<T, FileName> {
        Spanned {
            value: self.value,
            span: self.span.in_file_named(file_name),
        }
    }
}

impl<T, File: SpanFile> std::ops::Deref for Spanned<T, File> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}
