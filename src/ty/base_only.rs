//! A type family where we just erase all permissions and we support inference.

use crate::intern::{Intern, Untern};
use crate::ty::interners::HasTyInternTables;
use crate::ty::interners::TyInternTables;
use crate::ty::BaseData;
use crate::ty::Erased;
use crate::ty::InferVarOr;
use crate::ty::PlaceholderOr;
use crate::ty::TypeFamily;
use crate::unify::{InferVar, Inferable};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct BaseOnly;

impl TypeFamily for BaseOnly {
    type Perm = Erased;
    type Base = Base;

    fn intern_base_data(tables: &dyn HasTyInternTables, base_data: BaseData<Self>) -> Self::Base {
        tables
            .ty_intern_tables()
            .intern(InferVarOr::Known(PlaceholderOr::Known(base_data)))
    }
}

crate type BaseTy = crate::ty::Ty<BaseOnly>;

index_type! {
    crate struct Base { .. }
}

impl Inferable<TyInternTables> for Base {
    type KnownData = PlaceholderOr<BaseData<BaseOnly>>;
    type Data = InferVarOr<PlaceholderOr<BaseData<BaseOnly>>>;

    /// Check if this is an inference variable and return the inference
    /// index if so.
    fn as_infer_var(self, interners: &TyInternTables) -> Option<InferVar> {
        match self.untern(interners) {
            InferVarOr::InferVar(var) => Some(var),
            InferVarOr::Known(_) => None,
        }
    }

    /// Create an inferable representing the inference variable `var`.
    fn from_infer_var(var: InferVar, interners: &TyInternTables) -> Self {
        let i: InferVarOr<PlaceholderOr<BaseData<BaseOnly>>> = InferVarOr::InferVar(var);
        i.intern(interners)
    }

    /// Asserts that this is not an inference variable and returns the
    /// "known data" that it represents.
    fn assert_known(self, interners: &TyInternTables) -> Self::KnownData {
        self.untern(interners).assert_known()
    }
}
