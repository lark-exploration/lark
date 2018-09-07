use codespan::ByteIndex;
use codespan::ByteSpan;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Copy, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Span {
    Real(ByteSpan),
    Synthetic,
}

impl Span {
    crate fn from(left: ByteIndex, right: ByteIndex) -> Span {
        Span::Real(ByteSpan::new(left, right))
    }

    crate fn to(&self, to: Span) -> Span {
        match (self, to) {
            (Span::Real(left), Span::Real(right)) => Span::Real(left.to(right)),
            _ => Span::Synthetic,
        }
    }
}

impl Default for Span {
    fn default() -> Span {
        Span::Synthetic
    }
}

impl Hash for Span {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Span::Synthetic => 1.hash(state),
            Span::Real(span) => {
                2.hash(state);
                span.start().hash(state);
                span.end().hash(state);
            }
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Span::Real(span) => write!(f, "{}", span),
            Span::Synthetic => write!(f, "synthetic"),
        }
    }
}

#[derive(Copy, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Spanned<T> {
    crate node: T,
    crate span: Span,
}

impl<T> Spanned<T> {
    crate fn wrap(node: T, span: ByteSpan) -> Spanned<T> {
        Spanned {
            node,
            span: Span::Real(span),
        }
    }

    crate fn from(node: T, left: ByteIndex, right: ByteIndex) -> Spanned<T> {
        Spanned {
            node,
            span: Span::Real(ByteSpan::new(left, right)),
        }
    }

    crate fn synthetic(node: T) -> Spanned<T> {
        Spanned {
            node,
            span: Span::Synthetic,
        }
    }
}

crate trait HasSpan {
    type Inner;
    fn span(&self) -> Span;
}

impl<T> HasSpan for Spanned<T> {
    type Inner = T;
    fn span(&self) -> Span {
        self.span
    }
}
