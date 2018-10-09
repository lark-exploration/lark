//! A type family where we just erase all permissions and we support inference.

use crate::ty::interners::TyInternTables;
use crate::ty::BaseData;
use crate::ty::Erased;
use crate::ty::InferVarOr;
use crate::ty::Placeholder;
use crate::ty::TypeFamily;
use crate::unify::{InferVar, Inferable};
use intern::Has;
use intern::{Intern, Untern};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct BaseOnly;

impl TypeFamily for BaseOnly {
    type Perm = Erased;
    type Base = Base;
    type Placeholder = Placeholder;

    fn intern_base_data(tables: &dyn Has<TyInternTables>, base_data: BaseData<Self>) -> Self::Base {
        tables.intern_tables().intern(InferVarOr::Known(base_data))
    }
}

crate type BaseTy = crate::ty::Ty<BaseOnly>;

indices::index_type! {
    crate struct Base { .. }
}

impl Inferable<TyInternTables> for Base {
    type KnownData = BaseData<BaseOnly>;
    type Data = InferVarOr<BaseData<BaseOnly>>;

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
        let i: InferVarOr<BaseData<BaseOnly>> = InferVarOr::InferVar(var);
        i.intern(interners)
    }

    /// Asserts that this is not an inference variable and returns the
    /// "known data" that it represents.
    fn assert_known(self, interners: &TyInternTables) -> Self::KnownData {
        self.untern(interners).assert_known()
    }
}
