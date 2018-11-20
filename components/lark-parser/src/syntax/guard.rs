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
    fn guard(&mut self) -> &mut G {
        &mut self.0
    }

    fn value(&mut self) -> &mut V {
        &mut self.1
    }
}

impl<G, V> Syntax<'parse> for Guard<G, V>
where
    G: NonEmptySyntax<'parse>,
    V: Syntax<'parse>,
{
    type Data = V::Data;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(self.guard())
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let Guard(guard, value) = self;
        parser.expect(guard)?;
        parser.expect(value)
    }
}
