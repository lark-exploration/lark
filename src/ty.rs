use crate::ir::DefId;
use rustc_hash::FxHashMap;
use std::hash::{Hash, Hasher};

crate mod context;

/// Internal, semantic representation for a Lark type. Derived from
/// the AST, but not equivalent to it. `Ty` values are always interned
/// into the global arenas via a `TyContext`.
#[derive(Copy, Clone)]
crate struct Ty<'global> {
    // Correctness invariant: `Ty` is always interned for uniqueness,
    // so we can rely on pointer equality.
    data: &'global TyData<'global>,
}

impl Ty<'global> {
    crate fn kind(self) -> &'ty TyKind<'global> {
        &self.data.kind
    }
}

impl PartialEq<Ty<'global>> for Ty<'global> {
    fn eq(&self, other: &Ty<'global>) -> bool {
        let ptr1: *const TyData<'global> = self.data;
        let ptr2: *const TyData<'global> = other.data;
        ptr1 == ptr2
    }
}

impl Eq for Ty<'global> {}

impl Hash for Ty<'global> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.data.hash.hash(hasher);
    }
}

/// A "mostly internal" struct containing information about types.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate struct TyData<'global> {
    /// Pre-computed hash.
    hash: u64,

    /// The type-kind.
    kind: TyKind<'global>,
}

/// The "kinds" of types that can appear in Lark. Access from a `Ty`
/// via `ty.kind()`.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate enum TyKind<'global> {
    /// Apply the given mode to the given type.
    Mode { mode: Mode, subty: Ty<'global> },

    /// A "named" type corresponds to something the user declared
    /// (e.g., a struct) as well as built-ins like `u32` or what have
    /// you.
    Named {
        /// Name, like `Vec`.
        name: TyName,

        /// Arguments to the type, if any. e.g., for `Vec<T>` might be
        /// `[T]`.
        kinds: Kinds<'global>,
    },

    /// Inference variables are used during type inference for types
    /// that are not yet known; consult the relevant inference context
    /// to see if this particular variable has been bound and -- if so
    /// -- to what.
    Infer(TyInferVar),
}

/// The "name" of a type
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate enum TyName {
    DefId(DefId),
}

/// "Mode" -- note that "owned" is the default and hence not
/// represented.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate enum Mode {
    Shared(Region),
    Borrow(Region),
    Infer(ModeInferVar),
}

/// A "region" is a kind of marker that we attach to shared/borrowed
/// values to distinguish them. During borrow checker, we will
/// associate each region with a set of possible shares/loans that may
/// have created this value.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate struct Region {
    index: usize,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate struct TyInferVar {
    index: usize,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate struct ModeInferVar {
    index: usize,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate enum Kind<'global> {
    Ty(Ty<'global>),
}

crate type Kinds<'global> = &'global [Kind<'global>];
