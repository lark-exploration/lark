use crate::parser::Parser;
use crate::syntax::NonEmptySyntax;
use crate::syntax::Syntax;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;

/// If the "guard" `G` is present, parse the "value" `V`. The
/// resulting data is the data from the value `V`; the guard is
/// ignored.
///
/// Example: `-> Ty` parses the value `Ty` only if the guard `->` is present.
#[derive(DebugWith)]
pub struct Guard<G, V>(pub G, pub V);

impl<G, V> Guard<G, V> {
    fn guard(&self) -> &G {
        &self.0
    }

    fn value(&self) -> &V {
        &self.1
    }
}

impl<G, V> Syntax for Guard<G, V>
where
    G: NonEmptySyntax,
    V: Syntax,
{
    type Data = V::Data;

    fn test(&self, parser: &Parser<'_>) -> bool {
        parser.test(self.guard())
    }

    fn expect(&self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        parser.expect(self.guard())?;
        parser.expect(self.value())
    }
}
