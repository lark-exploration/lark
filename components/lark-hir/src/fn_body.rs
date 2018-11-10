use parser::prelude::*;

use ast::ast as a;
use crate as hir;
use crate::HirDatabase;
use lark_entity::Entity;
use lark_error::Diagnostic;
use lark_error::ErrorReported;
use lark_error::WithError;
use lark_string::global::GlobalIdentifier;
use map::FxIndexMap;
use parser::pos::{Span, Spanned};
use std::sync::Arc;

crate fn fn_body(db: &impl HirDatabase, item_entity: Entity) -> WithError<Arc<crate::FnBody>> {
    let mut errors = vec![];
    let fn_body = HirLower::new(db, item_entity, &mut errors).lower_ast_of_item();
    WithError {
        value: Arc::new(fn_body),
        errors,
    }
}

struct HirLower<'me, DB: HirDatabase> {
    db: &'me DB,
    item_entity: Entity,
    fn_body_tables: hir::FnBodyTables,
    variables: FxIndexMap<GlobalIdentifier, hir::Variable>,
    errors: &'me mut Vec<Diagnostic>,
}

impl<'me, DB> HirLower<'me, DB>
where
    DB: HirDatabase,
{
    fn new(db: &'me DB, item_entity: Entity, errors: &'me mut Vec<Diagnostic>) -> Self {
        HirLower {
            db,
            errors,
            item_entity,
            fn_body_tables: Default::default(),
            variables: Default::default(),
        }
    }

    fn add<D: hir::HirIndexData>(&mut self, span: Span, node: D) -> D::Index {
        D::index_vec_mut(&mut self.fn_body_tables).push(Spanned(node, span))
    }

    fn span(&self, index: impl hir::SpanIndex) -> Span {
        index.span_from(&self.fn_body_tables)
    }

    fn save_scope(&self) -> FxIndexMap<GlobalIdentifier, hir::Variable> {
        self.variables.clone()
    }

    fn restore_scope(&mut self, scope: FxIndexMap<GlobalIdentifier, hir::Variable>) {
        self.variables = scope;
    }

    /// Brings a variable into scope, returning anything that was shadowed.
    fn bring_into_scope(&mut self, variable: hir::Variable) {
        let name = self[variable].name;
        self.variables.insert(self[name].text, variable);
    }

    fn lower_ast_of_item(mut self) -> hir::FnBody {
        match self.db.ast_of_item(self.item_entity) {
            Ok(ast) => match &*ast {
                a::Item::Struct(_) => panic!("asked for fn-body of struct {:?}", self.item_entity),
                a::Item::Def(def) => {
                    let arguments = self.lower_parameters(&def.parameters);

                    for &argument in &arguments {
                        self.bring_into_scope(argument);
                    }

                    let root_expression = self.lower_block(&def.body);

                    let arguments = hir::List::from_iterator(&mut self.fn_body_tables, arguments);

                    hir::FnBody {
                        arguments,
                        root_expression,
                        tables: self.fn_body_tables,
                    }
                }
            },

            Err(ErrorReported(ref spans)) => {
                let root_expression = self.already_reported_error_expression(
                    spans.first().unwrap().span,
                    hir::ErrorData::Misc,
                );

                hir::FnBody {
                    arguments: hir::List::default(),
                    root_expression,
                    tables: self.fn_body_tables,
                }
            }
        }
    }

    fn lower_parameters(&mut self, parameters: &Vec<a::Field>) -> Vec<hir::Variable> {
        parameters
            .iter()
            .map(|parameter| {
                let name = self.add(
                    parameter.name.span(),
                    hir::IdentifierData {
                        text: *parameter.name,
                    },
                );
                self.add(parameter.span, hir::VariableData { name })
            })
            .collect()
    }

    fn lower_block(&mut self, block: &Spanned<a::Block>) -> hir::Expression {
        self.lower_block_items(&block.expressions)
            .unwrap_or_else(|| self.unit_expression(block.span()))
    }

    fn lower_block_items(&mut self, all_block_items: &[a::BlockItem]) -> Option<hir::Expression> {
        let (first_block_item, remaining_block_items) = all_block_items.split_first()?;
        match first_block_item {
            a::BlockItem::Item(_) => return self.lower_block_items(remaining_block_items),

            a::BlockItem::Decl(decl) => match decl {
                a::Declaration::Let(l) => Some(self.lower_let(l, remaining_block_items)),
            },

            a::BlockItem::Expr(expr) => {
                let first = self.lower_expression(expr);

                match self.lower_block_items(remaining_block_items) {
                    None => Some(first),

                    Some(second) => {
                        let span = self.span(second);
                        Some(self.add(span, hir::ExpressionData::Sequence { first, second }))
                    }
                }
            }
        }
    }

    fn lower_let(&mut self, let_decl: &a::Let, block_items: &[a::BlockItem]) -> hir::Expression {
        let saved_scope = self.save_scope();

        let a::Let {
            pattern,
            ty: _, /* FIXME */
            init,
        } = let_decl;

        let variable = match **pattern {
            a::Pattern::Underscore => unimplemented!("underscore patterns -- too lazy"),

            a::Pattern::Identifier(identifier, _mode) => {
                let name = self.add(identifier.span(), hir::IdentifierData { text: *identifier });
                self.add(identifier.span(), hir::VariableData { name })
            }
        };

        let variable_span = self.span(variable);

        let initializer = init
            .as_ref()
            .map(|expression| self.lower_expression(expression));

        self.bring_into_scope(variable);

        let body = self
            .lower_block_items(block_items)
            .unwrap_or_else(|| self.unit_expression(variable_span)); // FIXME: wrong span

        self.restore_scope(saved_scope);

        self.add(
            variable_span,
            hir::ExpressionData::Let {
                variable,
                initializer,
                body,
            },
        )
    }

    fn lower_expression(&mut self, expr: &a::Expression) -> hir::Expression {
        match expr {
            a::Expression::Block(block) => self.lower_block(block),

            a::Expression::Literal(lit) => match lit {
                a::Literal::String(s) => self.add(
                    s.span(),
                    hir::ExpressionData::Literal {
                        data: hir::LiteralData::String(**s),
                    },
                ),
            },
            a::Expression::Call(call) => {
                let a::Callee::Identifier(ref identifier) = call.callee;

                let function = self.lower_identifier_place(identifier);

                let mut args = vec![];

                for call_argument in call.arguments.iter() {
                    args.push(self.lower_expression(&call_argument));
                }

                let arguments = hir::List::from_iterator(&mut self.fn_body_tables, args);

                self.add(
                    call.span(),
                    hir::ExpressionData::Call {
                        function,
                        arguments,
                    },
                )
            }
            a::Expression::Interpolation(..) | a::Expression::ConstructStruct(_) => {
                self.unimplemented(expr.span())
            }

            a::Expression::Binary(spanned_op, lhs_expr, rhs_expr) => {
                let left = self.lower_expression(lhs_expr);
                let right = self.lower_expression(rhs_expr);
                let operator = match **spanned_op {
                    parser::parser::ast::Op::Add => hir::BinaryOperator::Add,
                    parser::parser::ast::Op::Sub => hir::BinaryOperator::Subtract,
                    parser::parser::ast::Op::Mul => hir::BinaryOperator::Multiply,
                    parser::parser::ast::Op::Div => hir::BinaryOperator::Divide,
                };
                self.add(
                    spanned_op.span(),
                    hir::ExpressionData::Binary {
                        operator,
                        left,
                        right,
                    },
                )
            }

            a::Expression::Ref(_) => {
                let place = self.lower_place(expr);
                let span = self.span(place);
                let perm = self.add(span, hir::PermData::Default);
                self.add(span, hir::ExpressionData::Place { perm, place })
            }
        }
    }

    fn unimplemented(&mut self, span: Span) -> hir::Expression {
        self.errors
            .push(Diagnostic::new("unimplemented".into(), span));
        let error = self.add(span, hir::ErrorData::Unimplemented);
        self.add(span, hir::ExpressionData::Error { error })
    }

    fn lower_identifier_place(&mut self, identifier: &a::Identifier) -> hir::Place {
        if let Some(&variable) = self.variables.get(identifier.node()) {
            return self.add(identifier.span(), hir::PlaceData::Variable(variable));
        }

        if let Some(entity) = self.db.resolve_name(self.item_entity, *identifier.node()) {
            return self.add(identifier.span(), hir::PlaceData::Entity(entity));
        }

        let error_expression = self.report_error_expression(
            identifier.span(),
            hir::ErrorData::UnknownIdentifier {
                text: *identifier.node(),
            },
        );

        self.add(
            identifier.span(),
            hir::PlaceData::Temporary(error_expression),
        )
    }

    fn lower_place(&mut self, expr: &a::Expression) -> hir::Place {
        match expr {
            a::Expression::Ref(identifier) => self.lower_identifier_place(identifier),

            a::Expression::Block(_)
            | a::Expression::ConstructStruct(_)
            | a::Expression::Call(_)
            | a::Expression::Binary(..)
            | a::Expression::Interpolation(..)
            | a::Expression::Literal(..) => {
                let expression = self.lower_expression(expr);
                let span = self.span(expression);
                self.add(span, hir::PlaceData::Temporary(expression))
            }
        }
    }

    fn report_error_expression(&mut self, span: Span, data: hir::ErrorData) -> hir::Expression {
        let message = match data {
            hir::ErrorData::Misc => "error".to_string(),
            hir::ErrorData::Unimplemented => "unimplemented".to_string(),
            hir::ErrorData::UnknownIdentifier { text } => {
                format!("unknown identifier `{}`", self.db.untern_string(text))
            }
        };

        self.errors.push(Diagnostic::new(message, span));

        self.already_reported_error_expression(span, data)
    }

    fn already_reported_error_expression(
        &mut self,
        span: Span,
        data: hir::ErrorData,
    ) -> hir::Expression {
        let error = self.add(span, data);
        self.add(span, hir::ExpressionData::Error { error })
    }

    fn unit_expression(&mut self, span: Span) -> hir::Expression {
        self.add(span, hir::ExpressionData::Unit {})
    }
}

impl<'me, DB, I> std::ops::Index<I> for HirLower<'me, DB>
where
    DB: HirDatabase,
    I: hir::HirIndex,
{
    type Output = I::Data;

    fn index(&self, index: I) -> &I::Data {
        &self.fn_body_tables[index]
    }
}
