use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::span::CurrentFile;
use crate::span::Span;
use crate::span::Spanned;
use crate::syntax::delimited::Delimited;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::identifier::SpannedLocalIdentifier;
use crate::syntax::list::CommaList;
use crate::syntax::list::SeparatedList;
use crate::syntax::sigil::Colon;
use crate::syntax::sigil::Curlies;
use crate::syntax::sigil::Dot;
use crate::syntax::sigil::ExclamationPoint;
use crate::syntax::sigil::OpenParenthesis;
use crate::syntax::sigil::Parentheses;
use crate::syntax::sigil::Semicolon;
use crate::syntax::skip_newline::SkipNewline;
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
// Expression = Place | Value
//
// Value = Block
//    | Expression "(" (Expression),* ")"
//    | Expression BinaryOp Expression
//    | UnaryOp Expression
//    | Place "=" Expression
//
// Place = Identifier
//    | Value // temporary
//    | Place "." Identifier // field
//
// # Factored into "almost LL" form:
//
// Expression = Expr5
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
//   Expr0 "(" Comma(Expression) ")"
//   Expr0 "(" Comma(Field) ")"
//   Expr0 MemberAccess*
// }
//
// MemberAccess = {
//   \n* "." Identifier,
//   "(" Comma(Expression) ")",
// }
//
// Expr0 = {
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

#[derive(Copy, Clone)]
enum ParsedStatement {
    Expression(hir::Expression),
    Let(Span<CurrentFile>, hir::Variable, Option<hir::Expression>),
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
            hir::ErrorData::CanOnlyConstructStructs => {
                "can only supply named arguments when constructing structs".to_string()
            }
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
        parser.test(Expr5::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(Expr5::new(self.scope))
    }
}

#[derive(new, DebugWith)]
struct Expr5<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Expr5<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expr4::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(BinaryOperatorExpression {
            expr: Expr4::new(self.scope),
            op: BinaryOperator::new(BINARY_OPERATORS_EXPR5),
        })
    }
}

#[derive(new, DebugWith)]
struct Expr4<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl AsMut<ExpressionScope<'parse>> for Expr4<'_, 'parse> {
    fn as_mut(&mut self) -> &mut ExpressionScope<'parse> {
        self.scope
    }
}

impl Syntax<'parse> for Expr4<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expr3::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(BinaryOperatorExpression {
            expr: Expr3::new(self.scope),
            op: BinaryOperator::new(BINARY_OPERATORS_EXPR4),
        })
    }
}

#[derive(new, DebugWith)]
struct Expr3<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl AsMut<ExpressionScope<'parse>> for Expr3<'_, 'parse> {
    fn as_mut(&mut self) -> &mut ExpressionScope<'parse> {
        self.scope
    }
}

impl Syntax<'parse> for Expr3<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expr2::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(BinaryOperatorExpression {
            expr: Expr2::new(self.scope),
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
        let left = parser.expect(&mut self.expr)?;

        if parser.test(&mut self.op) {
            // From this point out, we know that this is not a "place expression".
            let mut left = left.to_hir_expression(self.scope());

            while let Some(operator) = parser.parse_if_present(&mut self.op) {
                let operator = operator?;
                let right = parser
                    .expect(SkipNewline(Expr2::new(self.scope())))?
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
        }

        Ok(left)
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
struct Expr2<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl AsMut<ExpressionScope<'parse>> for Expr2<'_, 'parse> {
    fn as_mut(&mut self) -> &mut ExpressionScope<'parse> {
        self.scope
    }
}

impl Syntax<'parse> for Expr2<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expr1::new(self.scope)) || parser.test(UnaryOperator)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        if let Some(operator) = parser.parse_if_present(UnaryOperator) {
            let operator = operator?;
            let value = parser
                .expect(SkipNewline(Expr2::new(self.scope)))?
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

        parser.expect(Expr1::new(self.scope))
    }
}

#[derive(new, DebugWith)]
struct UnaryOperator;

impl Syntax<'parse> for UnaryOperator {
    type Data = Spanned<hir::UnaryOperator>;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(ExclamationPoint)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let spanned = parser.expect(ExclamationPoint)?;
        Ok(spanned.map(|_| hir::UnaryOperator::Not))
    }
}

#[derive(new, DebugWith)]
struct Expr1<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Expr1<'me, 'parse> {
    type Data = ParsedExpression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(Expr0::new(self.scope))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let mut expr = parser.expect(Expr0::new(self.scope))?;

        // foo(a, b, c) -- call
        if let Some(arguments) = parser.parse_if_present(CallArguments::new(self.scope)) {
            let arguments = arguments?;
            let function = expr.to_hir_place(self.scope);
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
            if let hir::PlaceData::Entity(entity) = self.scope.fn_body_tables[place] {
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

        // foo.bar.baz
        // foo.bar.baz(a, b, c)
        while let Some(member_access) = parser.parse_if_present(MemberAccess::new(self.scope)) {
            let member_access = member_access?;
            let owner = expr.to_hir_place(self.scope);
            let span = self
                .scope
                .span(owner)
                .extended_until_end_of(parser.last_span());
            match member_access {
                ParsedMemberAccess::Field { member_name: name } => {
                    expr = ParsedExpression::Place(
                        self.scope.add(span, hir::PlaceData::Field { owner, name }),
                    );
                }
                ParsedMemberAccess::Method {
                    member_name: method,
                    arguments,
                } => {
                    expr = ParsedExpression::Expression(self.scope.add(
                        span,
                        hir::ExpressionData::MethodCall {
                            owner,
                            method,
                            arguments,
                        },
                    ));
                }
            }
        }

        Ok(expr)
    }
}

#[derive(new, DebugWith)]
struct MemberAccess<'me, 'parse> {
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
    type Data = ParsedMemberAccess;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(SkipNewline(Dot))
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        parser.expect(SkipNewline(Dot))?;
        let member_name = parser.expect(HirIdentifier::new(self.scope))?;

        if let Some(arguments) = parser.parse_if_present(CallArguments::new(self.scope)) {
            let arguments = arguments?;
            Ok(ParsedMemberAccess::Method {
                member_name,
                arguments,
            })
        } else {
            Ok(ParsedMemberAccess::Field { member_name })
        }
    }
}

#[derive(new, DebugWith)]
struct CallArguments<'me, 'parse> {
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
        Ok(hir::List::from_iterator(
            &mut self.scope.fn_body_tables,
            expressions.iter().cloned(),
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
        parser.test(OpenParenthesis)
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
        parser.test(HirIdentifier::new(self.scope))
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

        // Expr0 = `(` Expression ')'
        if let Some(expr) = parser.parse_if_present(Delimited(
            Parentheses,
            SkipNewline(Expression::new(self.scope)),
        )) {
            return Ok(expr?);
        }

        // Expr0 = `{` Block `}`
        if let Some(block) = parser.parse_if_present(Block::new(self.scope)) {
            return Ok(ParsedExpression::Expression(block?));
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
    type Data = hir::Expression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(self.definition())
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        // Save the map of variables before we start parsing
        let variables_on_entry = self.scope.variables.clone();

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
        self.scope.variables = variables_on_entry;

        Ok(result)
    }
}

#[derive(new, DebugWith)]
struct Statement<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Statement<'me, 'parse> {
    type Data = ParsedStatement;

    fn test(&mut self, _parser: &Parser<'parse>) -> bool {
        unimplemented!()
    }

    fn expect(&mut self, _parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        unimplemented!()
    }
}
