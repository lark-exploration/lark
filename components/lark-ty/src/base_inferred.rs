//! A type family where we have fully inferred all the "base types" --
//! but all permissions are erased. This is the output of the
//! `base_type_check` query.

use crate::BaseData;
use crate::Erased;
use crate::Placeholder;
use crate::ReprKind;
use crate::TypeFamily;
use lark_debug_derive::DebugWith;
use lark_debug_with::{DebugWith, FmtWithSpecialized};
use lark_intern::{Intern, Untern};
use std::fmt;

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct BaseInferred;

impl TypeFamily for BaseInferred {
    type InternTables = BaseInferredTables;
    type Repr = Erased;
    type Perm = Erased;
    type Base = Base;
    type Placeholder = Placeholder;

    fn own_perm(_tables: &dyn AsRef<BaseInferredTables>) -> Erased {
        Erased
    }

    fn known_repr(_tables: &dyn AsRef<BaseInferredTables>, _repr_kind: ReprKind) -> Self::Repr {
        Erased
    }

    fn intern_base_data(
        tables: &dyn AsRef<BaseInferredTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        base_data.intern(tables)
    }
}

lark_collections::index_type! {
    pub struct Base { .. }
}

lark_debug_with::debug_fallback_impl!(Base);

impl<Cx> FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<BaseInferredTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

lark_intern::intern_tables! {
    pub struct BaseInferredTables {
        struct BaseInferredTablesData {
            base_inferred_base: map(Base, BaseData<BaseInferred>),
        }
    }
}
