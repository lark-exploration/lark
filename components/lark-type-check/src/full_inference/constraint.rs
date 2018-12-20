use crate::full_inference::perm::Perm;
use crate::HirLocation;
use lark_debug_derive::DebugWith;
use lark_hir as hir;

#[derive(Copy, Clone, Hash, Debug, DebugWith, PartialEq, Eq)]
crate enum Constraint {
    /// Perm `a` must be **equivalent** to permission `b`.
    PermEquate { a: Perm, b: Perm },

    /// Perm `Pc[Pa = Pb]` ("Pa permits Pb") means that, if `Pc`
    /// permits mutation (i.e., is borrow or own), then permissions Pa
    /// and Pb must be equal.
    PermEquateConditionally { condition: Perm, a: Perm, b: Perm },

    /// Perm `Pa: Pb` ("Pa permits Pb") means that any reads/writes
    /// you could do through a reference with perm Pa, you could do
    /// through a reference with perm Pb.
    PermPermits { a: Perm, b: Perm },
}

#[derive(Copy, Clone, Hash, Debug, DebugWith, PartialEq, Eq)]
crate struct ConstraintAt {
    crate cause: hir::MetaIndex,
    crate location: HirLocation,
    crate constraint: Constraint,
}
