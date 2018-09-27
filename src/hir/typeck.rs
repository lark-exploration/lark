use codespan_reporting::{Diagnostic, Label};
use crate::hir;
use crate::ty;
use std::sync::Arc;

/// A unit type that means "the operation failed and an error
/// has been reported to the user". It implies you should suppress
/// any downstream work.
crate struct ErrorReported;

crate struct MethodSignature<TC: HirTypeChecker> {
    inputs: Arc<Vec<TC::Ty>>,
    output: TC::Ty,
}

crate trait HirTypeChecker: Sized {
    type FieldId: Copy;
    type MethodId: Copy;
    type Ty: Copy;

    /// Return the HIR that we are type-checking.
    fn hir(&self) -> &Arc<hir::Hir>;

    /// Fetch the field of the given field from the given owner,
    /// appropriately substituted.
    fn field_ty(&mut self, owner_ty: Self::Ty, def_id: Self::FieldId) -> Self::Ty;

    /// Given the type of a field and its owner, substitute any generics appropriately
    /// and return an instantiated type.
    fn method_sig(
        &mut self,
        owner_ty: Self::Ty,
        method_def_id: Self::MethodId,
    ) -> MethodSignature<Self>;

    /// Records the computed type for an expression, variable, etc.
    fn record_ty(&mut self, index: impl hir::HirIndex, ty: Self::Ty);

    /// Lookup the type for a variable.
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
    fn with_field(
        &mut self,
        location: impl hir::HirIndex,
        owner_ty: Self::Ty,
        field_name: hir::Identifier,
        op: impl FnOnce(&mut Self, Self::FieldId) -> Result<Self::Ty, ErrorReported>,
    ) -> Self::Ty;

    /// Invokes `op` once the base-data of `ty` is known. The returned
    /// type will be equal to the return type of `op` (though it may be
    /// a type variable if the base-data of `ty` is not known yet).
    fn with_method(
        &mut self,
        location: impl hir::HirIndex,
        owner_ty: Self::Ty,
        method_name: hir::Identifier,
        op: impl FnOnce(&mut Self, Self::MethodId) -> Result<Self::Ty, ErrorReported>,
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

    /// Type-check `expression`, recording and returning the resulting type (which may be
    /// an inference variable).
    fn check_expression(&mut self, expression: hir::Expression) -> Self::Ty {
        let ty = self
            .compute_expression_ty(expression)
            .unwrap_or_else(|ErrorReported| self.error_type());
        self.record_ty(expression, ty);
        ty
    }

    /// Helper for `check_expression`: compute the type of the given expression.
    fn compute_expression_ty(
        &mut self,
        expression: hir::Expression,
    ) -> Result<Self::Ty, ErrorReported> {
        let expression_data = self.hir()[expression].clone();
        match expression_data {
            hir::ExpressionData::Let {
                var,
                initializer,
                body,
            } => {
                let initializer_ty = self.check_expression(initializer);
                self.record_ty(var, initializer_ty);
                Ok(self.check_expression(body))
            }

            hir::ExpressionData::Place { perm, place } => {
                let place_ty = self.check_place(place);
                Ok(self.apply_user_perm(perm, place_ty))
            }

            hir::ExpressionData::Assignment { place, value } => {
                let place_ty = self.check_place(place);
                let value_ty = self.check_expression(value);
                self.require_assignable(expression, value_ty, place_ty);
                Ok(place_ty)
            }

            hir::ExpressionData::MethodCall {
                owner,
                method,
                arguments,
            } => {
                let owner_ty = self.check_place(owner);
                Ok(self.compute_method_call_ty(expression, owner_ty, method, arguments))
            }

            hir::ExpressionData::Sequence { first, second } => {
                let _ = self.check_expression(first);
                Ok(self.check_expression(second))
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
                Ok(self.least_upper_bound(expression, true_ty, false_ty))
            }

            hir::ExpressionData::Unit {} => unimplemented!(),
        }
    }

    /// Type-check `place`, recording and returning the resulting type (which may be
    /// an inference variable).
    fn check_place(&mut self, place: hir::Place) -> Self::Ty {
        let ty = self
            .compute_place_ty(place)
            .unwrap_or_else(|ErrorReported| self.error_type());;
        self.record_ty(place, ty);
        ty
    }

    /// Helper for `check_place`.
    fn compute_place_ty(&mut self, place: hir::Place) -> Result<Self::Ty, ErrorReported> {
        let place_data = self.hir()[place];
        match place_data {
            hir::PlaceData::Variable(var) => Ok(self.variable_ty(var)),

            hir::PlaceData::Temporary(expr) => Ok(self.check_expression(expr)),

            hir::PlaceData::Field { owner, name } => {
                let owner_ty = self.check_place(owner);
                Ok(
                    self.with_field(place, owner_ty, name, move |this, field_id| {
                        Ok(this.field_ty(owner_ty, field_id))
                    }),
                )
            }
        }
    }

    /// Helper for `check_expression`: Compute the type from a method call.
    fn compute_method_call_ty(
        &mut self,
        expression: hir::Expression,
        owner_ty: Self::Ty,
        method_name: hir::Identifier,
        arguments: Arc<Vec<hir::Expression>>,
    ) -> Self::Ty {
        self.with_method(expression, owner_ty, method_name, move |this, method_id| {
            let method_sig = this.method_sig(owner_ty, method_id);
            for (&argument, &expected_ty) in arguments.iter().zip(method_sig.inputs.iter()) {
                let argument_ty = this.check_expression(argument);
                this.require_assignable(expression, argument_ty, expected_ty);
            }
            Ok(method_sig.output)
        })
    }
}
