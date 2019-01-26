use crate::lexer::token::LexToken;
use crate::macros::EntityMacroDefinition;
use crate::parser::Parser;
use crate::syntax::expression::ident::HirIdentifier;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::expression::{HirExpression, ParsedStatement};
use crate::syntax::guard::Guard;
use crate::syntax::sigil::{Equals, Let};
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::Syntax;
use crate::ParserDatabase;
use derive_new::new;
use lark_collections::{FxIndexMap, Seq};
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::ErrorReported;
use lark_error::WithError;
use lark_hir as hir;
use lark_span::FileName;
use lark_span::Spanned;
use lark_string::GlobalIdentifier;
use lark_string::Text;
use std::sync::Arc;

// # True grammar:
//
// Expression = Place | Value
//
// Value = Block
//    | Expression `(` (Expression),* `)`
//    | Expression BinaryOp Expression
//    | UnaryOp Expression
//    | Place `=` Expression
//    | Literal
//
// Place = Identifier
//    | Value // temporary
//    | Place `.` Identifier // field
//
// # Factored into "almost LL" form:
//
// Expression = {
//   Expression5,
//   Expression5 `=` Expression5,
// }
//
// Expression5 = {
//   Expression4,
//   Expression4 \n* `==` Expression4,
//   Expression4 \n* `!=` Expression4,
// }
//
// Expression4 = {
//   Expression3,
//   Expression4 \n* `+` Expression3,
//   Expression4 \n* `-` Expression3,
// }
//
// Expression3 = {
//   Expression2,
//   Expression3 \n* `*` Expression2,
//   Expression3 \n* `/` Expression2,
// }
//
// Expression2 = {
//   Expression1,
//   UnaryOp Expression0,
// }
//
// Expression1 = {
//   Expression0 `(` Comma(Expression) `)`
//   Expression0 `(` Comma(Field) `)`
//   Expression0 MemberAccess*
// }
//
// MemberAccess = {
//   \n* `.` Identifier,
//   `(` Comma(Expression) `)`,
// }
//
// Expression0 = {
//   Literal
//   Identifier,
//   `(` \n* Expression \n* `)`,  // Should we allow newlines *anywhere* here?
//   Block,
//   "if" Expression Block [ "else" Block ]
// }
//
// Block = {
//   `{` Statement* \n* `}`
// }
//
// Statement = {
//   \n* Expression Terminator,
//   \n* `let` Identifier [`:` Ty ] `=` Expression Terminator,
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
crate fn parse_fn_body(
    item_entity: Entity,
    db: &dyn ParserDatabase,
    entity_macro_definitions: &FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>>,
    input: &Text,                              // complete Text of file
    tokens: &Seq<Spanned<LexToken, FileName>>, // subset of Token corresponding to this expression
    self_argument: Option<Spanned<GlobalIdentifier, FileName>>,
    arguments: Seq<Spanned<GlobalIdentifier, FileName>>, // names of the arguments
) -> WithError<hir::FnBody> {
    let mut scope = ExpressionScope {
        db,
        item_entity,
        variables: Default::default(),
        fn_body_tables: Default::default(),
    };

    let arguments: Vec<_> = self_argument
        .iter()
        .chain(arguments.iter())
        .map(|&argument| {
            let name = scope.add(
                argument.span,
                hir::IdentifierData {
                    text: argument.value,
                },
            );
            let variable = scope.add(argument.span, hir::VariableData { name });
            scope.introduce_variable(variable);
            variable
        })
        .collect();
    let arguments = hir::List::from_iterator(&mut scope.fn_body_tables, arguments);

    let file_name = item_entity.input_file(&db).unwrap();
    let mut parser = Parser::new(file_name, db, entity_macro_definitions, input, tokens, 0);

    let root_expression = match parser.expect(HirExpression::new(&mut scope)) {
        Ok(e) => e,
        Err(err) => {
            let error = scope.add(err.span(), hir::ErrorData::Misc);
            scope.add(err.span(), hir::ExpressionData::Error { error })
        }
    };

    if let Some(span) = parser.parse_extra_input() {
        parser.report_error("extra input after end of expression", span);
    }

    parser.into_with_error(hir::FnBody {
        arguments: Ok(arguments),
        root_expression,
        tables: scope.fn_body_tables,
    })
}

#[derive(new, DebugWith)]
crate struct Statement<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Statement<'me, 'parse> {
    type Data = ParsedStatement;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(LetStatement::new(self.scope)) || parser.test(HirExpression::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        if let Some(r) = parser.parse_if_present(LetStatement::new(self.scope)) {
            return r;
        }

        let expression = parser.expect(HirExpression::new(self.scope))?;
        Ok(ParsedStatement::Expression(expression))
    }
}

#[derive(new, DebugWith)]
struct LetStatement<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for LetStatement<'me, 'parse> {
    type Data = ParsedStatement;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Let)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let let_keyword = parser.expect(Let)?;
        let name = parser.expect(HirIdentifier::new(self.scope))?;

        let mut initializer = None;
        if let Some(expression) =
            parser.parse_if_present(Guard(Equals, SkipNewline(HirExpression::new(self.scope))))
        {
            initializer = Some(expression?);
        }

        let span = let_keyword.span.extended_until_end_of(parser.peek_span());

        let name_span = self.scope.span(name);
        let variable = self.scope.add(name_span, hir::VariableData { name });

        // Subtle: This is a "side effect" that is visible to other
        // parsers that come after us within the same scope. Note that
        // entering a block (or other lexical scope) saves/restores
        // the set of variable bindings.
        self.scope.introduce_variable(variable);

        Ok(ParsedStatement::Let(span, variable, initializer))
    }
}
