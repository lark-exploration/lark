use crate::pos::Span;

use codespan::ByteOffset;

impl Span {
    pub fn to_range(&self, start: i32) -> std::ops::Range<usize> {
        let span = match self {
            Span::Real(span) => *span,
            other => unimplemented!("Can't turn {:?} into range", other),
        };

        let start_pos = span.start() + ByteOffset(start as i64);
        let end_pos = span.end() + ByteOffset(start as i64);

        start_pos.to_usize()..end_pos.to_usize()
    }
}
