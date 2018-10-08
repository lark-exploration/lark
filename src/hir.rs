//! The `Hir` is the "high-level IR". It is a simpified, somewhat resolved version of the bare AST.

use crate::indices::{IndexVec, U32Index};
use crate::parser::pos::{Span, Spanned};
use crate::parser::StringId;
use crate::ty;
use crate::ty::declaration::Declaration;
use ir::DefId;
use std::sync::Arc;

crate mod query_definitions;

salsa::query_group! {
    crate trait HirDatabase: salsa::Database {
        /// Get the def-id for the built-in boolean type.
        fn boolean_def_id(key: ()) -> DefId {
            type BooleanDefIdQuery;
            use fn query_definitions::boolean_def_id;
        }

        /// Get the fn-body for a given def-id.
        fn fn_body(key: DefId) -> Arc<FnBody> {
            type FnBodyQuery;
            use fn query_definitions::fn_body;
        }

        /// Get the list of member names and their def-ids for a given struct.
        fn members(key: DefId) -> Arc<Vec<Member>> {
            type MembersQuery;
            use fn query_definitions::members;
        }

        /// Gets the def-id for a field of a given class.
        fn member_def_id(m: (DefId, MemberKind, StringId)) -> Option<DefId> {
            type MemberDefIdQuery;
            use fn query_definitions::member_def_id;
        }

        /// Get the type of something.
        fn ty(key: DefId) -> ty::Ty<Declaration> {
            type TyQuery;
            use fn query_definitions::ty;
        }

        /// Get the signature of a method or function -- defined for fields and structs.
        fn signature(key: DefId) -> ty::Signature<Declaration> {
            type SignatureQuery;
            use fn query_definitions::signature;
        }

        /// Get the generic declarations from a particular item.
        fn generic_declarations(key: DefId) -> Arc<ty::GenericDeclarations> {
            type GenericDeclarations;
            use fn query_definitions::generic_declarations;
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum MemberKind {
    Field,
    Method,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct Member {
    crate name: StringId,
    crate kind: MemberKind,
    crate def_id: DefId,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct FnBody {
    /// List of arguments to the function. The type of each argument
    /// is given by the function signature (which can be separately queried).
    crate arguments: Vec<Variable>,

    /// Index of the root expression in the function body. Its result
    /// will be returned.
    crate root_expression: Expression,

    /// Map each expression index to its associated data.
    crate expressions: IndexVec<Expression, Spanned<ExpressionData>>,

    /// Map each place index to its associated data.
    crate places: IndexVec<Place, Spanned<PlaceData>>,

    /// Map each perm index to its associated data.
    crate perms: IndexVec<Perm, Spanned<PermData>>,

    /// Map each variable index to its associated data.
    crate variables: IndexVec<Variable, Spanned<VariableData>>,

    /// Map each identifier index to its associated data.
    crate identifiers: IndexVec<Identifier, Spanned<IdentifierData>>,
}

/// Trait implemented by the various kinds of indices that reach into
/// the HIR; allows us to grab the vector that they correspond to.
crate trait HirIndex: U32Index + Into<MetaIndex> {
    type Data;

    fn index_vec(hir: &FnBody) -> &IndexVec<Self, Spanned<Self::Data>>;
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
        &I::index_vec(self)[index].node
    }
}

/// Trait for the various types for which a span can be had --
/// corresponds to all the index types plus `MetaIndex`.
crate trait SpanIndex {
    fn span_from(self, fn_body: &FnBody) -> Span;
}

impl FnBody {
    /// Get the span for the given part of the HIR.
    crate fn span(&self, index: impl SpanIndex) -> Span {
        index.span_from(self)
    }
}

impl<I: HirIndex> SpanIndex for I {
    fn span_from(self, fn_body: &FnBody) -> Span {
        I::index_vec(fn_body)[self].span
    }
}

/// Declares impls for each kind of HIR index as well as the
/// `hir::MetaIndex` enum.
macro_rules! define_meta_index {
    ($(($index_ty:ident, $data_ty:ty, $field:ident),)*) => {
        $(
            impl HirIndex for $index_ty {
                type Data = $data_ty;

                fn index_vec(hir: &FnBody) -> &IndexVec<Self, Spanned<Self::Data>> {
                    &hir.$field
                }
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
        crate enum MetaIndex {
            $(
                $index_ty($index_ty),
            )*
        }

        impl SpanIndex for MetaIndex {
            fn span_from(self, fn_body: &FnBody) -> Span {
                match self {
                    $(
                        MetaIndex::$index_ty(index) => index.span_from(fn_body),
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
}

index_type! {
    crate struct Expression { .. }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate enum ExpressionData {
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
}

index_type! {
    crate struct Perm { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum PermData {
    Share,
    Borrow,
    Own,
    Other(DefId),
    Default,
}

index_type! {
    crate struct Place { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate enum PlaceData {
    Variable(Variable),
    Temporary(Expression),
    Field { owner: Place, name: Identifier },
}

index_type! {
    crate struct Variable { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct VariableData {
    crate name: Identifier,
}

index_type! {
    crate struct Identifier { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
crate struct IdentifierData {
    crate text: StringId,
}
