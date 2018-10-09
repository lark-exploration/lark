//! A type family where we just erase all permissions and we support inference.

use crate::ty::interners::TyInternTables;
use crate::ty::BaseData;
use crate::ty::Erased;
use crate::ty::Placeholder;
use crate::ty::TypeFamily;
use intern::Has;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct BaseInferred;

impl TypeFamily for BaseInferred {
    type Perm = Erased;
    type Base = Base;
    type Placeholder = Placeholder;

    fn intern_base_data(tables: &dyn Has<TyInternTables>, base_data: BaseData<Self>) -> Self::Base {
        tables.intern_tables().intern(base_data)
    }
}

crate type BaseTy = crate::ty::Ty<BaseInferred>;

indices::index_type! {
    crate struct Base { .. }
}
