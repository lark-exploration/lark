use crate::parser::Parser;
use crate::syntax::expression::binary::{
    BinaryOperator, BinaryOperatorExpression, BINARY_OPERATORS_EXPR3, BINARY_OPERATORS_EXPR4,
};
use crate::syntax::expression::expr2_unary::Expression2;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::expression::ParsedExpression;
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;

#[derive(new, DebugWith)]
crate struct Expression3<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl AsMut<ExpressionScope<'parse>> for Expression3<'_, 'parse> {
    fn as_mut(&mut self) -> &mut ExpressionScope<'parse> {
        self.scope
    }
}

impl Syntax<'parse> for Expression3<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expression2::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(BinaryOperatorExpression::new(
            Expression2::new(self.scope),
            BinaryOperator::new(BINARY_OPERATORS_EXPR3),
        ))
    }
}

#[derive(new, DebugWith)]
crate struct Expression4<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl AsMut<ExpressionScope<'parse>> for Expression4<'_, 'parse> {
    fn as_mut(&mut self) -> &mut ExpressionScope<'parse> {
        self.scope
    }
}

impl Syntax<'parse> for Expression4<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expression3::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(BinaryOperatorExpression::new(
            Expression3::new(self.scope),
            BinaryOperator::new(BINARY_OPERATORS_EXPR4),
        ))
    }
}
