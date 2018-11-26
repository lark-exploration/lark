use intern::Untern;
use lark_ty::declaration;
use lark_ty::declaration::Declaration;
use lark_ty::declaration::DeclarationTables;
use lark_ty::map_family::FamilyMapper;
use lark_ty::map_family::Map;
use lark_ty::BoundVar;
use lark_ty::BoundVarOr;
use lark_ty::Generic;
use lark_ty::ReprKind;
use lark_ty::Ty;
use lark_ty::TypeFamily;

crate struct Substitution<'me, F, V>
where
    F: TypeFamily,
    V: std::ops::Index<BoundVar, Output = Generic<F>>,
{
    delegate: &'me mut dyn SubstitutionDelegate<F>,
    values: &'me V,
}

crate trait SubstitutionDelegate<F: TypeFamily>: AsRef<DeclarationTables> {
    // FIXME(rust-lang/rust#56229) -- can't use `AsRef` supertrait here due to ICE
    fn as_f_tables(&self) -> &F::InternTables;

    fn map_repr_perm(&mut self, repr: ReprKind, perm: declaration::Perm) -> (F::Repr, F::Perm);

    fn apply_repr_perm(&mut self, repr: ReprKind, perm: declaration::Perm, ty: Ty<F>) -> Ty<F>;
}

impl<F, V> Substitution<'me, F, V>
where
    F: TypeFamily,
    V: std::ops::Index<BoundVar, Output = Generic<F>>,
{
    crate fn new(delegate: &'me mut dyn SubstitutionDelegate<F>, values: &'me V) -> Self {
        Substitution { delegate, values }
    }
}

impl<F, V> AsRef<DeclarationTables> for Substitution<'me, F, V>
where
    F: TypeFamily,
    V: std::ops::Index<BoundVar, Output = Generic<F>>,
{
    fn as_ref(&self) -> &DeclarationTables {
        &self.delegate.as_ref()
    }
}

impl<F, V> FamilyMapper<Declaration, F> for Substitution<'me, F, V>
where
    F: TypeFamily,
    V: std::ops::Index<BoundVar, Output = Generic<F>>,
{
    fn map_ty(&mut self, ty: Ty<Declaration>) -> Ty<F> {
        let Ty { repr, perm, base } = ty;

        match base.untern(self) {
            BoundVarOr::BoundVar(var) => {
                // This corresponds to something like `own T`.
                let g = self.values[var].assert_ty();
                self.delegate.apply_repr_perm(repr, perm, g)
            }

            BoundVarOr::Known(base_data) => {
                let base_data1 = base_data.map(self);
                let (repr1, perm1) = self.delegate.map_repr_perm(repr, perm);
                Ty {
                    repr: repr1,
                    perm: perm1,
                    base: F::intern_base_data(self.delegate.as_f_tables(), base_data1),
                }
            }
        }
    }

    fn map_placeholder(&mut self, placeholder: !) -> F::Placeholder {
        placeholder
    }
}
