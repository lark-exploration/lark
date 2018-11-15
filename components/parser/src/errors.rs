use crate::prelude::*;

use lark_span::{ByteIndex, Span, SpanFile};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct ParseError<File: SpanFile> {
    pub description: String,
    pub span: Span<File>,
}

impl<File: SpanFile> ParseError<File> {
    pub fn from_pos(description: impl Into<String>, left: impl Into<ByteIndex>) -> ParseError {
        let pos = left.into();
        ParseError {
            description: description.into(),
            span: Span::from_indices(pos, pos),
        }
    }

    pub fn from_eof(description: impl Into<String>) -> ParseError {
        ParseError {
            description: description.into(),
            span: Span::EOF,
        }
    }

    pub fn from(
        description: impl Into<String>,
        left: impl Into<ByteIndex>,
        right: impl Into<ByteIndex>,
    ) -> ParseError {
        ParseError {
            description: description.into(),
            span: Span::from_indices(left.into(), right.into()),
        }
    }
}

impl<File: SpanFile> fmt::Display for ParseError<File> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {} at {:?}", self.description, self.span)
    }
}
