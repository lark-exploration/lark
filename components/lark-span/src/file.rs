use crate::{CurrentFile, Span};

use lark_string::Text;
use std::fmt::Debug;

pub trait SpanFile: Copy + Debug + Eq {}
impl<T: Copy + Debug + Eq> SpanFile for T {}

impl std::ops::Index<Span<CurrentFile>> for Text {
    type Output = str;

    fn index(&self, span: Span<CurrentFile>) -> &str {
        let s: &str = self;
        &s[span.start..span.end]
    }
}
