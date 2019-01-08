use crate::parser::Parser;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_hir as hir;

#[derive(new, DebugWith)]
crate struct HirIdentifier<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for HirIdentifier<'me, 'parse> {
    type Data = hir::Identifier;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let global_identifier = parser.expect(SpannedGlobalIdentifier)?;
        Ok(self.scope.add(
            global_identifier.span,
            hir::IdentifierData {
                text: global_identifier.value,
            },
        ))
    }
}
