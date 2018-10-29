//! A type family where we preserve what the user wrote in all cases.
//! We do not support inference and bases and things may map to bound
//! variables from generic declarations.

use crate::interners::TyInternTables;
use crate::BaseData;
use crate::BoundVarOr;
use crate::Erased;
use crate::TypeFamily;
use debug::{DebugWith, FmtWithSpecialized};
use intern::Intern;
use intern::Untern;
use lark_debug_derive::DebugWith;
use std::fmt;

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct Declaration;

impl TypeFamily for Declaration {
    type InternTables = crate::interners::TyInternTables;
    type Perm = Erased; // Not Yet Implemented
    type Base = Base;
    type Placeholder = !;

    fn own_perm(_tables: &dyn AsRef<TyInternTables>) -> Erased {
        Erased
    }

    fn intern_base_data(
        tables: &dyn AsRef<TyInternTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        BoundVarOr::Known(base_data).intern(tables)
    }
}

pub type DeclarationTy = crate::Ty<Declaration>;

indices::index_type! {
    pub struct Base { .. }
}

debug::debug_fallback_impl!(Base);

impl<Cx> FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<TyInternTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}
