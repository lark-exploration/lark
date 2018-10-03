//! A type family where we just erase all permissions

use crate::ty::Erased;
use crate::ty::TypeFamily;
use crate::unify::InferVar;

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
