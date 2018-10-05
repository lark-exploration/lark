use codespan_reporting::{Diagnostic, Label};
use crate::hir;
use crate::ty::declaration::Declaration;
use crate::ty::BaseData;
use crate::ty::BaseKind;
use crate::ty::Signature;
use crate::ty::Ty;
use crate::ty::TypeFamily;
use std::sync::Arc;

/// A unit type that means "the operation failed and an error
/// has been reported to the user". It implies you should suppress
/// any downstream work.
crate struct ErrorReported;

crate trait HirTypeChecker<DB: hir::HirQueries, F: TypeFamily>: Sized {
    type FieldId: Copy + 'static;
    type MethodId: Copy + 'static;

    /// Return the query database.
    fn db(&self) -> &DB;

    /// Return the HIR that we are type-checking.
    fn hir(&self) -> &Arc<hir::FnBody>;

    /// Report an error at the given location. Eventually we should include
    /// a bit more detail about what sort of error it is. =)
    fn report_error(&mut self, location: impl hir::HirIndex);

    /// Once the base data of `owner_ty` is known, invoke `op` with it
    /// to create a derived type. (If the base data of `owner_ty` is not
    /// immediately known, then create a type variable and return immediately,
    /// enqueueing a call to `op` for later.)
    fn with_base_data(
        &mut self,
        cause: impl hir::HirIndex,
        base: F::Base,
        op: impl FnOnce(&mut Self, BaseData<F>) -> Ty<F> + 'static,
    ) -> Ty<F>;

    /// Fetch the field of the given field from the given owner,
    /// appropriately substituted.
    fn substitute_ty(
        &mut self,
        location: impl hir::HirIndex,
        owner_ty: Ty<F>,
        field_decl_ty: Ty<Declaration>,
    ) -> Ty<F>;

    /// Fetch the field of the given field from the given owner,
    /// appropriately substituted.
    fn substitute_signature(
        &mut self,
        location: impl hir::HirIndex,
        owner_ty: Ty<F>,
        field_decl_ty: Signature<Declaration>,
    ) -> Signature<F>;

    /// Records the computed type for an expression, variable, etc.
    fn record_ty(&mut self, index: impl hir::HirIndex, ty: Ty<F>);

    /// Lookup the type for a variable.
    fn variable_ty(&mut self, var: hir::Variable) -> Ty<F>;

    /// Given some permissions supplied by the user (which may be a "default"),
    /// apply them to `place_ty` to yield a new type.
    fn apply_user_perm(&mut self, perm: hir::Perm, place_ty: Ty<F>) -> Ty<F>;

    /// Requires that `value_ty` can be assigned to `place_ty`.
    fn require_assignable(&mut self, expression: hir::Expression, value_ty: Ty<F>, place_ty: Ty<F>);

    /// Requires that `value_ty` is a boolean value.
    fn require_boolean(&mut self, expression: hir::Expression, value_ty: Ty<F>);

    /// Compute the least-upper-bound (mutual supertype) of two types
    /// (owing to the given expression) and return the resulting type.
    fn least_upper_bound(
        &mut self,
        if_expression: hir::Expression,
        true_ty: Ty<F>,
        false_ty: Ty<F>,
    ) -> Ty<F>;

    /// Returns a type used to indicate that an error has been reported.
    fn error_type(&mut self) -> Ty<F>;

    fn check_expression_has_type(&mut self, expected_ty: Ty<F>, expression: hir::Expression) {
        let actual_ty = self.check_expression(expression);
        self.require_assignable(expression, actual_ty, expected_ty);
    }

    /// Type-check `expression`, recording and returning the resulting type (which may be
    /// an inference variable).
    fn check_expression(&mut self, expression: hir::Expression) -> Ty<F> {
        let ty = self.compute_expression_ty(expression);
        self.record_ty(expression, ty);
        ty
    }

    /// Helper for `check_expression`: compute the type of the given expression.
    fn compute_expression_ty(&mut self, expression: hir::Expression) -> Ty<F> {
        let expression_data = self.hir()[expression].clone();
        match expression_data {
            hir::ExpressionData::Let {
                var,
                initializer,
                body,
            } => {
                let initializer_ty = self.check_expression(initializer);
                self.record_ty(var, initializer_ty);
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
                self.compute_method_call_ty(expression, owner_ty, method, arguments)
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

    /// Type-check `place`, recording and returning the resulting type (which may be
    /// an inference variable).
    fn check_place(&mut self, place: hir::Place) -> Ty<F> {
        let ty = self.compute_place_ty(place);
        self.record_ty(place, ty);
        ty
    }

    /// Helper for `check_place`.
    fn compute_place_ty(&mut self, place: hir::Place) -> Ty<F> {
        let place_data = self.hir()[place];
        match place_data {
            hir::PlaceData::Variable(var) => self.variable_ty(var),

            hir::PlaceData::Temporary(expr) => self.check_expression(expr),

            hir::PlaceData::Field { owner, name } => {
                let text = self.hir()[name].text;
                let owner_ty = self.check_place(owner);
                self.with_base_data(place, owner_ty.base, move |this, base_data| {
                    let BaseData { kind, generics } = base_data;
                    match kind {
                        BaseKind::Named(def_id) => {
                            if let Some(field_def_id) = self.db().member_def_id().get((
                                def_id,
                                hir::MemberKind::Field,
                                text,
                            )) {
                                let field_decl_ty = this.db().ty().get(field_def_id);
                                self.substitute_ty(place, owner_ty, field_decl_ty)
                            } else {
                                this.report_error(place);
                                this.error_type()
                            }
                        }

                        BaseKind::Error => this.error_type(),
                    }
                })
            }
        }
    }

    /// Helper for `check_expression`: Compute the type from a method call.
    fn compute_method_call_ty(
        &mut self,
        expression: hir::Expression,
        owner_ty: Ty<F>,
        method_name: hir::Identifier,
        arguments: Arc<Vec<hir::Expression>>,
    ) -> Ty<F> {
        self.with_base_data(expression, owner_ty.base, move |this, base_data| {
            this.check_method_call(expression, owner_ty, method_name, arguments, base_data)
        })
    }

    fn check_method_call(
        &mut self,
        expression: hir::Expression,
        owner_ty: Ty<F>,
        method_name: hir::Identifier,
        arguments: Arc<Vec<hir::Expression>>,
        base_data: BaseData<F>,
    ) -> Ty<F> {
        let BaseData { kind, generics } = base_data;
        match kind {
            BaseKind::Named(def_id) => {
                let text = self.hir()[method_name].text;
                let method_def_id =
                    match self
                        .db()
                        .member_def_id()
                        .get((def_id, hir::MemberKind::Method, text))
                    {
                        Some(def_id) => def_id,
                        None => {
                            self.report_error(expression);
                            return self.error_type();
                        }
                    };
                let signature_decl = self.db().signature().get(method_def_id);
                let signature = self.substitute_signature(expression, owner_ty, signature_decl);
                if signature.inputs.len() != arguments.len() {
                    self.report_error(expression);
                }
                for (&expected_ty, &argument_expr) in signature.inputs.iter().zip(arguments.iter())
                {
                    self.check_expression_has_type(expected_ty, argument_expr);
                }
                signature.output
            }

            BaseKind::Error => self.error_type(),
        }
    }
}
