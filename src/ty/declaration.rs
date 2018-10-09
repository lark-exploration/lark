//! A type family where we preserve what the user wrote in all cases.
//! We do not support inference and bases and things may map to bound
//! variables from generic declarations.

use crate::ty::interners::HasTyInternTables;
use crate::ty::BaseData;
use crate::ty::BoundVarOr;
use crate::ty::Erased;
use crate::ty::TypeFamily;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Declaration;

impl TypeFamily for Declaration {
    type Perm = Erased; // Not Yet Implemented
    type Base = Base;
    type Placeholder = !;

    fn intern_base_data(tables: &dyn HasTyInternTables, base_data: BaseData<Self>) -> Self::Base {
        tables
            .ty_intern_tables()
            .intern(BoundVarOr::Known(base_data))
    }
}

crate type DeclarationTy = crate::ty::Ty<Declaration>;

indices::index_type! {
    crate struct Base { .. }
}
