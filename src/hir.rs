//! The `Hir` is the "high-level IR". It is a simpified, somewhat resolved version of the bare AST.

use crate::ir::DefId;
use crate::parser::pos::{Span, Spanned};
use crate::parser::StringId;
use indexed_vec::{Idx, IndexVec};
use std::rc::Rc;

crate mod typed;

crate struct Hir {
    crate expressions: IndexVec<Expression, ExpressionData>,
    crate places: IndexVec<Place, PlaceData>,
    crate perms: IndexVec<Perm, PermData>,
    crate variables: IndexVec<Variable, VariableData>,
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
}

/// Declares impls for each kind of HIR index; this permits
/// you to do `hir[foo]` as well as `MetaIndex::from(foo)`.
macro_rules! hir_index_impls {
    ($index_ty:ident, $data_ty:ty, $field:ident) => {
        impl std::ops::Index<$index_ty> for Hir {
            type Output = $data_ty;

            fn index(&self, index: $index_ty) -> &$data_ty {
                &self.$field[index]
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

index_type! {
    crate struct Function { .. }
}

crate struct FunctionData {
    crate arguments: Rc<Vec<Variable>>,
    crate body: Expression,
}

index_type! {
    crate struct Expression { .. }
}

crate struct ExpressionData {
    crate span: Span,
    crate kind: ExpressionKind,
}

crate enum ExpressionKind {
    /// `let <var> = <initializer> in <body>`
    Let {
        var: Variable,
        initializer: Expression,
        body: Expression,
    },

    /// reference to a local variable `X`
    Place { perm: Option<Perm>, place: Place },

    /// `<place> = <value>`
    Assignment { place: Place, value: Expression },

    /// `<place>.method(<args>)`
    MethodCall {
        owner: Place,
        method: Spanned<StringId>,
        arguments: Rc<Vec<Expression>>,
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

crate enum PermData {
    Share,
    Borrow,
    Own,
    Other(DefId),
}

index_type! {
    crate struct Place { .. }
}

crate enum PlaceData {
    Variable(Variable),
    Temporary(Expression),
    Field { owner: Place, name: DefId },
}

index_type! {
    crate struct Variable { .. }
}

crate struct VariableData {
    crate name: Spanned<StringId>,
}

index_type! {
    crate struct ExpressionVec { .. }
}

crate struct ExpressionVecData {
    crate expressions: Rc<Vec<Expression>>,
}
