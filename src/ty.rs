#![warn(unused_imports)]

use crate::ir::DefId;
use std::iter::IntoIterator;
use std::sync::Arc;

crate mod debug;
mod inferable;
crate mod intern;
crate mod map;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Ty {
    crate perm: Perm,
    crate base: Base,
}

index_type! {
    crate struct Perm { .. }
}

index_type! {
    crate struct Base { .. }
}

index_type! {
    /// A "region" is a kind of marker that we attach to shared/borrowed
    /// values to distinguish them. During borrow checker, we will
    /// associate each region with a set of possible shares/loans that may
    /// have created this value.
    crate struct Region { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum PermData {
    Shared(Region),

    Borrow(Region),

    Own,

    /// A "placeholder" is what you get when you instantiate a
    /// universally quantified bound variable. For example, `forall<A>
    /// { ... }` -- inside the `...`, the variable `A` might be
    /// replaced with a placeholder, representing "any" type `A`.
    Placeholder(Placeholder),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum Inferable<T> {
    Known(T),

    /// An inference variable in the current context.
    Infer(InferVar),

    /// A "bound" type is a generic parameter that has yet to be
    /// substituted with its value.
    Bound(BoundIndex),
}

impl<T> Inferable<T> {
    crate fn assert_known(self) -> T {
        match self {
            Inferable::Known(v) => v,
            _ => panic!("found inferable, expected known value"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct BaseData {
    crate kind: BaseKind,
    crate generics: Generics,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum BaseKind {
    /// A named type (might be value, might be linear, etc).
    Named(DefId),

    /// A "placeholder" is what you get when you instantiate a
    /// universally quantified bound variable. For example, `forall<A>
    /// { ... }` -- inside the `...`, the variable `A` might be
    /// replaced with a placeholder, representing "any" type `A`.
    Placeholder(Placeholder),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct Generics {
    crate elements: Arc<Vec<Generic>>,
}

impl Generics {
    crate fn is_empty(&self) -> bool {
        self.len() == 0
    }

    crate fn is_not_empty(&self) -> bool {
        self.len() != 0
    }

    crate fn len(&self) -> usize {
        self.elements.len()
    }

    crate fn iter(&self) -> impl Iterator<Item = Generic> + '_ {
        self.into_iter()
    }
}

impl std::iter::FromIterator<Generic> for Generics {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Generic>,
    {
        let vec = iter.into_iter().collect();
        Generics {
            elements: Arc::new(vec),
        }
    }
}

impl IntoIterator for &'iter Generics {
    type IntoIter = std::iter::Cloned<std::slice::Iter<'iter, Generic>>;
    type Item = Generic;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter().cloned()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum Generic {
    Ty(Ty),
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
        assert!(
            self != Self::INNERMOST,
            "cannot shift out from innermost binder"
        );
        DebruijnIndex::new(self.as_usize() - 1)
    }

    /// Number of binders in between self and some outer binder `outer`.
    ///
    /// e.g., in `for<X> for<Y> for<Z> T`, `Y.difference(X)` would
    /// yield 1 and `Z.difference(X)` would yield 2.
    crate fn difference(self, outer: Self) -> usize {
        assert!(
            outer.as_usize() >= self.as_usize(),
            "outer binder is not outer"
        );
        outer.as_usize() - self.as_usize()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct BoundIndex {
    crate binder: DebruijnIndex,
    crate index: ParameterIndex,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Placeholder {
    crate universe: UniverseIndex,
    crate index: ParameterIndex,
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

impl UniverseIndex {
    crate const ROOT: UniverseIndex = UniverseIndex::new(0);
}

index_type! {
    crate struct InferVar {
        debug_name["?"],
        ..
    }
}

/// Predicates that can be proven about types.
crate enum Predicate {
    // The two bases are "base-equal".
    BaseBaseEq(Base, Base),

    // The two bases are "repr-equal".
    BaseReprEq(Base, Base),

    // The two permissions are "repr-equal".
    PermReprEq(Perm, Perm),

    RelateTypes {
        direction: Variance,
        ty1: Ty,
        ty2: Ty,
    },

    RelatePerms {
        direction: Permits,
        perm1: Perm,
        perm2: Perm,
    },

    RelateRegions {
        direction: RegionDirection,
        region1: Region,
        region2: Region,
    },

    /// GLB(perm1, perm2) = perm3
    IntersectPerms {
        perm1: Perm,
        perm2: Perm,
        perm3: Perm,
    },

    /// LUB(region1, region2) = region3
    UnionRegions {
        region1: Region,
        region2: Region,
        region3: Region,
    },
}

///
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
crate enum Variance {
    Covariant,
    Contravariant,
    Invariant,
}

impl Variance {
    /// Returns a "permits" relation R1 such that `P1 B1 (R) P2 B1` if `P1 (R1) P2`.
    crate fn permits(self) -> Permits {
        match self {
            Variance::Covariant => {
                // p1 T1 <: p2 T2 if p1: p2
                Permits::Permits
            }

            Variance::Contravariant => {
                // p1 T1 <: p2 T2 if p2: p1
                Permits::PermittedBy
            }

            Variance::Invariant => {
                // p1 T1 == p2 T2 if p2 == p1
                Permits::Equals
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
crate enum Permits {
    /// `P1 permits P2` if, everything you can do with P2, you can also do with P1.
    ///  Alternatively, in terms of the permission lattice, if `P1 >= P2`.
    Permits,

    /// Inverse of `permits`.
    PermittedBy,

    /// Both permits and permitted by.
    Equals,
}

impl Permits {
    fn region_direction(self) -> RegionDirection {
        match self {
            // e.g., `shared(r1) permits shared(r2)` if `r1 <= r2` --
            // in particular, as `r1` grows, it becomes more restrictive.
            // (That implies more loans you must respect, which means
            // less *other things* you can do.
            //
            // If you prefer to think in terms of lifetimes, then
            // `shared(r1) permits shared(r2)` if `r1: r2` for the same
            // reason.
            Permits::Permits => RegionDirection::Subset,
            Permits::PermittedBy => RegionDirection::Superset,
            Permits::Equals => RegionDirection::Equals,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
crate enum RegionDirection {
    /// `R1 <= R2`, where `Rn` are sets of loans that must be
    /// respected.
    Subset,

    Superset,

    Equals,
}
