//! A type family where we preserve what the user wrote in all cases.
//! We do not support inference and bases and things may map to bound
//! variables from generic declarations.

use crate::BaseData;
use crate::BoundVar;
use crate::BoundVarOr;
use crate::Erased;
use crate::ReprKind;
use crate::TypeFamily;
use debug::{DebugWith, FmtWithSpecialized};
use intern::Intern;
use intern::Untern;
use lark_debug_derive::DebugWith;
use std::fmt;

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct Declaration;

impl TypeFamily for Declaration {
    type InternTables = DeclarationTables;
    type Repr = ReprKind;
    type Perm = Erased; // Not Yet Implemented
    type Base = Base;
    type Placeholder = !;

    fn own_perm(_tables: &dyn AsRef<DeclarationTables>) -> Erased {
        Erased
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

pub type DeclarationTy = crate::Ty<Declaration>;

indices::index_type! {
    pub struct Base { .. }
}

debug::debug_fallback_impl!(Base);

impl<Cx> FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<DeclarationTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

intern::intern_tables! {
    pub struct DeclarationTables {
        struct DeclarationTablesData {
            declaration_base: map(Base, BoundVarOr<BaseData<Declaration>>),
        }
    }
}
