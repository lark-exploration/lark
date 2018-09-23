#![warn(unused_imports)]

use crate::ir::DefId;
use std::iter::IntoIterator;
use std::rc::Rc;

crate mod context;
crate mod intern;
crate mod map;
crate mod query;
crate mod unify;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Ty {
    perm: Perm,
    base: Base,
}

index_type! {
    crate struct Perm { .. }
}

index_type! {
    crate struct Base { .. }
}

index_type! {
    crate struct Generics { .. }
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
    Shared {
        region: Region,
    },
    Borrow {
        region: Region,
    },
    Own,
    Infer {
        var: InferVar,
    },
    Bound {
        binder: DebruijnIndex,
        index: ParameterIndex,
    },
    Placeholder {
        index: Placeholder,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct BaseData {
    crate kind: BaseKind,
    crate generics: Generics,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum BaseKind {
    /// A named type (might be value, might be linear, etc).
    Named { name: DefId },

    /// An inference variable in the current context.
    Infer { var: InferVar },

    /// A "bound" type is a generic parameter that has yet to be
    /// substituted with its value.
    Bound {
        binder: DebruijnIndex,
        index: ParameterIndex,
    },

    /// A "placeholder" is what you get when you instantiate a
    /// universally quantified bound variable. For example, `forall<A>
    /// { ... }` -- inside the `...`, the variable `A` might be
    /// replaced with a placeholder, representing "any" type `A`.
    Placeholder { placeholder: Placeholder },
}

#[derive(Clone, PartialEq, Eq, Hash)]
crate struct GenericsData {
    crate elements: Rc<Vec<Generic>>,
}

impl GenericsData {
    crate fn len(&self) -> usize {
        self.elements.len()
    }

    crate fn iter(&self) -> impl Iterator<Item = Generic> + '_ {
        self.into_iter()
    }
}

impl IntoIterator for &'iter GenericsData {
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
    crate struct InferVar { .. }
}

crate trait AsInferVar {
    fn as_infer_var(&self) -> Option<InferVar>;
}

impl AsInferVar for PermData {
    fn as_infer_var(&self) -> Option<InferVar> {
        if let PermData::Infer { var } = self {
            Some(*var)
        } else {
            None
        }
    }
}

impl AsInferVar for BaseData {
    fn as_infer_var(&self) -> Option<InferVar> {
        if let BaseKind::Infer { var } = self.kind {
            Some(var)
        } else {
            None
        }
    }
}

/// Predicates that can be proven about types.
crate enum Predicate {
    BaseEq(Base, Base),
    RegionConstraint {},
}
