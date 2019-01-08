use crate::parser::Parser;
use crate::syntax::expression::expr1_group::Expression1;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::expression::ParsedExpression;
use crate::syntax::sigil::ExclamationPoint;
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_hir as hir;
use lark_span::{Spanned, FileName};

#[derive(new, DebugWith)]
crate struct Expression2<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl AsMut<ExpressionScope<'parse>> for Expression2<'_, 'parse> {
    fn as_mut(&mut self) -> &mut ExpressionScope<'parse> {
        self.scope
    }
}

impl Syntax<'parse> for Expression2<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expression1::new(self.scope)) || parser.test(UnaryOperator)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        if let Some(operator) = parser.parse_if_present(UnaryOperator) {
            let operator = operator?;
            let value = parser
                .expect(SkipNewline(Expression2::new(self.scope)))?
                .to_hir_expression(self.scope);
            let span = operator.span.extended_until_end_of(self.scope.span(value));
            return Ok(ParsedExpression::Expression(self.scope.add(
                span,
                hir::ExpressionData::Unary {
                    operator: operator.value,
                    value,
                },
            )));
        }

        parser.expect(Expression1::new(self.scope))
    }
}

#[derive(new, DebugWith)]
struct UnaryOperator;

impl Syntax<'parse> for UnaryOperator {
    type Data = Spanned<hir::UnaryOperator, FileName>;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(ExclamationPoint)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let spanned = parser.expect(ExclamationPoint)?;
        Ok(spanned.map(|_| hir::UnaryOperator::Not))
    }
}
