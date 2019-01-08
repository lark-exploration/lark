use crate::parser::Parser;
use crate::syntax::expression::binary::{
    BinaryOperator, BinaryOperatorExpression, BINARY_OPERATORS_EXPR5,
};
use crate::syntax::expression::expr34_math::Expression4;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::expression::ParsedExpression;
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;

#[derive(new, DebugWith)]
crate struct Expression5<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Expression5<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expression4::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(BinaryOperatorExpression::new(
            Expression4::new(self.scope),
            BinaryOperator::new(BINARY_OPERATORS_EXPR5),
        ))
    }
}
