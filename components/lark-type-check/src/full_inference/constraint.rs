use crate::full_inference::perm::Perm;
use lark_debug_derive::DebugWith;
use lark_hir as hir;

#[derive(Copy, Clone, Hash, Debug, DebugWith, PartialEq, Eq)]
crate enum Constraint {
    /// Perm `a` must be **equivalent** to permission `b`.
    PermEquate { a: Perm, b: Perm },
}

#[derive(Copy, Clone, Hash, Debug, DebugWith, PartialEq, Eq)]
crate struct ConstraintAt {
    crate cause: hir::MetaIndex,
    crate constraint: Constraint,
}
