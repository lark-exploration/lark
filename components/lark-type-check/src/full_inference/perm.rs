//! The representation of Permissions when doing full inference.

use crate::full_inference::FullInferenceTables;
use lark_debug_derive::DebugWith;
use lark_intern::Intern;
use lark_ty::PermKind;
use lark_ty::Placeholder;

/// An intern'd permission.
lark_collections::index_type! {
    crate struct Perm { .. }
}

lark_debug_with::debug_fallback_impl!(Perm);

lark_collections::index_type! {
    crate struct PermVar { .. }
}

lark_debug_with::debug_fallback_impl!(PermVar);

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
crate enum PermData {
    /// Known value.
    Known(PermKind),

    /// Generic placeholder: used for formal arguments. Not known
    /// precisely *what* it is.
    Placeholder(Placeholder),

    /// Inferred permission: we figure out which permission is needed
    /// based on how the resulting value is used.
    Inferred(PermVar),
}

impl Intern<FullInferenceTables> for PermKind {
    type Key = Perm;

    fn intern(self, interner: &dyn AsRef<FullInferenceTables>) -> Self::Key {
        PermData::Known(self).intern(interner)
    }
}
