//! A type family where we just erase all permissions and we support inference.

use crate::interners::TyInternTables;
use crate::BaseData;
use crate::Erased;
use crate::InferVarOr;
use crate::Placeholder;
use crate::TypeFamily;
use intern::Has;
use intern::{Intern, Untern};
use unify::{InferVar, Inferable};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BaseOnly;

impl TypeFamily for BaseOnly {
    type Perm = Erased;
    type Base = Base;
    type Placeholder = Placeholder;

    fn intern_base_data(tables: &dyn Has<TyInternTables>, base_data: BaseData<Self>) -> Self::Base {
        tables.intern_tables().intern(InferVarOr::Known(base_data))
    }
}

pub type BaseTy = crate::Ty<BaseOnly>;

indices::index_type! {
    pub struct Base { .. }
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
