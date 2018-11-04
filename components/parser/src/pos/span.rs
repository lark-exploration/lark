use crate::pos::Spanned;

use codespan::{ByteIndex, ByteSpan};
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Span {
    Real(ByteSpan),
    EOF,

    // TODO: This is silly
    Synthetic,
}
impl From<ByteSpan> for Span {
    fn from(v: ByteSpan) -> Self {
        Span::Real(v)
    }
}

impl Span {
    crate fn from_indices(left: ByteIndex, right: ByteIndex) -> Span {
        Span::Real(ByteSpan::new(left, right))
    }

    pub fn for_str(offset: usize, s: &str) -> Span {
        Span::from_pos(offset as u32, (offset + s.len()) as u32)
    }

    pub fn from_pos(left: u32, right: u32) -> Span {
        Span::Real(ByteSpan::new(ByteIndex(left), ByteIndex(right)))
    }

    crate fn to(&self, to: Span) -> Span {
        match (self, to) {
            (Span::Real(left), Span::Real(right)) => Span::Real(left.to(right)),
            _ => Span::Synthetic,
        }
    }

    pub fn start(&self) -> Option<ByteIndex> {
        match self {
            Span::Real(span) => Some(span.start()),
            Span::EOF => None,
            Span::Synthetic => None,
        }
    }

    pub fn end(&self) -> Option<ByteIndex> {
        match self {
            Span::Real(span) => Some(span.end()),
            Span::EOF => None,
            Span::Synthetic => None,
        }
    }

    pub fn contains(&self, position: ByteIndex) -> bool {
        match self {
            Span::Real(span) => position >= span.start() && position < span.end(),
            Span::EOF => false,
            Span::Synthetic => false,
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
            Span::EOF => 2.hash(state),
            Span::Real(span) => {
                3.hash(state);
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
            Span::EOF => write!(f, "end of file"),
        }
    }
}

pub trait HasSpan {
    type Inner;
    fn span(&self) -> Span;
    fn node(&self) -> &Self::Inner;
    fn copy<T>(&self, other: T) -> Spanned<T> {
        Spanned::wrap_span(other, self.span())
    }
}

impl<T> HasSpan for Spanned<T> {
    type Inner = T;
    fn span(&self) -> Span {
        self.1
    }

    fn node(&self) -> &T {
        &self.0
    }
}
