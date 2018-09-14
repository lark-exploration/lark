use crate::ir::DefId;
use rustc_hash::FxHashMap;
use std::hash::{Hash, Hasher};

crate mod context;
crate mod intern;
crate mod mode;
crate mod query;

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
    crate fn kind(self) -> TyKind<'global> {
        self.data.key.kind
    }

    crate fn generics(self) -> Generics<'global> {
        &self.data.key.generics
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
struct TyData<'global> {
    /// Pre-computed hash.
    hash: u64,

    /// The key in our hashtable.
    key: TyKey<'global>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate struct TyKey<'global> {
    crate kind: TyKind<'global>,
    crate generics: Generics<'global>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate enum TyKind<'global> {
    /// Expects a single type argument. Applies this mode to that.
    Mode { mode: Mode<'global> },

    /// A named type (might be value, might be linear, etc).
    Named { name: TyName },

    /// A "bound" type is a generic parameter that has yet to be
    /// substituted with its value.
    Bound { binder: DebruijnIndex, index: ParameterIndex },

    /// An inference variable in the current context.
    Infer { index: TyInferIndex },

    /// A "placeholder" is what you get when you instantiate a
    /// universally quantified bound variable. For example, `forall<A>
    /// { ... }` -- inside the `...`, the variable `A` might be
    /// replaced with a placeholder, representing "any" type `A`.
    Placeholder { universe: UniverseIndex, index: ParameterIndex },
}

index_type! {
    /// Identifies the binding site where a parameter is bound by counting
    /// *backwards* through the in-scope binderes. In our case, since we
    /// don't have higher-ranked types, or even impls etc, this will
    /// always be INNERMOST, identifying the struct.
    crate struct DebruijnIndex { .. }
}

impl DebruijnIndex {
    crate const INNERMOST: DebruijnIndex = DebruijnIndex::new(0);

    /// Shifts the debruijn index in through a series of binders.
    crate fn shifted_in(self) -> Self {
        DebruijnIndex::new(self.as_usize() + 1)
    }

    /// Shifts the debruijn index out through a series of binders.
    /// Illegal if it represents the innermost binder.
    crate fn shifted_out(self) -> Self {
        assert!(self != Self::INNERMOST, "cannot shift out from innermost binder");
        DebruijnIndex::new(self.as_usize() - 1)
    }

    /// Number of binders in between self and some outer binder `outer`.
    ///
    /// e.g., in `for<X> for<Y> for<Z> T`, `Y.difference(X)` would
    /// yield 1 and `Z.difference(X)` would yield 2.
    crate fn difference(self, outer: Self) -> usize {
        assert!(outer.as_usize() >= self.as_usize(), "outer binder is not outer");
        outer.as_usize() - self.as_usize()
    }
}

index_type! {
    /// Within a given binder, identifies a particular parameter.
    ///
    /// e.g., in `struct Foo<A, B> { x: A }`, `A` would be repesented as
    /// `(INNERMOST, 0)` and `B` would be represented as `(INNERMOST, 1)`.
    crate struct ParameterIndex { .. }
}

index_type! {
    crate struct UniverseIndex { .. }
}

/// The "name" of a type
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate enum TyName {
    DefId(DefId),
}

/// Modes define the following grammar:
///
/// ```
/// Mode = shared(r) Mode
///      | borrow(r)
///      | own
/// ```
///
/// Modes can be normalized against one another. For example,
/// `shared(a) shared(b)` is equivalent to `shared(b)` (and requires
/// that `b: a`).
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate struct Mode<'global> {
    kind: &'global ModeKind<'global>
}

impl Mode<'global> {
    fn kind(self) -> &'global ModeKind<'global> {
        self.kind
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate enum ModeKind<'global> {
    Shared { region: Region, mode: Mode<'global> },
    Borrow { region: Region },
    Owned,
}

index_type! {
    /// A "region" is a kind of marker that we attach to shared/borrowed
    /// values to distinguish them. During borrow checker, we will
    /// associate each region with a set of possible shares/loans that may
    /// have created this value.
    crate struct Region { .. }
}

index_type! {
    crate struct TyInferIndex { .. }
}

index_type! {
    crate struct ModeInferVar { .. }
}

/// The value for a generic parameter.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
crate enum Generic<'global> {
    Ty(Ty<'global>),
}

impl From<Ty<'global>> for Generic<'global> {
    fn from(value: Ty<'global>) -> Generic<'global> {
        Generic::Ty(value)
    }
}

/// A series of values.
crate type Generics<'global> = &'global [Generic<'global>];

//// Definition of a generic parameter.
crate struct GenericParameter {
    role: GenericParameterRole,
}

crate enum GenericParameterRole {
    Owned,
    Associated,
}
