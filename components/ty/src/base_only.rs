//! A type family where we just erase all permissions and we support inference.

use crate::BaseData;
use crate::Erased;
use crate::InferVarOr;
use crate::Placeholder;
use crate::TypeFamily;
use debug::DebugWith;
use intern::{Intern, Untern};
use lark_debug_derive::DebugWith;
use std::fmt;
use unify::{InferVar, Inferable};

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct BaseOnly;

impl TypeFamily for BaseOnly {
    type InternTables = BaseOnlyTables;
    type Perm = Erased;
    type Base = Base;
    type Placeholder = Placeholder;

    fn own_perm(_tables: &dyn AsRef<BaseOnlyTables>) -> Erased {
        Erased
    }

    fn intern_base_data(
        tables: &dyn AsRef<BaseOnlyTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        InferVarOr::Known(base_data).intern(tables)
    }
}

pub type BaseTy = crate::Ty<BaseOnly>;

indices::index_type! {
    pub struct Base { .. }
}

impl Inferable<BaseOnlyTables> for Base {
    type KnownData = BaseData<BaseOnly>;
    type Data = InferVarOr<BaseData<BaseOnly>>;

    /// Check if this is an inference variable and return the inference
    /// index if so.
    fn as_infer_var(self, interners: &BaseOnlyTables) -> Option<InferVar> {
        match self.untern(interners) {
            InferVarOr::InferVar(var) => Some(var),
            InferVarOr::Known(_) => None,
        }
    }

    /// Create an inferable representing the inference variable `var`.
    fn from_infer_var(var: InferVar, interners: &BaseOnlyTables) -> Self {
        let i: InferVarOr<BaseData<BaseOnly>> = InferVarOr::InferVar(var);
        i.intern(interners)
    }

    /// Asserts that this is not an inference variable and returns the
    /// "known data" that it represents.
    fn assert_known(self, interners: &BaseOnlyTables) -> Self::KnownData {
        self.untern(interners).assert_known()
    }
}

debug::debug_fallback_impl!(Base);

impl<Cx> debug::FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<BaseOnlyTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

intern::intern_tables! {
    pub struct BaseOnlyTables {
        struct BaseOnlyTablesData {
            base_only_base: map(Base, InferVarOr<BaseData<BaseOnly>>),
        }
    }
}
