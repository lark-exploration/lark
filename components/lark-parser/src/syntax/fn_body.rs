use crate::lexer::token::LexToken;
use crate::macros::EntityMacroDefinition;
use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::guard::Guard;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::identifier::SpannedLocalIdentifier;
use crate::syntax::list::CommaList;
use crate::syntax::list::SeparatedList;
use crate::syntax::sigil::Colon;
use crate::syntax::sigil::Curlies;
use crate::syntax::sigil::Dot;
use crate::syntax::sigil::Equals;
use crate::syntax::sigil::ExclamationPoint;
use crate::syntax::sigil::Let;
use crate::syntax::sigil::OpenParenthesis;
use crate::syntax::sigil::Parentheses;
use crate::syntax::sigil::Semicolon;
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::Syntax;
use derive_new::new;
use lark_collections::{FxIndexMap, Seq};
use lark_debug_derive::DebugWith;
use lark_debug_with::DebugWith;
use lark_entity::Entity;
use lark_error::ErrorReported;
use lark_error::WithError;
use lark_hir as hir;
use lark_intern::Intern;
use lark_intern::Untern;
use lark_span::FileName;
use lark_span::Span;
use lark_span::Spanned;
use lark_string::GlobalIdentifier;
use lark_string::GlobalIdentifierTables;
use lark_string::Text;
use std::rc::Rc;
use std::sync::Arc;

// # True grammar:
//
// Expression = Place | Value
//
// Value = Block
//    | Expression "(" (Expression),* ")"
//    | Expression BinaryOp Expression
//    | UnaryOp Expression
//    | Place "=" Expression
//    | Literal
//
// Place = Identifier
//    | Value // temporary
//    | Place "." Identifier // field
//
// # Factored into "almost LL" form:
//
// Expression = Expression5
//
// Expression5 = {
//   Expression4,
//   Expression5 \n* `==` Expression4,
//   Expression5 \n* `!=` Expression4,
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
//   Expression0 "(" Comma(Expression) ")"
//   Expression0 "(" Comma(Field) ")"
//   Expression0 MemberAccess*
// }
//
// MemberAccess = {
//   \n* "." Identifier,
//   "(" Comma(Expression) ")",
// }
//
// Expression0 = {
//   Literal
//   Identifier,
//   "(" \n* Expression \n* ")",  // Should we allow newlines *anywhere* here?
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
    db: &dyn LazyParsedEntityDatabase,
    entity_macro_definitions: &FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>>,
    input: &Text,                                        // complete Text of file
    tokens: &Seq<Spanned<LexToken, FileName>>, // subset of Token corresponding to this expression
    arguments: Seq<Spanned<GlobalIdentifier, FileName>>, // names of the arguments
) -> WithError<hir::FnBody> {
    let mut scope = ExpressionScope {
        db,
        item_entity,
        variables: Default::default(),
        fn_body_tables: Default::default(),
    };

    let arguments: Vec<_> = arguments
        .iter()
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

#[derive(Copy, Clone, DebugWith)]
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
enum ParsedStatement {
    Expression(hir::Expression),
    Let(Span<FileName>, hir::Variable, Option<hir::Expression>),
}

struct ExpressionScope<'parse> {
    db: &'parse dyn LazyParsedEntityDatabase,
    item_entity: Entity,

    // FIXME -- we should not need to make *global identifiers* here,
    // but the current HIR requires it. We would need to refactor
    // `hir::Identifier` to take a `Text` instead (and, indeed, we
    // should do so).
    variables: Rc<FxIndexMap<GlobalIdentifier, hir::Variable>>,

    fn_body_tables: hir::FnBodyTables,
}

impl ExpressionScope<'parse> {
    fn span(&self, node: impl hir::SpanIndex) -> Span<FileName> {
        node.span_from(&self.fn_body_tables)
    }

    fn save_scope(&self) -> Rc<FxIndexMap<GlobalIdentifier, hir::Variable>> {
        self.variables.clone()
    }

    fn restore_scope(&mut self, scope: Rc<FxIndexMap<GlobalIdentifier, hir::Variable>>) {
        self.variables = scope;
    }

    /// Lookup a variable by name.
    fn lookup_variable(&self, text: &str) -> Option<hir::Variable> {
        // FIXME -- we should not need to intern this; see
        // definition of `variables` field above for details
        let global_id = text.intern(&self.db);

        self.variables.get(&global_id).cloned()
    }

    /// Brings a variable into scope, returning anything that was shadowed.
    fn introduce_variable(&mut self, variable: hir::Variable) {
        let name = self[variable].name;
        let text = self[name].text;
        Rc::make_mut(&mut self.variables).insert(text, variable);
    }

    fn add<D: hir::HirIndexData>(&mut self, span: Span<FileName>, value: D) -> D::Index {
        let index = D::index_vec_mut(&mut self.fn_body_tables).push(value);
        let meta_index: hir::MetaIndex = index.into();
        self.fn_body_tables.spans.insert(meta_index, span);

        index
    }

    fn report_error_expression(
        &mut self,
        parser: &mut Parser<'parser>,
        span: Span<FileName>,
        data: hir::ErrorData,
    ) -> hir::Expression {
        let message = match data {
            hir::ErrorData::Misc => "error".to_string(),
            hir::ErrorData::Unimplemented => "unimplemented".to_string(),
            hir::ErrorData::CanOnlyConstructStructs => {
                "can only supply named arguments when constructing structs".to_string()
            }
            hir::ErrorData::UnknownIdentifier { text } => {
                format!("unknown identifier `{}`", text.untern(&self.db))
            }
        };

        parser.report_error(message, span);

        self.already_reported_error_expression(span, data)
    }

    fn already_reported_error_expression(
        &mut self,
        span: Span<FileName>,
        data: hir::ErrorData,
    ) -> hir::Expression {
        let error = self.add(span, data);
        self.add(span, hir::ExpressionData::Error { error })
    }

    fn unit_expression(&mut self, span: Span<FileName>) -> hir::Expression {
        self.add(span, hir::ExpressionData::Unit {})
    }
}

impl AsRef<hir::FnBodyTables> for ExpressionScope<'_> {
    fn as_ref(&self) -> &hir::FnBodyTables {
        &self.fn_body_tables
    }
}

impl AsRef<GlobalIdentifierTables> for ExpressionScope<'_> {
    fn as_ref(&self) -> &GlobalIdentifierTables {
        self.db.as_ref()
    }
}

impl<I> std::ops::Index<I> for ExpressionScope<'parse>
where
    I: hir::HirIndex,
{
    type Output = I::Data;

    fn index(&self, index: I) -> &I::Data {
        &self.fn_body_tables[index]
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
struct HirExpression<'me, 'parse> {
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
        parser.expect(Expression5::new(self.scope))
    }
}

#[derive(new, DebugWith)]
struct Expression5<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Expression5<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expression4::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(BinaryOperatorExpression {
            expr: Expression4::new(self.scope),
            op: BinaryOperator::new(BINARY_OPERATORS_EXPR5),
        })
    }
}

#[derive(new, DebugWith)]
struct Expression4<'me, 'parse> {
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
        parser.expect(BinaryOperatorExpression {
            expr: Expression3::new(self.scope),
            op: BinaryOperator::new(BINARY_OPERATORS_EXPR4),
        })
    }
}

#[derive(new, DebugWith)]
struct Expression3<'me, 'parse> {
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
        parser.expect(BinaryOperatorExpression {
            expr: Expression2::new(self.scope),
            op: BinaryOperator::new(BINARY_OPERATORS_EXPR3),
        })
    }
}

#[derive(new, DebugWith)]
struct BinaryOperatorExpression<EXPR, OP> {
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

const BINARY_OPERATORS_EXPR3: &[(&str, hir::BinaryOperator)] = &[
    ("*", hir::BinaryOperator::Multiply),
    ("/", hir::BinaryOperator::Divide),
];

const BINARY_OPERATORS_EXPR4: &[(&str, hir::BinaryOperator)] = &[
    ("+", hir::BinaryOperator::Add),
    ("_", hir::BinaryOperator::Subtract),
];

const BINARY_OPERATORS_EXPR5: &[(&str, hir::BinaryOperator)] = &[
    ("==", hir::BinaryOperator::Equals),
    ("!=", hir::BinaryOperator::NotEquals),
];

#[derive(new, DebugWith)]
struct BinaryOperator {
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

#[derive(new, DebugWith)]
struct Expression2<'me, 'parse> {
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

#[derive(new, DebugWith)]
struct Expression1<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Expression1<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expression0::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let mut expr = parser.expect(Expression0::new(self.scope))?;

        // foo(f: a, g: b) -- struct construction
        //
        // FIXME -- we probably want to support `foo.bar.baz(f: a, g:
        // b)`, too? Have to figure out the module system.
        if let Some(fields) = parser.parse_if_present(IdentifiedCallArguments::new(self.scope)) {
            let fields = fields?;

            let place = expr.to_hir_place(self.scope);

            let span = self
                .scope
                .span(place)
                .extended_until_end_of(parser.last_span());

            // This is only legal if the receiver is a struct. This
            // seems like it should maybe not be baked into the
            // structure of the HIR, though...? (At minimum, the
            // entity reference should have a span!) In particular, I
            // imagine that at some point we might want to support
            // "type-relative" paths here, through associated
            // types. Ah well, worry about it then.
            if let hir::PlaceData::Entity(entity) = self.scope[place] {
                let expression = self
                    .scope
                    .add(span, hir::ExpressionData::Aggregate { entity, fields });
                return Ok(ParsedExpression::Expression(expression));
            } else {
                // Everything else is an error.
                let error_expression = self.scope.report_error_expression(
                    parser,
                    span,
                    hir::ErrorData::CanOnlyConstructStructs,
                );
                return Ok(ParsedExpression::Expression(error_expression));
            }
        }

        // foo(a, b, c) -- call
        //
        // NB. This must be tested *after* the "identified" form.
        if let Some(arguments) = parser.parse_if_present(CallArguments::new(None, self.scope)) {
            let arguments = arguments?;
            let function = expr.to_hir_expression(self.scope);
            let span = self
                .scope
                .span(function)
                .extended_until_end_of(parser.last_span());
            let expression = self.scope.add(
                span,
                hir::ExpressionData::Call {
                    function,
                    arguments,
                },
            );
            return Ok(ParsedExpression::Expression(expression));
        }

        // foo.bar.baz
        // foo.bar.baz(a, b, c)
        while let Some(member_access) = parser.parse_if_present(MemberAccess::new(expr, self.scope))
        {
            expr = member_access?;
        }

        Ok(expr)
    }
}

#[derive(new, DebugWith)]
struct MemberAccess<'me, 'parse> {
    owner: ParsedExpression,
    scope: &'me mut ExpressionScope<'parse>,
}

enum ParsedMemberAccess {
    Field {
        member_name: hir::Identifier,
    },
    Method {
        member_name: hir::Identifier,
        arguments: hir::List<hir::Expression>,
    },
}

impl Syntax<'parse> for MemberAccess<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(SkipNewline(Dot))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(SkipNewline(Dot))?;
        let member_name = parser.expect(HirIdentifier::new(self.scope))?;

        if let Some(arguments) =
            parser.parse_if_present(CallArguments::new(Some(self.owner), self.scope))
        {
            let arguments = arguments?;
            let span = self
                .scope
                .span(self.owner)
                .extended_until_end_of(parser.last_span());
            Ok(ParsedExpression::Expression(self.scope.add(
                span,
                hir::ExpressionData::MethodCall {
                    method: member_name,
                    arguments,
                },
            )))
        } else {
            let owner = self.owner.to_hir_place(self.scope);
            let span = self
                .scope
                .span(owner)
                .extended_until_end_of(parser.last_span());
            Ok(ParsedExpression::Place(self.scope.add(
                span,
                hir::PlaceData::Field {
                    owner,
                    name: member_name,
                },
            )))
        }
    }
}

#[derive(new, DebugWith)]
struct CallArguments<'me, 'parse> {
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
struct IdentifiedCallArguments<'me, 'parse> {
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

#[derive(new, DebugWith)]
struct HirIdentifier<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for HirIdentifier<'me, 'parse> {
    type Data = hir::Identifier;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let global_identifier = parser.expect(SpannedGlobalIdentifier)?;
        Ok(self.scope.add(
            global_identifier.span,
            hir::IdentifierData {
                text: global_identifier.value,
            },
        ))
    }
}

#[derive(new, DebugWith)]
struct Expression0<'me, 'parse> {
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

#[derive(new, DebugWith)]
struct Literal<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Literal<'me, 'parse> {
    type Data = hir::Expression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.is(LexToken::Integer) || parser.is(LexToken::String)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let text = parser.peek_str();
        let token = parser.shift();
        let kind = match token.value {
            LexToken::Integer => hir::LiteralKind::UnsignedInteger,
            LexToken::String => hir::LiteralKind::String,
            _ => return Err(parser.report_error("expected a literal", token.span)),
        };
        let value = text.intern(parser);
        let data = hir::LiteralData { kind, value };
        Ok(self
            .scope
            .add(token.span, hir::ExpressionData::Literal { data }))
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

#[derive(new, DebugWith)]
struct Statement<'me, 'parse> {
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
