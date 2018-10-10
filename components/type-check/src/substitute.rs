use intern::Has;
use ty::declaration::Declaration;
use ty::interners::TyInternTables;
use ty::map_family::FamilyMapper;
use ty::map_family::Map;
use ty::BoundVar;
use ty::BoundVarOr;
use ty::Erased;
use ty::Generic;
use ty::Ty;
use ty::TypeFamily;

crate struct Substitution<'me, T, V>
where
    T: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<T>>,
{
    intern_tables: &'me TyInternTables,
    values: &'me V,
}

impl<T, V> Substitution<'me, T, V>
where
    T: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<T>>,
{
    crate fn new(intern_tables: &'me dyn Has<TyInternTables>, values: &'me V) -> Self {
        Substitution {
            intern_tables: intern_tables.intern_tables(),
            values,
        }
    }
}

impl<T, V> Has<TyInternTables> for Substitution<'me, T, V>
where
    T: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<T>>,
{
    fn intern_tables(&self) -> &TyInternTables {
        &self.intern_tables
    }
}

impl<T, V> FamilyMapper<Declaration, T> for Substitution<'me, T, V>
where
    T: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<T>>,
{
    fn map_ty(&mut self, ty: Ty<Declaration>) -> Ty<T> {
        let Ty { perm: Erased, base } = ty;

        match self.untern(base) {
            BoundVarOr::BoundVar(var) => self.values[var].assert_ty(),

            BoundVarOr::Known(base_data) => {
                let base_data1 = base_data.map(self);
                Ty {
                    perm: Erased,
                    base: T::intern_base_data(self, base_data1),
                }
            }
        }
    }

    fn map_placeholder(&mut self, placeholder: !) -> T::Placeholder {
        placeholder
    }
}
