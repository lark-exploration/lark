use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::expression::ParsedExpression;
use crate::syntax::expression::{HirExpression, IdentifiedExpression};
use crate::syntax::list::CommaList;
use crate::syntax::sigil::{OpenParenthesis, Parentheses};
use crate::syntax::Syntax;
use derive_new::new;
use lark_collections::Seq;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_hir as hir;

#[derive(new, DebugWith)]
crate struct CallArguments<'me, 'parse> {
    arg0: Option<ParsedExpression>,
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for CallArguments<'me, 'parse> {
    type Data = hir::List<hir::Expression>;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(OpenParenthesis)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let expressions = parser.expect(Delimited(
            Parentheses,
            CommaList(HirExpression::new(self.scope)),
        ))?;

        let arg0 = self.arg0.map(|p| p.to_hir_expression(self.scope));

        Ok(hir::List::from_iterator(
            &mut self.scope.fn_body_tables,
            arg0.into_iter().chain(expressions.iter().cloned()),
        ))
    }
}

#[derive(new, DebugWith)]
crate struct IdentifiedCallArguments<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for IdentifiedCallArguments<'me, 'parse> {
    type Data = hir::List<hir::IdentifiedExpression>;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        let mut parser = parser.checkpoint();
        if let Some(_) = parser.parse_if_present(OpenParenthesis) {
            parser.test(IdentifiedExpression::new(self.scope))
        } else {
            false
        }
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let seq: Seq<hir::IdentifiedExpression> = parser.expect(Delimited(
            Parentheses,
            CommaList(IdentifiedExpression::new(self.scope)),
        ))?;
        Ok(hir::List::from_iterator(
            &mut self.scope.fn_body_tables,
            seq.iter().cloned(),
        ))
    }
}
