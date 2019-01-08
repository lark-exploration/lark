crate mod args;
crate mod binary;
crate mod block;
crate mod expr0_base;
crate mod expr1_group;
crate mod expr2_unary;
crate mod expr34_math;
crate mod expr5_eq;
crate mod ident;
crate mod literal;
crate mod member_access;
crate mod scope;

use crate::parser::Parser;
use crate::syntax::expression::expr5_eq::Expression5;
use crate::syntax::expression::ident::HirIdentifier;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::sigil::{Colon, Equals};
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_hir as hir;
use lark_span::FileName;
use lark_span::Span;

#[derive(Copy, Clone, DebugWith)]
crate enum ParsedExpression {
    Place(hir::Place),
    Expression(hir::Expression),
}

impl ParsedExpression {
    fn to_hir_expression(self, scope: &mut ExpressionScope<'_>) -> hir::Expression {
        match self {
            ParsedExpression::Expression(e) => e,
            ParsedExpression::Place(place) => {
                let span = scope.span(place);
                scope.add(span, hir::ExpressionData::Place { place })
            }
        }
    }

    fn to_hir_place(self, scope: &mut ExpressionScope<'_>) -> hir::Place {
        match self {
            ParsedExpression::Place(place) => place,
            ParsedExpression::Expression(expression) => {
                let span = scope.span(expression);
                scope.add(span, hir::PlaceData::Temporary(expression))
            }
        }
    }
}

impl hir::SpanIndex for ParsedExpression {
    fn span_from(self, tables: &hir::FnBodyTables) -> Span<FileName> {
        match self {
            ParsedExpression::Place(p) => p.span_from(tables),
            ParsedExpression::Expression(e) => e.span_from(tables),
        }
    }
}

#[derive(Copy, Clone)]
crate enum ParsedStatement {
    Expression(hir::Expression),
    Let(Span<FileName>, hir::Variable, Option<hir::Expression>),
}

#[derive(new, DebugWith)]
crate struct HirExpression<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for HirExpression<'me, 'parse> {
    type Data = hir::Expression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expression::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        Ok(parser
            .expect(Expression::new(self.scope))?
            .to_hir_expression(self.scope))
    }
}

#[derive(new, DebugWith)]
struct Expression<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Expression<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expression5::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        // Parse `Expression5`
        let expression = parser.expect(Expression5::new(self.scope))?;

        // Check for `Expression5 = Expression5`
        if let Some(_operator) = parser.parse_if_present(Equals) {
            let place = expression.to_hir_place(self.scope);

            let value = parser
                .expect(SkipNewline(Expression5::new(self.scope)))?
                .to_hir_expression(self.scope);

            let span = self
                .scope
                .span(place)
                .extended_until_end_of(parser.last_span());

            Ok(ParsedExpression::Expression(self.scope.add(
                span,
                hir::ExpressionData::Assignment { place, value },
            )))
        } else {
            Ok(expression)
        }
    }
}

#[derive(new, DebugWith)]
struct IdentifiedExpression<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for IdentifiedExpression<'me, 'parse> {
    type Data = hir::IdentifiedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        let mut parser = parser.checkpoint();
        if let Some(_) = parser.parse_if_present(HirIdentifier::new(self.scope)) {
            parser.test(Colon)
        } else {
            false
        }
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let identifier = parser.expect(HirIdentifier::new(self.scope))?;
        parser.expect(Colon)?;
        let expression = parser.expect(SkipNewline(HirExpression::new(self.scope)))?;
        let span = self
            .scope
            .span(identifier)
            .extended_until_end_of(self.scope.span(expression));
        Ok(self.scope.add(
            span,
            hir::IdentifiedExpressionData {
                identifier,
                expression,
            },
        ))
    }
}
