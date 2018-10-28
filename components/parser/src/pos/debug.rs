use crate::pos::Span;

use std::fmt;

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

debug::debug_fallback_impl!(Span);
