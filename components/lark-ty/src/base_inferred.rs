//! A type family where we just erase all permissions and we support inference.

use crate::BaseData;
use crate::Erased;
use crate::Placeholder;
use crate::TypeFamily;
use debug::{DebugWith, FmtWithSpecialized};
use intern::{Intern, Untern};
use lark_debug_derive::DebugWith;
use std::fmt;

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct BaseInferred;

impl TypeFamily for BaseInferred {
    type InternTables = BaseInferredTables;
    type Perm = Erased;
    type Base = Base;
    type Placeholder = Placeholder;

    fn own_perm(_tables: &dyn AsRef<BaseInferredTables>) -> Erased {
        Erased
    }

    fn intern_base_data(
        tables: &dyn AsRef<BaseInferredTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        base_data.intern(tables)
    }
}

pub type BaseTy = crate::Ty<BaseInferred>;

indices::index_type! {
    pub struct Base { .. }
}

debug::debug_fallback_impl!(Base);

impl<Cx> FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<BaseInferredTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

intern::intern_tables! {
    pub struct BaseInferredTables {
        struct BaseInferredTablesData {
            base_inferred_base: map(Base, BaseData<BaseInferred>),
        }
    }
}
