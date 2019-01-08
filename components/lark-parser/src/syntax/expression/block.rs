use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::expression::ParsedStatement;
use crate::syntax::fn_body::Statement;
use crate::syntax::list::SeparatedList;
use crate::syntax::sigil::{Curlies, Semicolon};
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_hir as hir;

#[derive(new, DebugWith)]
crate struct Block<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Block<'me, 'parse> {
    fn definition(
        &'a mut self,
    ) -> Delimited<Curlies, SeparatedList<Statement<'a, 'parse>, Semicolon>> {
        Delimited(
            Curlies,
            SeparatedList(Statement::new(self.scope), Semicolon),
        )
    }
}

impl Syntax<'parse> for Block<'me, 'parse> {
    type Data = hir::Expression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(self.definition())
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        // Save the map of variables before we start parsing
        let variables_on_entry = self.scope.save_scope();

        let start_span = parser.peek_span();
        let statements = parser.expect(self.definition())?;

        if statements.is_empty() {
            // FIXME -- it'd be better if `Delimited` gave back a
            // `Spanned<X>` for its contents.
            let span = start_span.extended_until_end_of(parser.peek_span());
            return Ok(self.scope.unit_expression(span));
        }

        // Convert a sequence of statements like `[a, b, c]` into a HIR tree
        // `[a, [b, c]]`.
        let mut statements_iter = statements.into_iter().rev().cloned();

        let mut result = match statements_iter.next().unwrap() {
            ParsedStatement::Expression(e) => e,
            ParsedStatement::Let(span, variable, initializer) => {
                // If a `let` appears as the last statement, then its associated
                // value is just a unit expression.
                let body = self.scope.unit_expression(parser.last_span());
                self.scope.add(
                    span,
                    hir::ExpressionData::Let {
                        variable,
                        initializer,
                        body,
                    },
                )
            }
        };

        while let Some(previous_statement) = statements_iter.next() {
            result = match previous_statement {
                ParsedStatement::Expression(previous) => self.scope.add(
                    self.scope.span(previous),
                    hir::ExpressionData::Sequence {
                        first: previous,
                        second: result,
                    },
                ),
                ParsedStatement::Let(span, variable, initializer) => self.scope.add(
                    span,
                    hir::ExpressionData::Let {
                        variable,
                        initializer,
                        body: result,
                    },
                ),
            };
        }

        // Restore the map of variables to what it used to be
        self.scope.restore_scope(variables_on_entry);

        Ok(result)
    }
}
