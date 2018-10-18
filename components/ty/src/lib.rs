#![feature(in_band_lifetimes)]
#![feature(macro_at_most_once_rep)]
#![feature(never_type)]
#![feature(const_fn)]
#![feature(const_let)]
#![warn(unused_imports)]

use crate::interners::TyInternTables;
use indices::IndexVec;
use intern::Has;
use lark_entity::ItemId;
use parser::StringId;
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::IntoIterator;
use std::sync::Arc;
use unify::InferVar;

pub mod base_inferred;
pub mod base_only;
pub mod declaration;
pub mod identity;
pub mod interners;
pub mod map_family;

pub trait TypeFamily: Copy + Clone + Debug + Eq + Hash + 'static {
    type Perm: Copy + Clone + Debug + Eq + Hash;
    type Base: Copy + Clone + Debug + Eq + Hash;

    type Placeholder: Copy + Clone + Debug + Eq + Hash;

    fn intern_base_data(tables: &dyn Has<TyInternTables>, base_data: BaseData<Self>) -> Self::Base;
}

/// A type is the combination of a *permission* and a *base type*.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ty<F: TypeFamily> {
    pub perm: F::Perm,
    pub base: F::Base,
}

/// Indicates something that we've opted not to track statically.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Erased;

/// The "base data" for a type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BaseData<F: TypeFamily> {
    pub kind: BaseKind<F>,
    pub generics: Generics<F>,
}

impl<F: TypeFamily> BaseData<F> {
    pub fn from_placeholder(p: F::Placeholder) -> Self {
        BaseData {
            kind: BaseKind::Placeholder(p),
            generics: Generics::empty(),
        }
    }
}

/// The *kinds* of base types we have on offer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BaseKind<F: TypeFamily> {
    /// A named type (might be value, might be linear, etc).
    Named(ItemId),

    /// Instantiated generic type -- exists only in type-check results
    /// for a function.
    Placeholder(F::Placeholder),

    /// Indicates that a type error was reported.
    Error,
}

/// Used as the value for inferable things during inference -- either
/// a given `Base` (etc) maps to an inference variable or to some
/// known value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum InferVarOr<T> {
    InferVar(InferVar),
    Known(T),
}

impl<T> InferVarOr<T> {
    pub fn assert_known(self) -> T {
        match self {
            InferVarOr::InferVar(_) => panic!("assert_known invoked on infer var"),
            InferVarOr::Known(v) => v,
        }
    }
}

/// A "placeholder" represents a dummy type (or permission, etc) meant to represent
/// "any type". It is used when you are "inside" a "forall" binder -- so, for example,
/// when we are type-checking a function like `fn foo<T>`, the `T` is represented by
/// a placeholder.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Placeholder {
    pub universe: Universe,
    pub bound_var: BoundVar,
}

/// A "universe" is a set of names -- the root universe (U(0)) contains all
/// the "global names"; each time we traverse into a binder, we instantiate a
/// new universe (e.g., U(1)) that can see all things from lower universes
/// as well as some new placeholders.
indices::index_type! {
    pub struct Universe {
        debug_name["U"],
        ..
    }
}

impl Universe {
    pub const ROOT: Universe = Universe::from_u32(0);
}

/// A "bound variable" refers to one of the generic type parameters in scope
/// within a declaration. So, for example, if you have `struct Foo<T> { x: T }`,
/// then the bound var #0 would be `T`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BoundVarOr<T> {
    BoundVar(BoundVar),
    Known(T),
}

indices::index_type! {
    pub struct BoundVar { .. }
}

/// A set of generic arguments; e.g., in a type like `Vec<i32>`, this
/// would be `[i32]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Generics<F: TypeFamily> {
    elements: Option<Arc<Vec<Generic<F>>>>,
}

impl<F: TypeFamily> Generics<F> {
    pub fn empty() -> Self {
        Generics { elements: None }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_not_empty(&self) -> bool {
        self.len() != 0
    }

    pub fn len(&self) -> usize {
        self.elements.as_ref().map(|v| v.len()).unwrap_or(0)
    }

    pub fn iter(&self) -> impl Iterator<Item = Generic<F>> + '_ {
        self.into_iter()
    }

    pub fn elements(&self) -> &[Generic<F>] {
        match &self.elements {
            Some(e) => &e[..],
            None => &[],
        }
    }

    /// Append an item to this vector; if this set of generics is
    /// shared, this will clone the contents so that we own them
    /// privately. (Effectively generic lists are a copy-on-write data
    /// structure.)
    pub fn push(&mut self, generic: Generic<F>) {
        self.extend(std::iter::once(generic));
    }
}

/// Append items to this generics, cloning if it is shared with
/// others. (Generics are effectively a simple persistent vector.)
impl<F> std::iter::Extend<Generic<F>> for Generics<F>
where
    F: TypeFamily,
{
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = Generic<F>>,
    {
        match &mut self.elements {
            None => {
                self.elements = Some(Arc::new(iter.into_iter().collect()));
            }

            Some(arc_vec) => {
                Arc::make_mut(arc_vec).extend(iter);
            }
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
pub type Generic<F: TypeFamily> = GenericKind<Ty<F>>;

/// An enum that lists out the various "kinds" of generic arguments
/// (currently only types) and a distinct type of value for each kind.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GenericKind<T> {
    Ty(T),
}

impl<T> GenericKind<T> {
    pub fn assert_ty(self) -> T {
        match self {
            GenericKind::Ty(ty) => ty,
        }
    }
}

/// Signature from a function or method: `(T1, T2) -> T3`.  `inputs`
/// are the list of the types of the arguments, and `output` is the
/// return type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signature<F: TypeFamily> {
    pub inputs: Arc<Vec<Ty<F>>>,
    pub output: Ty<F>,
}

/// The "generic declarations" list out the generic parameters for a
/// given item. Since items inherit generic items from one another
/// (e.g., from their parents),
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericDeclarations {
    pub parent_item: Option<ItemId>,
    pub declarations: IndexVec<BoundVar, GenericKind<GenericTyDeclaration>>,
}

/// Declaration of an individual generic type parameter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericTyDeclaration {
    pub def_id: ItemId,
    pub name: StringId,
}
