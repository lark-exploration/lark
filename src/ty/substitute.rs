use crate::declaration::Declaration;
use crate::map_family;
use crate::ty;
use crate::ty::BoundVarOr;

#[derive(new)]
crate struct Substitution<'me, T: TypeFamily> {
    intern_tables: TyInternTables,
    values: &'me IndexVec<BoundVar, ty::Generic<T>>,
}

impl<T> HasTyInternTables for Substitution<'me, T>
where
    T: TypeFamily,
{
    fn ty_intern_tables(&self) -> &TyInternTables {
        &self.intern_tables
    }
}

impl<T> map_family::Mapper for Substitution<'me, T>
where
    T: TypeFamily<Perm = ty::Erased>,
{
    type Source = Declaration;
    type Target = T;

    fn map_ty(&self, ty: Ty<Declaration>) -> Ty<T> {
        let Ty {
            perm: ty::Erased,
            base,
        } = ty;

        match base.untern(&self.intern_tables) {
            BoundVarOr::BoundVar(var) => self.values[var].assert_ty(),

            BoundVarOr::Known(base_data) => {
                let base_data1 = base_data.map(self);
                Ty {
                    perm: ty::Erased,
                    base: T::intern_base_data(self, base_data1),
                }
            }
        }
    }
}
