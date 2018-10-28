use crate::prelude::*;

use codespan::ByteIndex;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct ParseError {
    pub description: String,
    pub span: Span,
}

impl ParseError {
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

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {} at {:?}", self.description, self.span)
    }
}
