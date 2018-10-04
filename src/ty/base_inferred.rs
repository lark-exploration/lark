//! A type family where we just erase all permissions and we support inference.

use crate::ty::interners::HasTyInternTables;
use crate::ty::BaseData;
use crate::ty::Erased;
use crate::ty::TypeFamily;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct BaseInferred;

impl TypeFamily for BaseInferred {
    type Perm = Erased;
    type Base = Base;

    fn intern_base_data(tables: &dyn HasTyInternTables, base_data: BaseData<Self>) -> Self::Base {
        tables.ty_intern_tables().intern(base_data)
    }
}

crate type BaseTy = crate::ty::Ty<BaseInferred>;

index_type! {
    crate struct Base { .. }
}
