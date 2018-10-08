#![warn(unused_imports)]

use crate::ir::DefId;
use crate::ty::interners::HasTyInternTables;
use crate::unify::InferVar;
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::IntoIterator;
use std::sync::Arc;

crate mod base_inferred;
crate mod base_only;
crate mod debug;
crate mod declaration;
crate mod identity;
crate mod interners;
crate mod map_family;
crate mod substitute;

crate trait TypeFamily: Copy + Clone + Debug + Eq + Hash + 'static {
    type Perm: Copy + Clone + Debug + Eq + Hash;
    type Base: Copy + Clone + Debug + Eq + Hash;

    fn intern_base_data(tables: &dyn HasTyInternTables, base_data: BaseData<Self>) -> Self::Base;
}

/// A type is the combination of a *permission* and a *base type*.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Ty<F: TypeFamily> {
    crate perm: F::Perm,
    crate base: F::Base,
}

/// Indicates something that we've opted not to track statically.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Erased;

/// The "base data" for a type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct BaseData<F: TypeFamily> {
    crate kind: BaseKind,
    crate generics: Generics<F>,
}

/// The *kinds* of base types we have on offer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum BaseKind {
    /// A named type (might be value, might be linear, etc).
    Named(DefId),

    /// Indicates that a type error was reported.
    Error,
}

/// Used as the value for inferable things during inference -- either
/// a given `Base` (etc) maps to an inference variable or to some
/// known value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate enum InferVarOr<T> {
    InferVar(InferVar),
    Known(T),
}

impl<T> InferVarOr<T> {
    crate fn assert_known(self) -> T {
        match self {
            InferVarOr::InferVar(_) => panic!("assert_known invoked on infer var"),
            InferVarOr::Known(v) => v,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum PlaceholderOr<T> {
    Placeholder(Placeholder),
    Known(T),
}

/// A "placeholder" represents a dummy type (or permission, etc) meant to represent
/// "any type". It is used when you are "inside" a "forall" binder -- so, for example,
/// when we are type-checking a function like `fn foo<T>`, the `T` is represented by
/// a placeholder.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Placeholder {
    crate universe: Universe,
    crate index: BoundVar,
}

/// A "universe" is a set of names -- the root universe (U(0)) contains all
/// the "global names"; each time we traverse into a binder, we instantiate a
/// new universe (e.g., U(1)) that can see all things from lower universes
/// as well as some new placeholders.
index_type! {
    crate struct Universe {
        debug_name["U"],
        ..
    }
}

impl Universe {
    crate const ROOT: Universe = Universe::from_u32(0);
}

/// A "bound variable" refers to one of the generic type parameters in scope
/// within a declaration. So, for example, if you have `struct Foo<T> { x: T }`,
/// then the bound var #0 would be `T`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate enum BoundVarOr<T> {
    BoundVar(BoundVar),
    Known(T),
}

index_type! {
    crate struct BoundVar { .. }
}

/// A set of generic arguments; e.g., in a type like `Vec<i32>`, this
/// would be `[i32]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct Generics<F: TypeFamily> {
    elements: Option<Arc<Vec<Generic<F>>>>,
}

impl<F: TypeFamily> Generics<F> {
    crate fn empty() -> Self {
        Generics { elements: None }
    }

    crate fn is_empty(&self) -> bool {
        self.len() == 0
    }

    crate fn is_not_empty(&self) -> bool {
        self.len() != 0
    }

    crate fn len(&self) -> usize {
        self.elements.as_ref().map(|v| v.len()).unwrap_or(0)
    }

    crate fn iter(&self) -> impl Iterator<Item = Generic<F>> + '_ {
        self.into_iter()
    }

    crate fn elements(&self) -> &[Generic<F>] {
        match &self.elements {
            Some(e) => &e[..],
            None => &[],
        }
    }
}

impl<F> std::ops::Index<BoundVar> for Generics<F>
where
    F: TypeFamily,
{
    type Output = Generic<F>;

    fn index(&self, index: BoundVar) -> &Self::Output {
        &self.elements()[index.as_usize()]
    }
}

impl<F: TypeFamily> std::iter::FromIterator<Generic<F>> for Generics<F> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Generic<F>>,
    {
        let vec: Vec<Generic<F>> = iter.into_iter().collect();
        if vec.is_empty() {
            Generics { elements: None }
        } else {
            Generics {
                elements: Some(Arc::new(vec)),
            }
        }
    }
}

impl<F: TypeFamily> IntoIterator for &'iter Generics<F> {
    type IntoIter = std::iter::Cloned<std::slice::Iter<'iter, Generic<F>>>;
    type Item = Generic<F>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements().iter().cloned()
    }
}

/// The value of a single generic argument; e.g., in a type like
/// `Vec<i32>`, this might be the `i32`.
#[allow(type_alias_bounds)]
crate type Generic<F: TypeFamily> = GenericKind<Ty<F>>;

/// An enum that lists out the various "kinds" of generic arguments
/// (currently only types) and a distinct type of value for each kind.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum GenericKind<T> {
    Ty(T),
}

impl<T> GenericKind<T> {
    crate fn assert_ty(self) -> T {
        match self {
            GenericKind::Ty(ty) => ty,
        }
    }
}

/// Signature from a function or method: `(T1, T2) -> T3`.  `inputs`
/// are the list of the types of the arguments, and `output` is the
/// return type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct Signature<F: TypeFamily> {
    crate inputs: Arc<Vec<Ty<F>>>,
    crate output: Ty<F>,
}
