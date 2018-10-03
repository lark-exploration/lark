//! A type family where we just erase all permissions

use crate::intern::{Intern, Untern};
use crate::ty::interners::TyInternTables;
use crate::ty::BaseData;
use crate::ty::BaseKind;
use crate::ty::Erased;
use crate::ty::Generics;
use crate::ty::TypeFamily;
use crate::unify::{InferVar, Inferable};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct BaseOnly;

impl TypeFamily for BaseOnly {
    type Perm = Erased;
    type Base = Base;
    type Placeholder = !; // not implementing generics yet
    type InferVar = InferVar;
}

crate type BaseTy = crate::ty::Ty<BaseOnly>;

index_type! {
    crate struct Base { .. }
}

impl Inferable<TyInternTables> for Base {
    type KnownData = BaseData<BaseOnly>;
    type Data = BaseData<BaseOnly>;

    /// Check if this is an inference variable and return the inference
    /// index if so.
    fn as_infer_var(self, interners: &TyInternTables) -> Option<InferVar> {
        match self.untern(interners).kind {
            BaseKind::InferVar(var) => Some(var),
            _ => None,
        }
    }

    /// Create an inferable representing the inference variable `var`.
    fn from_infer_var(var: InferVar, interners: &TyInternTables) -> Self {
        BaseData {
            kind: BaseKind::InferVar(var),
            generics: Generics::empty(),
        }.intern(interners)
    }

    /// Asserts that this is not an inference variable and returns the
    /// "known data" that it represents.
    fn assert_known(self, interners: &TyInternTables) -> Self::KnownData {
        self.untern(interners)
    }
}
