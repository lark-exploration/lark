use ast::ast as a;
use ast::item_id::ItemId;
use crate as hir;
use crate::HirDatabase;
use map::FxIndexMap;
use parser::pos::{Span, Spanned};
use parser::StringId;
use std::sync::Arc;

crate fn fn_body(db: &impl HirDatabase, item_id: ItemId) -> Arc<crate::FnBody> {
    let lower = HirLower::new(db);
    Arc::new(lower.lower_ast_of_item(item_id))
}

struct HirLower<'db, DB: HirDatabase> {
    db: &'db DB,
    fn_body_tables: hir::FnBodyTables,
    variables: FxIndexMap<StringId, hir::Variable>,
}

impl<'db, DB> HirLower<'db, DB>
where
    DB: HirDatabase,
{
    fn new(db: &'db DB) -> Self {
        HirLower {
            db,
            fn_body_tables: Default::default(),
            variables: Default::default(),
        }
    }

    fn add<D: hir::HirIndexData>(&mut self, span: Span, node: D) -> D::Index {
        D::index_vec_mut(&mut self.fn_body_tables).push(Spanned { span, node })
    }

    fn span(&self, index: impl hir::SpanIndex) -> Span {
        index.span_from(&self.fn_body_tables)
    }

    fn save_scope(&self) -> FxIndexMap<StringId, hir::Variable> {
        self.variables.clone()
    }

    fn restore_scope(&mut self, scope: FxIndexMap<StringId, hir::Variable>) {
        self.variables = scope;
    }

    /// Brings a variable into scope, returning anything that was shadowed.
    fn bring_into_scope(&mut self, variable: hir::Variable) {
        let name = self[variable].name;
        self.variables.insert(self[name].text, variable);
    }

    fn lower_ast_of_item(mut self, item_id: ItemId) -> hir::FnBody {
        match self.db.ast_of_item(item_id) {
            Ok(ast) => match &*ast {
                a::Item::Struct(_) => panic!("asked for fn-body of struct {:?}", item_id),
                a::Item::Def(def) => {
                    let arguments = self.lower_parameters(&def.parameters);

                    for &argument in &arguments {
                        self.bring_into_scope(argument);
                    }

                    let root_expression = self.lower_block(&def.body);

                    hir::FnBody {
                        arguments,
                        root_expression,
                        tables: self.fn_body_tables,
                    }
                }
            },

            Err(parse_error) => {
                let root_expression = self.error_expression(
                    parse_error.span,
                    hir::ErrorData::ParseError {
                        description: parse_error.description,
                    },
                );

                hir::FnBody {
                    arguments: vec![],
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
                    parameter.name.span,
                    hir::IdentifierData {
                        text: parameter.name.node,
                    },
                );
                self.add(parameter.span, hir::VariableData { name })
            })
            .collect()
    }

    fn lower_block(&mut self, block: &Spanned<a::Block>) -> hir::Expression {
        let mut block_expr: hir::Expression = self.add(block.span, hir::ExpressionData::Unit {});
        for block_item in &block.node.expressions {
            if let Some(item_expr) = self.lower_block_item(block_item) {
                let item_expr_span = self.fn_body_tables.span(item_expr);
                block_expr = self.add(
                    item_expr_span,
                    hir::ExpressionData::Sequence {
                        first: block_expr,
                        second: item_expr,
                    },
                );
            }
        }
        block_expr
    }

    fn lower_block_item(&mut self, block_item: &a::BlockItem) -> Option<hir::Expression> {
        match block_item {
            // ignore nested block items
            a::BlockItem::Item(_) => None,

            a::BlockItem::Decl(_decl) => unimplemented!(),

            a::BlockItem::Expr(expr) => Some(self.lower_expression(expr)),
        }
    }

    fn lower_expression(&mut self, expr: &a::Expression) -> hir::Expression {
        match expr {
            a::Expression::Block(block) => self.lower_block(block),

            a::Expression::ConstructStruct(_) => unimplemented!(),

            a::Expression::Call(_) => unimplemented!(),

            a::Expression::Ref(_) => {
                let place = self.lower_place(expr);
                let span = self.span(place);
                let perm = self.add(span, hir::PermData::Default);
                self.add(span, hir::ExpressionData::Place { perm, place })
            }

            a::Expression::Binary(..) => unimplemented!(),

            a::Expression::Interpolation(..) => unimplemented!(),

            a::Expression::Literal(..) => unimplemented!(),
        }
    }

    fn lower_place(&mut self, expr: &a::Expression) -> hir::Place {
        match expr {
            a::Expression::Ref(identifier) => match self.variables.get(&identifier.node) {
                Some(&variable) => self.add(identifier.span, hir::PlaceData::Variable(variable)),

                None => {
                    let error_expression = self.error_expression(
                        identifier.span,
                        hir::ErrorData::UnknownIdentifier {
                            text: identifier.node,
                        },
                    );

                    self.add(identifier.span, hir::PlaceData::Temporary(error_expression))
                }
            },

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

    fn error_expression(&mut self, span: Span, data: hir::ErrorData) -> hir::Expression {
        let error = self.add(span, data);
        self.add(span, hir::ExpressionData::Error { error })
    }
}

impl<'db, DB, I> std::ops::Index<I> for HirLower<'db, DB>
where
    DB: HirDatabase,
    I: hir::HirIndex,
{
    type Output = I::Data;

    fn index(&self, index: I) -> &I::Data {
        &self.fn_body_tables[index]
    }
}
