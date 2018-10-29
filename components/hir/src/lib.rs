//! The `Hir` is the "high-level IR". It is a simpified, somewhat resolved version of the bare AST.

#![feature(crate_visibility_modifier)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(decl_macro)]
#![feature(in_band_lifetimes)]
#![feature(macro_at_most_once_rep)]
#![feature(specialization)]

use ast::AstDatabase;
use indices::{IndexVec, U32Index};
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_entity::MemberKind;
use lark_error::ErrorReported;
use lark_error::WithError;
use parser::pos::{HasSpan, Span, Spanned};
use parser::StringId;
use std::sync::Arc;
use ty::declaration::Declaration;
use ty::interners::TyInternTables;

mod fn_body;
mod query_definitions;
mod scope;
mod type_conversion;

salsa::query_group! {
    pub trait HirDatabase: AstDatabase + AsRef<TyInternTables> {
        /// Get the fn-body for a given def-id.
        fn fn_body(key: Entity) -> Arc<FnBody> {
            type FnBodyQuery;
            use fn fn_body::fn_body;
        }

        /// Get the list of member names and their def-ids for a given struct.
        fn members(key: Entity) -> Result<Arc<Vec<Member>>, ErrorReported> {
            type MembersQuery;
            use fn query_definitions::members;
        }

        /// Gets the def-id for a field of a given class.
        fn member_entity(entity: Entity, kind: MemberKind, id: StringId) -> Option<Entity> {
            type MemberEntityQuery;
            use fn query_definitions::member_entity;
        }

        fn subentities(entity: Entity) -> Arc<Vec<Entity>> {
            type SubentitiesQuery;
            use fn query_definitions::subentities;
        }

        /// Get the type of something.
        fn ty(key: Entity) -> WithError<ty::Ty<Declaration>> {
            type TyQuery;
            use fn type_conversion::ty;
        }

        /// Get the signature of a function.
        fn signature(key: Entity) -> WithError<Result<ty::Signature<Declaration>, ErrorReported>> {
            type SignatureQuery;
            use fn type_conversion::signature;
        }

        /// Get the generic declarations from a particular item.
        fn generic_declarations(key: Entity) -> WithError<Result<Arc<ty::GenericDeclarations>, ErrorReported>> {
            type GenericDeclarationsQuery;
            use fn type_conversion::generic_declarations;
        }

        /// Resolve a type name that appears in the given entity.
        fn resolve_name(scope: Entity, name: StringId) -> Option<Entity> {
            type ResolveNameQuery;
            use fn scope::resolve_name;
        }
    }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct Member {
    pub name: StringId,
    pub kind: MemberKind,
    pub entity: Entity,
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct FnBody {
    /// List of arguments to the function. The type of each argument
    /// is given by the function signature (which can be separately queried).
    pub arguments: List<Variable>,

    /// Index of the root expression in the function body. Its result
    /// will be returned.
    pub root_expression: Expression,

    /// Contains all the data.
    pub tables: FnBodyTables,
}

/// All the data for a fn-body is stored in these tables.a
#[derive(Clone, Debug, DebugWith, Default, PartialEq, Eq, Hash)]
pub struct FnBodyTables {
    /// Map each expression index to its associated data.
    pub expressions: IndexVec<Expression, Spanned<ExpressionData>>,

    /// A `a: b` pair.
    pub identified_expressions: IndexVec<IdentifiedExpression, Spanned<IdentifiedExpressionData>>,

    /// Map each place index to its associated data.
    pub places: IndexVec<Place, Spanned<PlaceData>>,

    /// Map each perm index to its associated data.
    pub perms: IndexVec<Perm, Spanned<PermData>>,

    /// Map each variable index to its associated data.
    pub variables: IndexVec<Variable, Spanned<VariableData>>,

    /// Map each identifier index to its associated data.
    pub identifiers: IndexVec<Identifier, Spanned<IdentifierData>>,

    /// Errors we encountered constructing the hir
    pub errors: IndexVec<Error, Spanned<ErrorData>>,

    /// The data values for any `List<I>` values that appear elsewhere
    /// in the HIR; the way this works is that all of the list value
    /// are concatenated into one big vector, and each list just pulls
    /// out a slice of that. Note that this just contains `u32` values
    /// -- the actual `List<I>` remembers the index type `I` for its
    /// own values and does the casting back and forth.
    pub list_entries: Vec<u32>,
}

impl AsMut<FnBodyTables> for FnBodyTables {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

/// Trait implemented by the various kinds of indices that reach into
/// the HIR; allows us to grab the vector that they correspond to.
pub trait HirIndex: U32Index + Into<MetaIndex> {
    type Data: Clone;

    fn index_vec(hir: &FnBodyTables) -> &IndexVec<Self, Spanned<Self::Data>>;
    fn index_vec_mut(hir: &mut FnBodyTables) -> &mut IndexVec<Self, Spanned<Self::Data>>;
}

pub trait HirIndexData: Sized + Clone {
    type Index: HirIndex<Data = Self>;

    fn index_vec(hir: &FnBodyTables) -> &IndexVec<Self::Index, Spanned<Self>> {
        <<Self as HirIndexData>::Index as HirIndex>::index_vec(hir)
    }

    fn index_vec_mut(hir: &mut FnBodyTables) -> &mut IndexVec<Self::Index, Spanned<Self>> {
        <<Self as HirIndexData>::Index as HirIndex>::index_vec_mut(hir)
    }
}

impl AsRef<FnBodyTables> for FnBody {
    fn as_ref(&self) -> &FnBodyTables {
        &self.tables
    }
}

impl AsRef<FnBodyTables> for Arc<FnBody> {
    fn as_ref(&self) -> &FnBodyTables {
        &self.tables
    }
}

/// Permit indexing the HIR by any of the various index types.
/// Returns the underlying data from the index, skipping over the
/// span.
impl<I> std::ops::Index<I> for FnBody
where
    I: HirIndex,
{
    type Output = I::Data;

    fn index(&self, index: I) -> &I::Data {
        &self.tables[index]
    }
}

impl<I> std::ops::Index<I> for FnBodyTables
where
    I: HirIndex,
{
    type Output = I::Data;

    fn index(&self, index: I) -> &I::Data {
        &I::index_vec(self)[index]
    }
}

/// Trait for the various types for which a span can be had --
/// corresponds to all the index types plus `MetaIndex`.
pub trait SpanIndex {
    fn span_from(self, tables: &FnBodyTables) -> Span;
}

impl FnBody {
    /// Get the span for the given part of the HIR.
    pub fn span(&self, index: impl SpanIndex) -> Span {
        index.span_from(&self.tables)
    }
}

impl FnBodyTables {
    /// Get the span for the given part of the HIR.
    pub fn span(&self, index: impl SpanIndex) -> Span {
        index.span_from(self)
    }
}

impl<I: HirIndex> SpanIndex for I {
    fn span_from(self, tables: &FnBodyTables) -> Span {
        I::index_vec(tables)[self].span()
    }
}

/// Declares impls for each kind of HIR index as well as the
/// `hir::MetaIndex` enum.
macro_rules! define_meta_index {
    ($(($index_ty:ident, $data_ty:ty, $field:ident),)*) => {
        $(
            impl HirIndex for $index_ty {
                type Data = $data_ty;

                fn index_vec(hir: &FnBodyTables) -> &IndexVec<Self, Spanned<Self::Data>> {
                    &hir.$field
                }

                fn index_vec_mut(
                    hir: &mut FnBodyTables,
                ) -> &mut IndexVec<Self, Spanned<Self::Data>> {
                    &mut hir.$field
                }
            }

            impl HirIndexData for $data_ty {
                type Index = $index_ty;
            }

            debug::debug_fallback_impl!($index_ty);

            impl From<$index_ty> for MetaIndex {
                fn from(value: $index_ty) -> MetaIndex {
                    MetaIndex::$index_ty(value)
                }
            }
        )*

        /// The HIR has a number of *kinds* of indices that
        /// reach into it. This enum brings them together into
        /// a sort of "meta index". It's useful sometimes.
        #[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum MetaIndex {
            $(
                $index_ty($index_ty),
            )*
        }

        impl SpanIndex for MetaIndex {
            fn span_from(self, tables: &FnBodyTables) -> Span {
                match self {
                    $(
                        MetaIndex::$index_ty(index) => index.span_from(tables),
                    )*
                }
            }
        }
    };
}

define_meta_index! {
    (Expression, ExpressionData, expressions),
    (IdentifiedExpression, IdentifiedExpressionData, identified_expressions),
    (Place, PlaceData, places),
    (Perm, PermData, perms),
    (Variable, VariableData, variables),
    (Identifier, IdentifierData, identifiers),
    (Error, ErrorData, errors),
}

/// A list of "HIR indices" of type `I`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct List<I: HirIndex> {
    start_index: u32,
    len: u32,
    marker: std::marker::PhantomData<I>,
}

impl<I: HirIndex> Default for List<I> {
    fn default() -> Self {
        List {
            start_index: 0,
            len: 0,
            marker: std::marker::PhantomData,
        }
    }
}

impl<I: HirIndex> List<I> {
    /// Creates a list containing the values from in the
    /// `start_index..end_index` from the enclosing `FnBodyTables`.
    /// Ordinarily, you would not use this constructor, but rather
    /// `from_iterator`.
    fn from_start_and_end(start_index: usize, end_index: usize) -> Self {
        assert_eq!((start_index as u32) as usize, start_index);
        assert!(end_index >= start_index);

        if start_index == end_index {
            List::default()
        } else {
            List {
                start_index: start_index as u32,
                len: (end_index - start_index) as u32,
                marker: std::marker::PhantomData,
            }
        }
    }

    /// Creates a `List` containing the results of `from_iterator`.
    pub fn from_iterator(
        mut fn_body: impl AsMut<FnBodyTables>,
        iterator: impl IntoIterator<Item = I>,
    ) -> Self {
        let tables = fn_body.as_mut();
        let start_index = tables.list_entries.len();
        tables
            .list_entries
            .extend(iterator.into_iter().map(|i| i.as_u32()));
        let end_index = tables.list_entries.len();
        List::from_start_and_end(start_index, end_index)
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Iterate over the elements in the list.
    pub fn iter(&self, fn_body: &'f impl AsRef<FnBodyTables>) -> impl Iterator<Item = I> + 'f {
        let tables: &FnBodyTables = fn_body.as_ref();
        let start_index = self.start_index as usize;
        let end_index = start_index + self.len as usize;
        tables.list_entries[start_index..end_index]
            .iter()
            .cloned()
            .map(I::from_u32)
    }

    /// Iterate over the data for each the element in the list.
    pub fn iter_data(
        &self,
        fn_body: &'f impl AsRef<FnBodyTables>,
    ) -> impl Iterator<Item = I::Data> + 'f {
        self.iter_enumerated_data(fn_body).map(|(_, d)| d)
    }

    /// Iterate over the elements in the list *and* their associated
    /// data.
    pub fn iter_enumerated_data(
        &self,
        fn_body: &'f impl AsRef<FnBodyTables>,
    ) -> impl Iterator<Item = (I, I::Data)> + 'f {
        let tables: &FnBodyTables = fn_body.as_ref();
        let data_vec = I::index_vec(tables);
        self.iter(fn_body).map(move |i| {
            let data: &I::Data = &data_vec[i];
            (i, data.clone())
        })
    }
}

debug::debug_fallback_impl!(for[I: HirIndex] List<I>);

indices::index_type! {
    pub struct Expression { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum ExpressionData {
    /// `let <var> = <initializer> in <body>`
    Let {
        variable: Variable,
        initializer: Option<Expression>,
        body: Expression,
    },

    /// reference to a local variable `X`
    Place { perm: Perm, place: Place },

    /// `<place> = <value>`
    Assignment { place: Place, value: Expression },

    /// `<place>.method(<args>)`
    MethodCall {
        owner: Place,
        method: Identifier,
        arguments: List<Expression>,
    },

    /// E1; E2
    Sequence {
        first: Expression,
        second: Expression,
    },

    /// if E1 { E2 } else { E3 }
    If {
        condition: Expression,
        if_true: Expression,
        if_false: Expression,
    },

    /// Construct a value of some aggregate type, such as a struct or
    /// tuple:
    ///
    /// - `Struct { field1: expression1, ... fieldN: expressionN }`
    Aggregate {
        entity: Entity,
        fields: List<IdentifiedExpression>,
    },

    /// `()`
    Unit {},

    /// `Error` -- some error condition
    Error { error: Error },
}

indices::index_type! {
    pub struct IdentifiedExpression { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct IdentifiedExpressionData {
    pub identifier: Identifier,
    pub expression: Expression,
}

indices::index_type! {
    pub struct Perm { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum PermData {
    Share,
    Borrow,
    Own,
    Other(Entity),
    Default,
}

indices::index_type! {
    pub struct Place { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum PlaceData {
    Variable(Variable),
    Temporary(Expression),
    Field { owner: Place, name: Identifier },
}

indices::index_type! {
    pub struct Variable { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct VariableData {
    pub name: Identifier,
}

indices::index_type! {
    pub struct Identifier { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct IdentifierData {
    pub text: StringId,
}

indices::index_type! {
    pub struct Error { .. }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum ErrorData {
    Misc,
    UnknownIdentifier { text: StringId },
}
