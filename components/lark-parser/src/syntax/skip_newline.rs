use crate::parser::Parser;
use crate::syntax::NonEmptySyntax;
use crate::syntax::Syntax;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;

/// Skips over any newlines
#[derive(DebugWith)]
pub struct SkipNewline<T>(pub T);

impl<T> SkipNewline<T> {
    fn content(&self) -> &T {
        &self.0
    }
}

impl<T> Syntax<'parse> for SkipNewline<T>
where
    T: Syntax<'parse>,
{
    type Data = T::Data;

    fn test(&self, parser: &Parser<'parse>) -> bool {
        let mut parser = parser.checkpoint();
        parser.skip_newlines();
        parser.test(self.content())
    }

    fn expect(&self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.skip_newlines();
        parser.expect(self.content())
    }
}

impl<T> NonEmptySyntax<'parse> for SkipNewline<T> where T: NonEmptySyntax<'parse> {}
