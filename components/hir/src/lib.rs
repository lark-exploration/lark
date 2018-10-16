//! The `Hir` is the "high-level IR". It is a simpified, somewhat resolved version of the bare AST.

#![feature(crate_visibility_modifier)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(macro_at_most_once_rep)]

use ast::item_id::ItemId;
use ast::AstDatabase;
use indices::{IndexVec, U32Index};
use parser::pos::{Span, Spanned};
use parser::StringId;
use std::sync::Arc;
use ty::declaration::Declaration;

mod fn_body;
mod query_definitions;

salsa::query_group! {
    pub trait HirDatabase: AstDatabase {
        /// Get the def-id for the built-in boolean type.
        fn boolean_item_id(key: ()) -> ItemId {
            type BooleanItemIdQuery;
            use fn query_definitions::boolean_item_id;
        }

        /// Get the fn-body for a given def-id.
        fn fn_body(key: ItemId) -> Arc<FnBody> {
            type FnBodyQuery;
            use fn fn_body::fn_body;
        }

        /// Get the list of member names and their def-ids for a given struct.
        fn members(key: ItemId) -> Arc<Vec<Member>> {
            type MembersQuery;
            use fn query_definitions::members;
        }

        /// Gets the def-id for a field of a given class.
        fn member_item_id(m: (ItemId, MemberKind, StringId)) -> Option<ItemId> {
            type MemberItemIdQuery;
            use fn query_definitions::member_item_id;
        }

        /// Get the type of something.
        fn ty(key: ItemId) -> ty::Ty<Declaration> {
            type TyQuery;
            use fn query_definitions::ty;
        }

        /// Get the signature of a method or function -- defined for fields and structs.
        fn signature(key: ItemId) -> ty::Signature<Declaration> {
            type SignatureQuery;
            use fn query_definitions::signature;
        }

        /// Get the generic declarations from a particular item.
        fn generic_declarations(key: ItemId) -> Arc<ty::GenericDeclarations> {
            type GenericDeclarations;
            use fn query_definitions::generic_declarations;
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MemberKind {
    Field,
    Method,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Member {
    pub name: StringId,
    pub kind: MemberKind,
    pub def_id: ItemId,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FnBody {
    /// List of arguments to the function. The type of each argument
    /// is given by the function signature (which can be separately queried).
    pub arguments: Vec<Variable>,

    /// Index of the root expression in the function body. Its result
    /// will be returned.
    pub root_expression: Expression,

    /// Contains all the data.
    pub tables: FnBodyTables,
}

/// All the data for a fn-body is stored in these tables.a
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FnBodyTables {
    /// Map each expression index to its associated data.
    pub expressions: IndexVec<Expression, Spanned<ExpressionData>>,

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
}

/// Trait implemented by the various kinds of indices that reach into
/// the HIR; allows us to grab the vector that they correspond to.
pub trait HirIndex: U32Index + Into<MetaIndex> {
    type Data;

    fn index_vec(hir: &FnBodyTables) -> &IndexVec<Self, Spanned<Self::Data>>;
    fn index_vec_mut(hir: &mut FnBodyTables) -> &mut IndexVec<Self, Spanned<Self::Data>>;
}

pub trait HirIndexData: Sized {
    type Index: HirIndex<Data = Self>;

    fn index_vec(hir: &FnBodyTables) -> &IndexVec<Self::Index, Spanned<Self>> {
        <<Self as HirIndexData>::Index as HirIndex>::index_vec(hir)
    }

    fn index_vec_mut(hir: &mut FnBodyTables) -> &mut IndexVec<Self::Index, Spanned<Self>> {
        <<Self as HirIndexData>::Index as HirIndex>::index_vec_mut(hir)
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
        &I::index_vec(self)[index].node
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
        I::index_vec(tables)[self].span
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

            impl From<$index_ty> for MetaIndex {
                fn from(value: $index_ty) -> MetaIndex {
                    MetaIndex::$index_ty(value)
                }
            }
        )*

        /// The HIR has a number of *kinds* of indices that
        /// reach into it. This enum brings them together into
        /// a sort of "meta index". It's useful sometimes.
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    (Place, PlaceData, places),
    (Perm, PermData, perms),
    (Variable, VariableData, variables),
    (Identifier, IdentifierData, identifiers),
    (Error, ErrorData, errors),
}

indices::index_type! {
    pub struct Expression { .. }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExpressionData {
    /// `let <var> = <initializer> in <body>`
    Let {
        var: Variable,
        initializer: Expression,
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
        arguments: Arc<Vec<Expression>>,
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

    /// `()`
    Unit {},

    /// `Error` -- some error condition
    Error { error: Error },
}

indices::index_type! {
    pub struct Perm { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PermData {
    Share,
    Borrow,
    Own,
    Other(ItemId),
    Default,
}

indices::index_type! {
    pub struct Place { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlaceData {
    Variable(Variable),
    Temporary(Expression),
    Field { owner: Place, name: Identifier },
}

indices::index_type! {
    pub struct Variable { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VariableData {
    pub name: Identifier,
}

indices::index_type! {
    pub struct Identifier { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdentifierData {
    pub text: StringId,
}

indices::index_type! {
    pub struct Error { .. }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ErrorData {
    ParseError { description: String },
    Misc,
}
