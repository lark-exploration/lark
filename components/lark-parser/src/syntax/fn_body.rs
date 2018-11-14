use crate::parser::Parser;
use crate::span::CurrentFile;
use crate::span::Span;
use crate::span::Spanned;
use crate::syntax::delimited::Delimited;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::identifier::SpannedLocalIdentifier;
use crate::syntax::list::SeparatedList;
use crate::syntax::sigil::Curlies;
use crate::syntax::sigil::Semicolon;
use crate::syntax::Syntax;
use debug::DebugWith;
use derive_new::new;
use intern::Intern;
use intern::Untern;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::ErrorReported;
use lark_hir as hir;
use lark_seq::Seq;
use lark_string::global::GlobalIdentifier;
use map::FxIndexMap;
use std::rc::Rc;

// # True grammar:
//
// Expr = Place | Value
//
// Value = Block
//    | Expr "(" (Expr),* ")"
//    | Expr BinaryOp Expr
//    | UnaryOp Expr
//    | Place "=" Expr
//
// Place = Identifier
//    | Value // temporary
//    | Place "." Identifier // field
//
// # Factored into "almost LL" form:
//
// Expr = Expr5
//
// Expr5 = {
//   Expr4,
//   Expr5 \n* `==` Expr4,
//   Expr5 \n* `!=` Expr4,
// }
//
// Expr4 = {
//   Expr3,
//   Expr4 \n* `+` Expr3,
//   Expr4 \n* `-` Expr3,
// }
//
// Expr3 = {
//   Expr2,
//   Expr3 \n* `*` Expr2,
//   Expr3 \n* `/` Expr2,
// }
//
// Expr2 = {
//   Expr1,
//   UnaryOp Expr0,
// }
//
// Expr1 = {
//   Expr0 Expr1Suffix*
// }
//
// Expr1Suffix = {
//   \n* "." Identifier,
//   "(" Comma(Expr) ")",
//   "(" Comma(Field) ")",
// }
//
// Expr0 = {
//   Identifier,
//   "(" \n* Expr \n* ")",  // Should we allow newlines *anywhere* here?
//   Block,
//   "if" Expression Block [ "else" Block ]
// }
//
// Block = {
//   `{` Statement* \n* `}`
// }
//
// Statement = {
//   \n* Expr Terminator,
//   \n* `let` Identifier [`:` Ty ] `=` Expr Terminator,
// }
//
// Terminator = {
//   `;`
//   \n
// }
//

/// Parses an expression to create a `hir::FnBody`. Despite the name,
/// this can be used for any "free-standing" expression, such as the
/// value of a `const` and so forth.
#[derive(DebugWith)]
pub struct FnBody {
    arguments: Seq<Spanned<GlobalIdentifier>>,
}

impl Syntax<'parse> for FnBody {
    type Data = hir::FnBody;

    fn test(&mut self, _parser: &Parser<'parse>) -> bool {
        unimplemented!()
    }

    fn expect(&mut self, _parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        unimplemented!()
    }
}

#[derive(Copy, Clone)]
enum ParsedExpression {
    Place(hir::Place),
    Expression(hir::Expression),
}

impl ParsedExpression {
    fn to_hir_expression(self, scope: &mut ExpressionScope<'_>) -> hir::Expression {
        match self {
            ParsedExpression::Expression(e) => e,
            ParsedExpression::Place(place) => {
                let span = scope.span(place);
                let perm = scope.add(span, hir::PermData::Default);
                scope.add(span, hir::ExpressionData::Place { perm, place })
            }
        }
    }
}

#[derive(Copy, Clone)]
enum ParsedStatement {
    Expression(hir::Expression),
}

struct ExpressionScope<'parse> {
    db: &'parse dyn LazyParsedEntityDatabase,
    item_entity: Entity,
    variables: Rc<FxIndexMap<&'parse str, hir::Variable>>,
    fn_body_tables: hir::FnBodyTables,
}

impl ExpressionScope<'parse> {
    fn span(&self, _node: impl hir::HirIndex) -> Span<CurrentFile> {
        unimplemented!()
    }

    fn add<D: hir::HirIndexData>(&mut self, span: Span<CurrentFile>, node: D) -> D::Index {
        use parser::pos;

        // FIXME -- bridge spans
        drop(span);
        let span = pos::Span::Synthetic;

        D::index_vec_mut(&mut self.fn_body_tables).push(pos::Spanned(node, span))
    }

    fn report_error_expression(
        &mut self,
        parser: &mut Parser<'parser>,
        span: Span<CurrentFile>,
        data: hir::ErrorData,
    ) -> hir::Expression {
        let message = match data {
            hir::ErrorData::Misc => "error".to_string(),
            hir::ErrorData::Unimplemented => "unimplemented".to_string(),
            hir::ErrorData::UnknownIdentifier { text } => {
                format!("unknown identifier `{}`", text.untern(self.db))
            }
        };

        parser.report_error(message, span);

        self.already_reported_error_expression(span, data)
    }

    fn already_reported_error_expression(
        &mut self,
        span: Span<CurrentFile>,
        data: hir::ErrorData,
    ) -> hir::Expression {
        let error = self.add(span, data);
        self.add(span, hir::ExpressionData::Error { error })
    }

    fn unit_expression(&mut self, span: Span<CurrentFile>) -> hir::Expression {
        self.add(span, hir::ExpressionData::Unit {})
    }
}

impl DebugWith for ExpressionScope<'parse> {
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("ExpressionScope")
            .field("item_entity", &self.item_entity.debug_with(cx))
            .field("variables", &self.variables.debug_with(cx))
            .field("fn_body_tables", &self.fn_body_tables.debug_with(cx))
            .finish()
    }
}

#[derive(new, DebugWith)]
struct Expr<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Expr<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, _parser: &Parser<'parse>) -> bool {
        unimplemented!()
    }

    fn expect(&mut self, _parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        unimplemented!()
    }
}

#[derive(new, DebugWith)]
struct Expr0<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Expr0<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        SpannedLocalIdentifier.test(parser)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        // Expr0 = Identifier
        // Expr0 = "if" Expression Block [ "else" Block ]
        if parser.test(SpannedLocalIdentifier) {
            let text = parser.expect(SpannedLocalIdentifier)?;

            // FIXME generalize this to any macro
            if text.value == "if" {
                let condition = parser
                    .expect(Expr::new(self.scope))?
                    .to_hir_expression(self.scope);
                let if_true = parser
                    .expect(Block::new(self.scope))?
                    .to_hir_expression(self.scope);
                let if_false = if let Some(b) = parser.parse_if_present(Block::new(self.scope)) {
                    b?.to_hir_expression(self.scope)
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

            if let Some(&variable) = self.scope.variables.get(&text.value) {
                let place = self
                    .scope
                    .add(text.span, hir::PlaceData::Variable(variable));
                return Ok(ParsedExpression::Place(place));
            }

            if let Some(entity) = self
                .scope
                .db
                .resolve_name(self.scope.item_entity, text.value)
            {
                let place = self.scope.add(text.span, hir::PlaceData::Entity(entity));
                return Ok(ParsedExpression::Place(place));
            }

            let error_expression = self.scope.report_error_expression(
                parser,
                text.span,
                hir::ErrorData::UnknownIdentifier {
                    text: text.value.intern(self.scope.db),
                },
            );

            return Ok(ParsedExpression::Expression(error_expression));
        }

        // Expr0 = `(` Expr ')'
        // FIXME

        // Expr0 = `{` Block `}`
        if let Some(block) = parser.parse_if_present(Block::new(self.scope)) {
            return Ok(block?);
        }

        Err(parser.report_error("unrecognized start of expression", parser.peek_span()))
    }
}

#[derive(new, DebugWith)]
struct Block<'me, 'parse> {
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
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(self.definition())
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        // Save the map of variables before we start parsing
        let variables_on_entry = self.scope.variables.clone();

        // Convert a sequence of statements like `[a, b, c]` into a HIR tree
        // like `Sequence { first: Sequence { first: a, second: b }, second: c }`
        let start_span = parser.peek_span();
        let statements = parser.expect(self.definition())?;

        let mut statements_iter = statements.into_iter().cloned();

        let mut result = if let Some(first) = statements_iter.next() {
            first.to_hir_expression(self.scope)
        } else {
            // FIXME -- it'd be better if `Delimited` gave back a
            // `Spanned<X>` for its contents.
            let span = start_span.extended_until_end_of(parser.peek_span());
            return Ok(ParsedExpression::Expression(
                self.scope.unit_expression(span),
            ));
        };

        while let Some(next) = statements_iter.next() {
            let next = next.to_hir_expression(self.scope);

            let span = self
                .scope
                .span(result)
                .extended_until_end_of(self.scope.span(next));

            result = self.scope.add(
                span,
                hir::ExpressionData::Sequence {
                    first: result,
                    second: next,
                },
            );
        }

        // Restore the map of variables to what it used to be
        self.scope.variables = variables_on_entry;

        Ok(ParsedExpression::Expression(result))
    }
}

#[derive(new, DebugWith)]
struct Statement<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Statement<'me, 'parse> {
    type Data = ParsedStatement;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        unimplemented!()
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        unimplemented!()
    }
}
