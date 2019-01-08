use crate::parser::Parser;
use crate::syntax::expression::args::{CallArguments, IdentifiedCallArguments};
use crate::syntax::expression::expr0_base::Expression0;
use crate::syntax::expression::member_access::MemberAccess;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::expression::ParsedExpression;
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_hir as hir;

#[derive(new, DebugWith)]
crate struct Expression1<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Expression1<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expression0::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let mut expr = parser.expect(Expression0::new(self.scope))?;

        // foo(f: a, g: b) -- struct construction
        //
        // FIXME -- we probably want to support `foo.bar.baz(f: a, g:
        // b)`, too? Have to figure out the module system.
        if let Some(fields) = parser.parse_if_present(IdentifiedCallArguments::new(self.scope)) {
            let fields = fields?;

            let place = expr.to_hir_place(self.scope);

            let span = self
                .scope
                .span(place)
                .extended_until_end_of(parser.last_span());

            // This is only legal if the receiver is a struct. This
            // seems like it should maybe not be baked into the
            // structure of the HIR, though...? (At minimum, the
            // entity reference should have a span!) In particular, I
            // imagine that at some point we might want to support
            // "type-relative" paths here, through associated
            // types. Ah well, worry about it then.
            if let hir::PlaceData::Entity(entity) = self.scope[place] {
                let expression = self
                    .scope
                    .add(span, hir::ExpressionData::Aggregate { entity, fields });
                return Ok(ParsedExpression::Expression(expression));
            } else {
                // Everything else is an error.
                let error_expression = self.scope.report_error_expression(
                    parser,
                    span,
                    hir::ErrorData::CanOnlyConstructStructs,
                );
                return Ok(ParsedExpression::Expression(error_expression));
            }
        }

        // foo(a, b, c) -- call
        //
        // NB. This must be tested *after* the "identified" form.
        if let Some(arguments) = parser.parse_if_present(CallArguments::new(None, self.scope)) {
            let arguments = arguments?;
            let function = expr.to_hir_expression(self.scope);
            let span = self
                .scope
                .span(function)
                .extended_until_end_of(parser.last_span());
            let expression = self.scope.add(
                span,
                hir::ExpressionData::Call {
                    function,
                    arguments,
                },
            );
            return Ok(ParsedExpression::Expression(expression));
        }

        // foo.bar.baz
        // foo.bar.baz(a, b, c)
        while let Some(member_access) = parser.parse_if_present(MemberAccess::new(expr, self.scope))
        {
            expr = member_access?;
        }

        Ok(expr)
    }
}
