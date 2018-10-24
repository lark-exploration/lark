use crate::TypeCheckDatabase;
use crate::TypeCheckFamily;
use crate::TypeChecker;
use crate::TypeCheckerFields;
use hir;
use lark_entity::MemberKind;
use std::sync::Arc;
use ty::Ty;
use ty::{BaseData, BaseKind};

impl<DB, F> TypeChecker<'_, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeCheckFamily,
{
    pub(super) fn check_fn_body(&mut self) {
        let signature = self.db.signature(self.fn_entity);
        let placeholders = self.placeholders_for(self.fn_entity);
        let signature = self.substitute(self.hir.root_expression, &placeholders, signature);
        assert_eq!(signature.inputs.len(), self.hir.arguments.len());
        for (&argument, &input) in self.hir.arguments.iter().zip(signature.inputs.iter()) {
            self.results.record_ty(argument, input);
        }
        self.check_expression_has_type(signature.output, self.hir.root_expression);
    }

    fn check_expression_has_type(&mut self, expected_ty: Ty<F>, expression: hir::Expression) {
        let actual_ty = self.check_expression(expression);
        self.require_assignable(expression, actual_ty, expected_ty);
    }

    /// Type-check `expression`, recording and returning the resulting type (which may be
    /// an inference variable).
    fn check_expression(&mut self, expression: hir::Expression) -> Ty<F> {
        let ty = self.compute_expression_ty(expression);
        self.results.record_ty(expression, ty);
        ty
    }

    /// Helper for `check_expression`: compute the type of the given expression.
    fn compute_expression_ty(&mut self, expression: hir::Expression) -> Ty<F> {
        let expression_data = self.hir[expression].clone();
        match expression_data {
            hir::ExpressionData::Let {
                variable,
                initializer: Some(initializer),
                body,
            } => {
                let initializer_ty = self.check_expression(initializer);
                self.results.record_ty(variable, initializer_ty);
                self.check_expression(body)
            }

            hir::ExpressionData::Let {
                variable: _,
                initializer: None,
                body: _,
            } => {
                unimplemented!() // FIXME
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
                self.require_assignable(expression, condition_ty, self.boolean_type());
                let true_ty = self.check_expression(if_true);
                let false_ty = self.check_expression(if_false);
                self.least_upper_bound(expression, true_ty, false_ty)
            }

            hir::ExpressionData::Unit {} => unimplemented!(),

            hir::ExpressionData::Error { error: _ } => self.error_type(),
        }
    }

    /// Type-check `place`, recording and returning the resulting type (which may be
    /// an inference variable).
    fn check_place(&mut self, place: hir::Place) -> Ty<F> {
        let ty = self.compute_place_ty(place);
        self.results.record_ty(place, ty);
        ty
    }

    /// Helper for `check_place`.
    fn compute_place_ty(&mut self, place: hir::Place) -> Ty<F> {
        let place_data = self.hir[place];
        match place_data {
            hir::PlaceData::Variable(var) => self.results.ty(var),

            hir::PlaceData::Temporary(expr) => self.check_expression(expr),

            hir::PlaceData::Field { owner, name } => {
                let text = self.hir[name].text;
                let owner_ty = self.check_place(owner);
                self.with_base_data(place, owner_ty.base.into(), move |this, base_data| {
                    let BaseData { kind, generics } = base_data;
                    match kind {
                        BaseKind::Named(def_id) => {
                            match this.db().member_entity(def_id, MemberKind::Field, text) {
                                Some(field_entity) => {
                                    let field_decl_ty = this.db().ty(field_entity).into_value();
                                    let field_ty = this.substitute(place, &generics, field_decl_ty);
                                    this.apply_owner_perm(place, owner_ty.perm, field_ty)
                                }

                                None => {
                                    this.results.record_error(place);
                                    this.error_type()
                                }
                            }
                        }

                        BaseKind::Placeholder(_placeholder) => unimplemented!(),

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
        self.with_base_data(expression, owner_ty.base.into(), move |this, base_data| {
            this.check_method_call(expression, owner_ty, method_name, arguments, base_data)
        })
    }

    fn check_method_call(
        &mut self,
        expression: hir::Expression,
        _owner_ty: Ty<F>,
        method_name: hir::Identifier,
        arguments: Arc<Vec<hir::Expression>>,
        base_data: BaseData<F>,
    ) -> Ty<F> {
        let BaseData { kind, generics } = base_data;
        match kind {
            BaseKind::Named(def_id) => {
                let text = self.hir[method_name].text;
                let method_entity = match self.db().member_entity(def_id, MemberKind::Method, text)
                {
                    Some(def_id) => def_id,
                    None => {
                        self.results.record_error(expression);
                        return self.error_type();
                    }
                };

                // FIXME -- what role does `owner_ty` place here??

                let signature_decl = self.db().signature(method_entity);
                let signature = self.substitute(expression, &generics, signature_decl);
                if signature.inputs.len() != arguments.len() {
                    self.results.record_error(expression);
                }
                for (&expected_ty, &argument_expr) in signature.inputs.iter().zip(arguments.iter())
                {
                    self.check_expression_has_type(expected_ty, argument_expr);
                }
                signature.output
            }

            BaseKind::Placeholder(_placeholder) => unimplemented!(),

            BaseKind::Error => self.error_type(),
        }
    }
}
