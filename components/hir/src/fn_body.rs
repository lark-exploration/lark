use ast::ast as a;
use ast::item_id::ItemId;
use crate as hir;
use crate::HirDatabase;
use map::FxIndexMap;
use parser::pos::{Span, Spanned};
use parser::StringId;
use std::sync::Arc;

crate fn fn_body(db: &impl HirDatabase, item_id: ItemId) -> Arc<crate::FnBody> {
    match db.ast_of_item(item_id) {
        Ok(ast) => match &*ast {
            a::Item::Struct(_) => panic!("asked for fn-body of struct {:?}", item_id),
            a::Item::Def(def) => {
                let mut lower = HirLower::default();

                let arguments = lower.lower_parameters(&def.parameters);

                for &argument in &arguments {
                    lower.bring_into_scope(argument);
                }

                let root_expression = lower.lower_block(&def.body);

                Arc::new(hir::FnBody {
                    arguments,
                    root_expression,
                    tables: lower.fn_body_tables,
                })
            }
        },

        Err(_error) => unimplemented!(),
    }
}

#[derive(Default)]
struct HirLower {
    fn_body_tables: hir::FnBodyTables,
    variables: FxIndexMap<StringId, hir::Variable>,
}

impl HirLower {
    fn add<D: hir::HirIndexData>(&mut self, span: Span, node: D) -> D::Index {
        D::index_vec_mut(&mut self.fn_body_tables).push(Spanned { span, node })
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

            a::BlockItem::Expr(expr) => Some(self.lower_expr(expr)),
        }
    }

    fn lower_expr(&mut self, expr: &a::Expression) -> hir::Expression {
        match expr {
            a::Expression::Block(block) => self.lower_block(block),

            a::Expression::ConstructStruct(_) => unimplemented!(),

            a::Expression::Call(_) => unimplemented!(),

            a::Expression::Ref(_) => unimplemented!(),

            a::Expression::Binary(..) => unimplemented!(),

            a::Expression::Interpolation(..) => unimplemented!(),

            a::Expression::Literal(..) => unimplemented!(),
        }
    }
}

impl<I> std::ops::Index<I> for HirLower
where
    I: hir::HirIndex,
{
    type Output = I::Data;

    fn index(&self, index: I) -> &I::Data {
        &self.fn_body_tables[index]
    }
}
