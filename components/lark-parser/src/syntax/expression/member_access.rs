use crate::parser::Parser;
use crate::syntax::expression::args::CallArguments;
use crate::syntax::expression::ident::HirIdentifier;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::expression::ParsedExpression;
use crate::syntax::sigil::Dot;
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_hir as hir;

#[derive(new, DebugWith)]
crate struct MemberAccess<'me, 'parse> {
    owner: ParsedExpression,
    scope: &'me mut ExpressionScope<'parse>,
}

crate enum ParsedMemberAccess {
    Field {
        member_name: hir::Identifier,
    },
    Method {
        member_name: hir::Identifier,
        arguments: hir::List<hir::Expression>,
    },
}

impl Syntax<'parse> for MemberAccess<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(SkipNewline(Dot))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(SkipNewline(Dot))?;
        let member_name = parser.expect(HirIdentifier::new(self.scope))?;

        if let Some(arguments) =
            parser.parse_if_present(CallArguments::new(Some(self.owner), self.scope))
        {
            let arguments = arguments?;
            let span = self
                .scope
                .span(self.owner)
                .extended_until_end_of(parser.last_span());
            Ok(ParsedExpression::Expression(self.scope.add(
                span,
                hir::ExpressionData::MethodCall {
                    method: member_name,
                    arguments,
                },
            )))
        } else {
            let owner = self.owner.to_hir_place(self.scope);
            let span = self
                .scope
                .span(owner)
                .extended_until_end_of(parser.last_span());
            Ok(ParsedExpression::Place(self.scope.add(
                span,
                hir::PlaceData::Field {
                    owner,
                    name: member_name,
                },
            )))
        }
    }
}
