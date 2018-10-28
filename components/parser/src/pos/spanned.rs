use crate::prelude::*;

use codespan::{ByteIndex, ByteSpan};

#[derive(Copy, Clone, DebugWith, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Spanned<T>(pub T, pub Span);

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Spanned<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {:?}", self.0, self.1)
    }
}

impl<T> Spanned<T> {
    crate fn wrap_span(node: T, span: Span) -> Spanned<T> {
        Spanned(node, span)
    }

    crate fn from(node: T, left: ByteIndex, right: ByteIndex) -> Spanned<T> {
        Spanned(node, Span::Real(ByteSpan::new(left, right)))
    }
}
