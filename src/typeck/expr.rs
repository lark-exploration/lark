use crate::hir;
use crate::ty;
use crate::typeck::{TypeChecker, TypeckFuture};

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

            hir::Assignment { place, value } => {
                let place_ty = self.check_place(place);
                let value_ty = self.check_expression(value);
                self.require_assignable(value_ty, place_ty);
                place_ty
            }
        }
    }

    fn apply_opt_perm(&mut self, perm: Option<hir::Perm>, place_ty: ty::Ty) -> ty::Ty {
        unimplemented!()
    }

    fn check_place(&mut self, expr: hir::Expression) -> ty::Ty {
        unimplemented!()
    }

    fn require_assignable(&mut self, value_ty: ty::Ty, place_ty: ty::Ty) {
        unimplemented!()
    }
}
