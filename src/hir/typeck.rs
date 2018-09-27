use codespan_reporting::{Diagnostic, Label};
use crate::hir;
use crate::ty;
use std::sync::Arc;

/// A unit type that means "the operation failed and an error
/// has been reported to the user". It implies you should suppress
/// any downstream work.
crate struct ErrorReported;

crate trait HirTypeChecker: Sized {
    type Ty: Copy;

    fn hir(&self) -> &Arc<hir::Hir>;

    /// Records that the variable's initializer has the given type.
    fn record_variable_initializer_ty(&mut self, var: hir::Variable, ty: Self::Ty);

    /// Records that the variable has the given type.
    fn variable_ty(&mut self, var: hir::Variable) -> Self::Ty;

    /// Given some permissions supplied by the user (which may be a "default"),
    /// apply them to `place_ty` to yield a new type.
    fn apply_user_perm(&mut self, perm: hir::Perm, place_ty: Self::Ty) -> Self::Ty;

    /// Requires that `value_ty` can be assigned to `place_ty`.
    fn require_assignable(
        &mut self,
        expression: hir::Expression,
        value_ty: Self::Ty,
        place_ty: Self::Ty,
    );

    /// Requires that `value_ty` is a boolean value.
    fn require_boolean(&mut self, expression: hir::Expression, value_ty: Self::Ty);

    /// Compute the least-upper-bound (mutual supertype) of two types
    /// (owing to the given expression) and return the resulting type.
    fn least_upper_bound(
        &mut self,
        if_expression: hir::Expression,
        true_ty: Self::Ty,
        false_ty: Self::Ty,
    ) -> Self::Ty;

    /// Invokes `op` once the base-data of `ty` is known. The returned
    /// type will be equal to the return type of `op` (though it may be
    /// a type variable if the base-data of `ty` is not known yet).
    fn with_base_data(
        &mut self,
        ty: Self::Ty,
        op: impl FnOnce(&mut Self, ty::BaseData) -> Self::Ty,
    ) -> Self::Ty;

    /// Reports an error to the user and returns a special type
    /// that indicates a type error occurred.
    fn report_error(
        &mut self,
        location: impl hir::HirIndex,
        message: impl FnOnce(codespan::ByteSpan) -> Diagnostic,
    ) -> Self::Ty;

    /// Returns a type used to indicate that an error has been reported.
    fn error_type(&mut self) -> Self::Ty;

    fn check_expression(&mut self, expression: hir::Expression) -> Self::Ty {
        let expression_data = self.hir()[expression].clone();
        match expression_data {
            hir::ExpressionData::Let {
                var,
                initializer,
                body,
            } => {
                let initializer_ty = self.check_expression(initializer);
                self.record_variable_initializer_ty(var, initializer_ty);
                self.check_expression(body)
            }

            hir::ExpressionData::Place { perm, place } => {
                let place_ty = self.check_place(place);
                self.apply_user_perm(perm, place_ty)
            }

            hir::ExpressionData::Assignment { place, value } => {
                let place_ty = self.check_place(place);
                let value_ty = self.check_expression(value);
                self.require_assignable(expression, value_ty, place_ty);
                place_ty
            }

            hir::ExpressionData::MethodCall {
                owner,
                method,
                arguments,
            } => {
                let owner_ty = self.check_place(owner);
                self.check_method_call(expression, owner_ty, method, arguments)
            }

            hir::ExpressionData::Sequence { first, second } => {
                let _ = self.check_expression(first);
                self.check_expression(second)
            }

            hir::ExpressionData::If {
                condition,
                if_true,
                if_false,
            } => {
                let condition_ty = self.check_expression(condition);
                self.require_boolean(expression, condition_ty);
                let true_ty = self.check_expression(if_true);
                let false_ty = self.check_expression(if_false);
                self.least_upper_bound(expression, true_ty, false_ty)
            }

            hir::ExpressionData::Unit {} => unimplemented!(),
        }
    }

    fn check_place(&mut self, place: hir::Place) -> Self::Ty {
        let place_data = self.hir()[place];
        match place_data {
            hir::PlaceData::Variable(var) => self.variable_ty(var),

            hir::PlaceData::Temporary(expr) => self.check_expression(expr),

            hir::PlaceData::Field { owner, name } => {
                let owner_ty = self.check_place(owner);
                self.with_base_data(owner_ty, move |this, base_data| match base_data.kind {
                    ty::BaseKind::Named(def_id) => {
                        let fields = self.fields(def_id);
                    }
                    ty::BaseKind::Placeholder(_) => this.report_error(place, |codespan| {
                        Diagnostic::new_error("cannot access field of generic type").with_label(
                            Label::new_primary(codespan)
                                .with_message(format!("cannot access field `{}`", name)),
                        )
                    }),
                    ty::BaseKind::Error => this.error_type(),
                })
            }
        }
    }

    fn check_method_call(
        &mut self,
        expression: hir::Expression,
        owner_ty: Self::Ty,
        _method_name: hir::Identifier,
        _arguments: Arc<Vec<hir::Expression>>,
    ) -> Self::Ty {
        self.with_base_data(owner_ty, move |this, base_data| match base_data.kind {
            ty::BaseKind::Named(_def_id) => unimplemented!(),
            ty::BaseKind::Placeholder(_) => this.report_error(expression, |codespan| {
                Diagnostic::new_error("cannot invoke method on generic type")
                    .with_label(Label::new_primary(codespan).with_message("cannot invoke"))
            }),
            ty::BaseKind::Error => this.error_type(),
        })
    }
}
