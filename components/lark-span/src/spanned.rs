use crate::{CurrentFile, Span};

use derive_new::new;
use lark_debug_derive::DebugWith;

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
