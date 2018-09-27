use codespan::ByteSpan;
use codespan_reporting::{Diagnostic, Label};
use crate::hir;
use crate::parser::pos::{Span, Spanned};
use crate::parser::StringId;
use crate::ty;
use crate::ty::intern::{Interners, TyInterners};
use crate::typeck::{TypeChecker, TypeckFuture};
use std::rc::Rc;

impl TypeChecker {
    fn check_expression(&mut self, expression: hir::Expression) -> ty::Ty {
        let expression_data = &self.hir[expression];
        match expression_data.kind {
            hir::ExpressionKind::Let {
                var,
                initializer,
                body,
            } => {
                let initializer_ty = self.check_expression(initializer);
                self.typed.insert_ty(var, initializer_ty);
                self.check_expression(body)
            }

            hir::ExpressionKind::Place { perm, place } => {
                let place_ty = self.check_place(place);
                self.apply_opt_perm(perm, place_ty)
            }

            hir::ExpressionKind::Assignment { place, value } => {
                let place_ty = self.check_place(place);
                let value_ty = self.check_expression(value);
                self.require_assignable(value_ty, place_ty);
                place_ty
            }

            hir::ExpressionKind::MethodCall {
                owner,
                method,
                ref arguments,
            } => {
                let owner_ty = self.check_place(owner);
                self.check_method_call(expression, owner_ty, method, arguments)
            }

            hir::ExpressionKind::Sequence { first, second } => {
                let _ = self.check_expression(first);
                self.check_expression(second)
            }

            hir::ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                let condition_ty = self.check_expression(condition);
                self.require_boolean(condition_ty);
                let true_ty = self.check_expression(if_true);
                let false_ty = self.check_expression(if_false);
                self.least_upper_bound(true_ty, false_ty)
            }

            hir::ExpressionKind::Unit {} => unimplemented!(),
        }
    }

    fn apply_opt_perm(&mut self, perm: Option<hir::Perm>, place_ty: ty::Ty) -> ty::Ty {
        unimplemented!()
    }

    fn check_place(&mut self, expr: hir::Place) -> ty::Ty {
        unimplemented!()
    }

    fn require_assignable(&mut self, value_ty: ty::Ty, place_ty: ty::Ty) {
        unimplemented!()
    }

    fn require_boolean(&mut self, value_ty: ty::Ty) {
        unimplemented!()
    }

    fn least_upper_bound(&mut self, left_ty: ty::Ty, right_ty: ty::Ty) -> ty::Ty {
        unimplemented!()
    }

    fn check_method_call(
        &mut self,
        expression: hir::Expression,
        owner_ty: ty::Ty,
        method_name: Spanned<StringId>,
        arguments: &Rc<Vec<hir::Expression>>,
    ) -> ty::Ty {
        let arguments = arguments.clone();
        self.with_base_data(owner_ty.base, move |this, base_data| match base_data.kind {
            ty::BaseKind::Named(def_id) => unimplemented!(),
            ty::BaseKind::Placeholder(_) => {
                let span = self.hir[expression].span;
                self.report_error(span, |codespan| {
                    Diagnostic::new_error("cannot invoke method on generic type")
                        .with_label(Label::new_primary(codespan).with_message("cannot invoke"))
                })
            }
        })
    }

    fn report_error(&mut self, span: Span, message: impl FnOnce(ByteSpan) -> Diagnostic) -> ty::Ty {
        self.errors.push(message(span.to_codespan()));
        self.common().error_ty
    }
}
