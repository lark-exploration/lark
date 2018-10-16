//! The `Hir` is the "high-level IR". It is a simpified, somewhat resolved version of the bare AST.

use crate::ir::DefId;
use crate::parser::pos::{Span, Spanned};
use crate::parser::StringId;
use indexed_vec::{Idx, IndexVec};
use std::sync::Arc;

crate mod typeck;
crate mod typed;

crate struct Hir {
    crate expressions: IndexVec<Expression, Spanned<ExpressionData>>,
    crate places: IndexVec<Place, Spanned<PlaceData>>,
    crate perms: IndexVec<Perm, Spanned<PermData>>,
    crate variables: IndexVec<Variable, Spanned<VariableData>>,
    crate identifiers: IndexVec<Identifier, Spanned<IdentifierData>>,
}

/// The HIR has a number of *kinds* of indices that
/// reach into it. This enum brings them together into
/// a sort of "meta index". It's useful sometimes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
crate enum MetaIndex {
    Expression(Expression),
    Place(Place),
    Perm(Perm),
    Variable(Variable),
    Identifier(Identifier),
}

crate trait HirIndex: Idx {
    type Data;

    fn index_vec(hir: &Hir) -> &IndexVec<Self, Spanned<Self::Data>>;
}

impl<I> std::ops::Index<I> for Hir
where
    I: HirIndex,
{
    type Output = I::Data;

    fn index(&self, index: I) -> &I::Data {
        &I::index_vec(self)[index].0
    }
}

impl Hir {
    /// Get the span for the given part of the HIR.
    crate fn span<I>(&self, index: I) -> Span
    where
        I: HirIndex,
    {
        I::index_vec(self)[index].1
    }
}

/// Declares impls for each kind of HIR index; this permits
/// you to do `hir[foo]` as well as `MetaIndex::from(foo)`.
macro_rules! hir_index_impls {
    ($index_ty:ident, $data_ty:ty, $field:ident) => {
        impl HirIndex for $index_ty {
            type Data = $data_ty;

            fn index_vec(hir: &Hir) -> &IndexVec<Self, Spanned<Self::Data>> {
                &hir.$field
            }
        }

        impl From<$index_ty> for MetaIndex {
            fn from(value: $index_ty) -> MetaIndex {
                MetaIndex::$index_ty(value)
            }
        }
    };
}

hir_index_impls!(Expression, ExpressionData, expressions);
hir_index_impls!(Place, PlaceData, places);
hir_index_impls!(Perm, PermData, perms);
hir_index_impls!(Variable, VariableData, variables);
hir_index_impls!(Identifier, IdentifierData, identifiers);

index_type! {
    crate struct Expression { .. }
}

#[derive(Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
crate enum PlaceData {
    Variable(Variable),
    Temporary(Expression),
    Field { owner: Place, name: Identifier },
}

index_type! {
    crate struct Variable { .. }
}

#[derive(Copy, Clone, Debug)]
crate struct VariableData {
    crate name: Identifier,
}

index_type! {
    crate struct Identifier { .. }
}

#[derive(Copy, Clone, Debug)]
crate struct IdentifierData {
    text: StringId,
}
