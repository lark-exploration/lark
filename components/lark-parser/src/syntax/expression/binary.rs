use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::expression::ParsedExpression;
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_hir as hir;

#[derive(new, DebugWith)]
crate struct BinaryOperatorExpression<EXPR, OP> {
    // Expressions from below this level of operator precedence.
    expr: EXPR,

    // Operator to parse.
    op: OP,
}

impl<EXPR, OP> BinaryOperatorExpression<EXPR, OP>
where
    EXPR: AsMut<ExpressionScope<'parse>>,
{
    fn scope(&mut self) -> &mut ExpressionScope<'parse> {
        self.expr.as_mut()
    }
}

impl<EXPR, OP> Syntax<'parse> for BinaryOperatorExpression<EXPR, OP>
where
    EXPR: Syntax<'parse, Data = ParsedExpression> + AsMut<ExpressionScope<'parse>>,
    OP: Syntax<'parse, Data = hir::BinaryOperator>,
{
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(&mut self.expr)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let left_parsed = parser.expect(&mut self.expr)?;

        if parser.test(&mut self.op) {
            // From this point out, we know that this is not a "place expression".
            let mut left = left_parsed.to_hir_expression(self.scope());

            while let Some(operator) = parser.parse_if_present(&mut self.op) {
                let operator = operator?;
                let right = parser
                    .expect(SkipNewline(&mut self.expr))?
                    .to_hir_expression(self.scope());
                let span = self
                    .scope()
                    .span(left)
                    .extended_until_end_of(parser.last_span());
                left = self.scope().add(
                    span,
                    hir::ExpressionData::Binary {
                        operator,
                        left,
                        right,
                    },
                );

                match operator {
                    hir::BinaryOperator::Equals | hir::BinaryOperator::NotEquals => {
                        // Do not parse `a == b == c` etc
                        break;
                    }

                    hir::BinaryOperator::Add
                    | hir::BinaryOperator::Subtract
                    | hir::BinaryOperator::Multiply
                    | hir::BinaryOperator::Divide => {
                        // `a + b + c` is ok
                    }
                }
            }

            Ok(ParsedExpression::Expression(left))
        } else {
            Ok(left_parsed)
        }
    }
}

crate const BINARY_OPERATORS_EXPR3: &[(&str, hir::BinaryOperator)] = &[
    ("*", hir::BinaryOperator::Multiply),
    ("/", hir::BinaryOperator::Divide),
];

crate const BINARY_OPERATORS_EXPR4: &[(&str, hir::BinaryOperator)] = &[
    ("+", hir::BinaryOperator::Add),
    ("_", hir::BinaryOperator::Subtract),
];

crate const BINARY_OPERATORS_EXPR5: &[(&str, hir::BinaryOperator)] = &[
    ("==", hir::BinaryOperator::Equals),
    ("!=", hir::BinaryOperator::NotEquals),
];

#[derive(new, DebugWith)]
crate struct BinaryOperator {
    operators: &'static [(&'static str, hir::BinaryOperator)],
}

impl Syntax<'parse> for BinaryOperator {
    type Data = hir::BinaryOperator;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        if !parser.is(LexToken::Sigil) {
            return false;
        }

        let s = parser.peek_str();
        self.operators.iter().any(|(text, _)| *text == s)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let sigil_str = parser.peek_str();
        let token = parser.shift();
        if token.value != LexToken::Sigil {
            return Err(parser.report_error("expected an operator", token.span));
        }

        self.operators
            .iter()
            .filter_map(|&(text, binary_operator)| {
                if text == sigil_str {
                    Some(binary_operator)
                } else {
                    None
                }
            })
            .next()
            .ok_or_else(|| parser.report_error("unexpected operator", token.span))
    }
}
