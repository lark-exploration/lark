use codespan::{ByteIndex, ByteOffset, ByteSpan};
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Span {
    Real(ByteSpan),
    EOF,
    Synthetic,
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Span::Real(span) => {
                let start = span.start();
                let end = span.end();

                write!(f, "{}..{}", start, end)
            }

            Span::Synthetic => write!(f, "synthetic"),
            Span::EOF => write!(f, "EOF"),
        }
    }
}

impl Span {
    crate fn from(left: ByteIndex, right: ByteIndex) -> Span {
        Span::Real(ByteSpan::new(left, right))
    }

    crate fn from_pos(left: u32, right: u32) -> Span {
        Span::Real(ByteSpan::new(ByteIndex(left), ByteIndex(right)))
    }

    crate fn to(&self, to: Span) -> Span {
        match (self, to) {
            (Span::Real(left), Span::Real(right)) => Span::Real(left.to(right)),
            _ => Span::Synthetic,
        }
    }

    // crate fn to_codespan(&self) -> ByteSpan {
    //     match self {
    //         Span::Real(span) => *span,
    //         other => unimplemented!("{:?}", other),
    //     }
    // }

    crate fn to_range(&self, start: i32) -> std::ops::Range<usize> {
        let span = match self {
            Span::Real(span) => *span,
            other => unimplemented!("Can't turn {:?} into range", other),
        };

        let start_pos = span.start() + ByteOffset(start as i64);
        let end_pos = span.end() + ByteOffset(start as i64);

        start_pos.to_usize()..end_pos.to_usize()
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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Spanned<T>(crate T, crate Span);

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
    crate fn wrap_codespan(node: T, span: ByteSpan) -> Spanned<T> {
        Spanned(node, Span::Real(span))
    }

    crate fn wrap_span(node: T, span: Span) -> Spanned<T> {
        Spanned(node, span)
    }

    crate fn from(node: T, left: ByteIndex, right: ByteIndex) -> Spanned<T> {
        Spanned(node, Span::Real(ByteSpan::new(left, right)))
    }

    crate fn synthetic(node: T) -> Spanned<T> {
        Spanned(node, Span::Synthetic)
    }
}

crate trait HasSpan {
    type Inner;
    fn span(&self) -> Span;
}

impl<T> HasSpan for Spanned<T> {
    type Inner = T;
    fn span(&self) -> Span {
        self.1
    }
}
