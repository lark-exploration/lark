//! A type family where we have fully inferred all the types.
//! Permissions are partly erased (aliasing information lost). This is
//! the output of the `full_type_check` query.

use crate::BaseData;
use crate::Erased;
use crate::Placeholder;
use crate::ReprKind;
use crate::TypeFamily;
use debug::{DebugWith, FmtWithSpecialized};
use intern::{Intern, Untern};
use lark_debug_derive::DebugWith;
use std::fmt;

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct FullInferred;

impl TypeFamily for FullInferred {
    type InternTables = FullInferredTables;
    type Repr = Erased; // FIXME
    type Perm = Erased; // FIXME
    type Base = Base;
    type Placeholder = Placeholder;

    fn own_perm(_tables: &dyn AsRef<FullInferredTables>) -> Erased {
        Erased
    }

    fn known_repr(_tables: &dyn AsRef<FullInferredTables>, _repr_kind: ReprKind) -> Self::Repr {
        Erased
    }

    fn intern_base_data(
        tables: &dyn AsRef<FullInferredTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        base_data.intern(tables)
    }
}

indices::index_type! {
    pub struct Base { .. }
}

debug::debug_fallback_impl!(Base);

impl<Cx> FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<FullInferredTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

intern::intern_tables! {
    pub struct FullInferredTables {
        struct FullInferredTablesData {
            full_inferred_base: map(Base, BaseData<FullInferred>),
        }
    }
}
