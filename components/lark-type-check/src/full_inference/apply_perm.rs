use crate::full_inference::constraint::Constraint;
use crate::full_inference::perm::Perm;
use crate::full_inference::type_checker::FullInferenceStorage;
use crate::full_inference::FullInference;
use crate::TypeCheckDatabase;
use crate::TypeChecker;
use lark_hir as hir;
use lark_intern::Intern;
use lark_ty::BaseData;
use lark_ty::Erased;
use lark_ty::Generic;
use lark_ty::GenericKind;
use lark_ty::Generics;
use lark_ty::Ty;

crate trait ApplyPerm {
    /// Given an access with permission `perm_access` to a value of
    /// type `ty`, returns a new type for the resulting value. This
    /// will have permissions based on `perm_access` and `ty` and add
    /// sufficient constraints to ensure that the result is legal.
    fn apply_access_perm(
        &mut self,
        cause: hir::MetaIndex,
        perm_access: Perm,
        ty: Ty<FullInference>,
    ) -> Ty<FullInference>;

    fn equate_ty(
        &mut self,
        cause: hir::MetaIndex,
        perm_access: Perm,
        ty: Ty<FullInference>,
    ) -> Ty<FullInference>;

    fn equate_generics(
        &mut self,
        cause: hir::MetaIndex,
        perm_access: Perm,
        generics: Generics<FullInference>,
    ) -> Generics<FullInference>;

    fn equate_generic(
        &mut self,
        cause: hir::MetaIndex,
        perm_access: Perm,
        generics: Generic<FullInference>,
    ) -> Generic<FullInference>;
}

impl<DB> ApplyPerm for TypeChecker<'_, DB, FullInference, FullInferenceStorage>
where
    DB: TypeCheckDatabase,
{
    fn apply_access_perm(
        &mut self,
        cause: hir::MetaIndex,
        perm_access: Perm,
        ty: Ty<FullInference>,
    ) -> Ty<FullInference> {
        self.with_base_data(cause, ty.base, move |this, BaseData { kind, generics }| {
            // The resulting type will have the permission
            // `perm_access`, so `perm_access` must be no more than
            // what we started with.
            this.storage.add_constraint(
                cause,
                Constraint::PermPermits {
                    a: ty.perm,
                    b: perm_access,
                },
            );

            let generics1 = this.equate_generics(cause, perm_access, generics);

            Ty {
                perm: perm_access,
                repr: Erased,
                base: BaseData {
                    kind,
                    generics: generics1,
                }
                .intern(this),
            }
        })
    }

    fn equate_ty(
        &mut self,
        cause: hir::MetaIndex,
        perm_access: Perm,
        ty: Ty<FullInference>,
    ) -> Ty<FullInference> {
        self.with_base_data(cause, ty.base, move |this, BaseData { kind, generics }| {
            // Create output perm `perm1` as an inference variable:
            //
            // If `perm_access` winds up being `Borrow` or `Own`, then
            // `perm1` must be equal to the permission from the type.
            //
            // Otherwise, it doesn't matter, as perm-access will only
            // grant shared permission to its contents.
            let perm1 = this.storage.new_inferred_perm(&this.f_tables);
            this.storage.add_constraint(
                cause,
                Constraint::PermEquateConditionally {
                    condition: perm_access,
                    a: ty.perm,
                    b: perm1,
                },
            );

            // Create `generics1` containing the (recursively) equated contents.
            let generics1 = this.equate_generics(cause, perm_access, generics);

            Ty {
                perm: perm_access,
                repr: Erased,
                base: BaseData {
                    kind,
                    generics: generics1,
                }
                .intern(this),
            }
        })
    }

    fn equate_generics(
        &mut self,
        cause: hir::MetaIndex,
        perm_access: Perm,
        generics: Generics<FullInference>,
    ) -> Generics<FullInference> {
        generics
            .iter()
            .map(|generic| self.equate_generic(cause, perm_access, generic))
            .collect()
    }

    fn equate_generic(
        &mut self,
        cause: hir::MetaIndex,
        perm_access: Perm,
        generic: Generic<FullInference>,
    ) -> Generic<FullInference> {
        match generic {
            GenericKind::Ty(ty) => GenericKind::Ty(self.equate_ty(cause, perm_access, ty)),
        }
    }
}
