use crate::ty::interners::TyInternTables;
use crate::ty::map_family::FamilyMapper;
use crate::ty::Ty;
use crate::ty::TypeFamily;
use derive_new::new;
use intern::Has;

#[derive(new)]
crate struct Identity<'me, DB> {
    db: &'me DB,
}

impl<DB, F> FamilyMapper<F, F> for Identity<'_, DB>
where
    DB: Has<TyInternTables>,
    F: TypeFamily,
{
    fn map_ty(&mut self, ty: Ty<F>) -> Ty<F> {
        ty
    }

    fn map_placeholder(&mut self, placeholder: F::Placeholder) -> F::Placeholder {
        placeholder
    }
}

impl<DB> Has<TyInternTables> for Identity<'_, DB>
where
    DB: Has<TyInternTables>,
{
    fn intern_tables(&self) -> &TyInternTables {
        self.db.intern_tables()
    }
}
