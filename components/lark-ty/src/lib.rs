#![feature(in_band_lifetimes)]
#![feature(macro_at_most_once_rep)]
#![feature(never_type)]
#![feature(specialization)]
#![feature(const_fn)]
#![feature(const_let)]
#![warn(unused_imports)]

use debug::DebugWith;
use indices::IndexVec;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_seq::Seq;
use lark_string::global::GlobalIdentifier;
use lark_unify::InferVar;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::iter::IntoIterator;
use std::sync::Arc;

pub mod base_inferred;
pub mod base_only;
pub mod declaration;
pub mod identity;
pub mod map_family;

pub use self::declaration::Declaration;

pub trait TypeFamily: Copy + Clone + Debug + DebugWith + Eq + Hash + 'static {
    type InternTables: AsRef<Self::InternTables>;

    type Perm: Copy + Clone + Debug + DebugWith + Eq + Hash;
    type Base: Copy + Clone + Debug + DebugWith + Eq + Hash;

    type Placeholder: Copy + Clone + Debug + DebugWith + Eq + Hash;

    fn intern_base_data(
        tables: &dyn AsRef<Self::InternTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base;

    fn own_perm(tables: &dyn AsRef<Self::InternTables>) -> Self::Perm;

    fn error_type(tables: &dyn AsRef<Self::InternTables>) -> Ty<Self> {
        Ty {
            perm: Self::own_perm(tables),
            base: Self::error_base_data(tables),
        }
    }

    fn error_base_data(tables: &dyn AsRef<Self::InternTables>) -> Self::Base {
        Self::intern_base_data(
            tables,
            BaseData {
                kind: BaseKind::Error,
                generics: Generics::empty(),
            },
        )
    }
}

/// A type is the combination of a *permission* and a *base type*.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct Ty<F: TypeFamily> {
    pub perm: F::Perm,
    pub base: F::Base,
}

impl<DB, F> ErrorSentinel<&DB> for Ty<F>
where
    DB: AsRef<F::InternTables>,
    F: TypeFamily,
{
    fn error_sentinel(db: &DB, _report: ErrorReported) -> Self {
        F::error_type(db)
    }
}

/// Indicates something that we've opted not to track statically.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct Erased;

/// The "base data" for a type.
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
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
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum BaseKind<F: TypeFamily> {
    /// A named type (might be value, might be linear, etc).
    Named(Entity),

    /// Instantiated generic type -- exists only in type-check results
    /// for a function.
    Placeholder(F::Placeholder),

    /// Indicates that a type error was reported.
    Error,
}

/// Used as the value for inferable things during inference -- either
/// a given `Base` (etc) maps to an inference variable or to some
/// known value.
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
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
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
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

debug::debug_fallback_impl!(Universe);

impl Universe {
    pub const ROOT: Universe = Universe::from_u32(0);
}

/// A "bound variable" refers to one of the generic type parameters in scope
/// within a declaration. So, for example, if you have `struct Foo<T> { x: T }`,
/// then the bound var #0 would be `T`.
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum BoundVarOr<T> {
    BoundVar(BoundVar),
    Known(T),
}

indices::index_type! {
    pub struct BoundVar { .. }
}

debug::debug_fallback_impl!(BoundVar);

/// A set of generic arguments; e.g., in a type like `Vec<i32>`, this
/// would be `[i32]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Generics<F: TypeFamily> {
    elements: Seq<Generic<F>>,
}

impl<F: TypeFamily> Generics<F> {
    pub fn empty() -> Self {
        Generics {
            elements: Seq::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_not_empty(&self) -> bool {
        self.len() != 0
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = Generic<F>> + '_ {
        self.into_iter()
    }

    pub fn elements(&self) -> &[Generic<F>] {
        &self.elements[..]
    }

    /// Append an item to this vector; if this set of generics is
    /// shared, this will clone the contents so that we own them
    /// privately. (Effectively generic lists are a copy-on-write data
    /// structure.)
    pub fn push(&mut self, generic: Generic<F>) {
        self.extend(std::iter::once(generic));
    }
}

impl<F: TypeFamily> DebugWith for Generics<F> {
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_list()
            .entries(self.elements().iter().map(|e| e.debug_with(cx)))
            .finish()
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
        self.elements.extend(iter);
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
        Generics {
            elements: Seq::from_iter(iter),
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
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
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
///
/// Note: the signature of a method *includes* the `self` type.
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct Signature<F: TypeFamily> {
    pub inputs: Seq<Ty<F>>,
    pub output: Ty<F>,
}

impl<F: TypeFamily> Signature<F> {
    pub fn error_sentinel(tables: &dyn AsRef<F::InternTables>, num_inputs: usize) -> Signature<F> {
        Signature {
            inputs: (0..num_inputs).map(|_| F::error_type(tables)).collect(),
            output: F::error_type(tables),
        }
    }
}

/// The "generic declarations" list out the generic parameters for a
/// given item. Since items inherit generic items from one another
/// (e.g., from their parents),
#[derive(Clone, Debug, DebugWith, Default, PartialEq, Eq, Hash)]
pub struct GenericDeclarations {
    pub parent_item: Option<Entity>,
    pub declarations: IndexVec<BoundVar, GenericKind<GenericTyDeclaration>>,
}

impl GenericDeclarations {
    pub fn empty(parent_item: Option<Entity>) -> Arc<Self> {
        Arc::new(GenericDeclarations {
            parent_item,
            declarations: IndexVec::default(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty() && self.parent_item.is_none()
    }
}

/// Declaration of an individual generic type parameter.
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct GenericTyDeclaration {
    pub def_id: Entity,
    pub name: GlobalIdentifier,
}
