//! A type family where we preserve what the user wrote in all cases.
//! We do not support inference.

use crate::ty::Erased;
use crate::ty::TypeFamily;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Declaration;

impl TypeFamily for Declaration {
    type Perm = Erased; // Not Yet Implemented
    type Base = Base;
}

crate type DeclarationTy = crate::ty::Ty<Declaration>;

index_type! {
    crate struct Base { .. }
}
