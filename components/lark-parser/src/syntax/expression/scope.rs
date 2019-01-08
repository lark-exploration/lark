use crate::parser::Parser;
use crate::syntax::entity::LazyParsedEntityDatabase;
use lark_collections::FxIndexMap;
use lark_debug_with::DebugWith;
use lark_entity::Entity;
use lark_hir as hir;
use lark_intern::{Intern, Untern};
use lark_span::FileName;
use lark_span::Span;
use lark_string::{GlobalIdentifier, GlobalIdentifierTables};
use std::rc::Rc;

crate struct ExpressionScope<'parse> {
    crate db: &'parse dyn LazyParsedEntityDatabase,
    crate item_entity: Entity,

    // FIXME -- we should not need to make *global identifiers* here,
    // but the current HIR requires it. We would need to refactor
    // `hir::Identifier` to take a `Text` instead (and, indeed, we
    // should do so).
    crate variables: Rc<FxIndexMap<GlobalIdentifier, hir::Variable>>,

    crate fn_body_tables: hir::FnBodyTables,
}

impl ExpressionScope<'parse> {
    crate fn span(&self, node: impl hir::SpanIndex) -> Span<FileName> {
        node.span_from(&self.fn_body_tables)
    }

    crate fn save_scope(&self) -> Rc<FxIndexMap<GlobalIdentifier, hir::Variable>> {
        self.variables.clone()
    }

    crate fn restore_scope(&mut self, scope: Rc<FxIndexMap<GlobalIdentifier, hir::Variable>>) {
        self.variables = scope;
    }

    /// Lookup a variable by name.
    crate fn lookup_variable(&self, text: &str) -> Option<hir::Variable> {
        // FIXME -- we should not need to intern this; see
        // definition of `variables` field above for details
        let global_id = text.intern(&self.db);

        self.variables.get(&global_id).cloned()
    }

    /// Brings a variable into scope, returning anything that was shadowed.
    crate fn introduce_variable(&mut self, variable: hir::Variable) {
        let name = self[variable].name;
        let text = self[name].text;
        Rc::make_mut(&mut self.variables).insert(text, variable);
    }

    crate fn add<D: hir::HirIndexData>(&mut self, span: Span<FileName>, value: D) -> D::Index {
        let index = D::index_vec_mut(&mut self.fn_body_tables).push(value);
        let meta_index: hir::MetaIndex = index.into();
        self.fn_body_tables.spans.insert(meta_index, span);

        index
    }

    crate fn report_error_expression(
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

    crate fn already_reported_error_expression(
        &mut self,
        span: Span<FileName>,
        data: hir::ErrorData,
    ) -> hir::Expression {
        let error = self.add(span, data);
        self.add(span, hir::ExpressionData::Error { error })
    }

    crate fn unit_expression(&mut self, span: Span<FileName>) -> hir::Expression {
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
