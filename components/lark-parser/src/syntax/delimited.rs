use debug::DebugWith;
use crate::parser::Parser;
use crate::syntax::Delimiter;
use crate::syntax::Syntax;
use lark_error::ErrorReported;
use lark_debug_derive::DebugWith;

#[derive(DebugWith)]
pub struct Delimited<D, T>(pub D, pub T);

impl<D, T> Delimited<D, T> {
    fn delimiters(&self) -> &D {
        &self.0
    }

    fn content(&self) -> &T {
        &self.1
    }
}

impl<D, T> Syntax for Delimited<D, T>
where
    D: Delimiter + DebugWith,
    T: Syntax + DebugWith,
{
    type Data = T::Data;

    fn test(&self, parser: &Parser<'_>) -> bool {
        parser.test(self.delimiters().open_syntax())
    }

    fn parse(&self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        try {
            let Delimited(delimiter, content) = self;
            parser.expect(delimiter.open_syntax())?;
            let content = parser.expect(content)?;
            parser.expect(delimiter.close_syntax())?;
            content
        }
    }
}
