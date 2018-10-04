#![warn(unused_imports)]

use crate::ir::DefId;
use crate::unify::InferVar;
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::IntoIterator;
use std::sync::Arc;

crate mod base_only;
crate mod debug;
crate mod interners;

crate trait TypeFamily: Copy + Clone + Debug + Eq + Hash {
    type Perm: Copy + Clone + Debug + Eq + Hash;
    type Base: Copy + Clone + Debug + Eq + Hash;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Ty<F: TypeFamily> {
    crate perm: F::Perm,
    crate base: F::Base,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Erased;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct BaseData<F: TypeFamily> {
    crate kind: BaseKind,
    crate generics: Generics<F>,
}

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
crate enum BaseKind {
    /// A named type (might be value, might be linear, etc).
    Named(DefId),

    /// Indicates that a type error was reported.
    Error,
}

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum Generic<F: TypeFamily> {
    Ty(Ty<F>),
}
