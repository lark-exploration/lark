//! A type family where we preserve what the user wrote in all cases.
//! We do not support inference and bases and things may map to bound
//! variables from generic declarations.

use crate::BaseData;
use crate::BoundVar;
use crate::BoundVarOr;
use crate::ReprKind;
use crate::TypeFamily;
use lark_debug_derive::DebugWith;
use lark_debug_with::{DebugWith, FmtWithSpecialized};
use lark_intern::{Intern, Untern};
use std::fmt;

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct Declaration;

impl TypeFamily for Declaration {
    type InternTables = DeclarationTables;
    type Repr = ReprKind;
    type Perm = Perm;
    type Base = Base;
    type Placeholder = !;

    fn own_perm(tables: &dyn AsRef<DeclarationTables>) -> Self::Perm {
        DeclaredPermKind::Own.intern(tables)
    }

    fn known_repr(_tables: &dyn AsRef<DeclarationTables>, repr_kind: ReprKind) -> ReprKind {
        repr_kind
    }

    fn intern_base_data(
        tables: &dyn AsRef<DeclarationTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        BoundVarOr::Known(base_data).intern(tables)
    }
}

impl Declaration {
    pub fn intern_bound_var(db: &AsRef<DeclarationTables>, bv: BoundVar) -> Base {
        let bv: BoundVarOr<BaseData<Declaration>> = BoundVarOr::BoundVar(bv);
        bv.intern(db)
    }
}

lark_indices::index_type! {
    pub struct Base { .. }
}

lark_debug_with::debug_fallback_impl!(Base);

impl<Cx> FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<DeclarationTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

lark_indices::index_type! {
    pub struct Perm { .. }
}

lark_debug_with::debug_fallback_impl!(Perm);

impl<Cx> FmtWithSpecialized<Cx> for Perm
where
    Cx: AsRef<DeclarationTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

/// For now, we only support `own T` in declarations.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum DeclaredPermKind {
    Own,
}

lark_intern::intern_tables! {
    pub struct DeclarationTables {
        struct DeclarationTablesData {
            bases: map(Base, BoundVarOr<BaseData<Declaration>>),
            perms: map(Perm, DeclaredPermKind),
        }
    }
}
