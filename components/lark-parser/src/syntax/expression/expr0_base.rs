use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::expression::block::Block;
use crate::syntax::expression::literal::Literal;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::expression::ParsedExpression;
use crate::syntax::expression::{Expression, HirExpression};
use crate::syntax::identifier::SpannedLocalIdentifier;
use crate::syntax::sigil::Parentheses;
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_hir as hir;
use lark_intern::Intern;

#[derive(new, DebugWith)]
crate struct Expression0<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Expression0<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        SpannedLocalIdentifier.test(parser) || Literal::new(self.scope).test(parser)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        // Expression0 = Identifier
        // Expression0 = "if" Expression Block [ "else" Block ]
        if parser.test(SpannedLocalIdentifier) {
            let text = parser.expect(SpannedLocalIdentifier)?;

            // FIXME generalize this to any macro
            if text.value == "if" {
                let condition = parser.expect(HirExpression::new(self.scope))?;
                let if_true = parser.expect(Block::new(self.scope))?;
                let if_false = if let Some(b) = parser.parse_if_present(Block::new(self.scope)) {
                    b?
                } else {
                    self.scope.unit_expression(parser.elided_span())
                };

                let expression = self.scope.add(
                    text.span,
                    hir::ExpressionData::If {
                        condition,
                        if_true,
                        if_false,
                    },
                );

                return Ok(ParsedExpression::Expression(expression));
            }

            if let Some(variable) = self.scope.lookup_variable(text.value) {
                let place = self
                    .scope
                    .add(text.span, hir::PlaceData::Variable(variable));
                return Ok(ParsedExpression::Place(place));
            }

            let id = text.value.intern(&self.scope.db);
            if let Some(entity) = self.scope.db.resolve_name(self.scope.item_entity, id) {
                let place = self.scope.add(text.span, hir::PlaceData::Entity(entity));
                return Ok(ParsedExpression::Place(place));
            }

            let error_expression = self.scope.report_error_expression(
                parser,
                text.span,
                hir::ErrorData::UnknownIdentifier {
                    text: text.value.intern(&self.scope.db),
                },
            );

            return Ok(ParsedExpression::Expression(error_expression));
        }

        // Expression0 = Literal
        if let Some(expr) = parser.parse_if_present(Literal::new(self.scope)) {
            return Ok(ParsedExpression::Expression(expr?));
        }

        // Expression0 = `(` Expression ')'
        if let Some(expr) = parser.parse_if_present(Delimited(
            Parentheses,
            SkipNewline(Expression::new(self.scope)),
        )) {
            return Ok(expr?);
        }

        // Expression0 = `{` Block `}`
        if let Some(block) = parser.parse_if_present(Block::new(self.scope)) {
            return Ok(ParsedExpression::Expression(block?));
        }

        let token = parser.shift();
        Err(parser.report_error("unrecognized start of expression", token.span))
    }
}
