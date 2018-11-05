//! A type family where we just erase all permissions and we support inference.

use crate::BaseData;
use crate::Erased;
use crate::InferVarOr;
use crate::Placeholder;
use crate::ReprKind;
use crate::TypeFamily;
use debug::DebugWith;
use intern::{Intern, Untern};
use lark_debug_derive::DebugWith;
use lark_unify::{InferVar, Inferable};
use std::fmt;

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct BaseInference;

impl TypeFamily for BaseInference {
    type InternTables = BaseInferenceTables;
    type Repr = Erased;
    type Perm = Erased;
    type Base = Base;
    type Placeholder = Placeholder;

    fn own_perm(_tables: &dyn AsRef<BaseInferenceTables>) -> Erased {
        Erased
    }

    fn known_repr(_tables: &dyn AsRef<BaseInferenceTables>, _repr_kind: ReprKind) -> Self::Repr {
        Erased
    }

    fn intern_base_data(
        tables: &dyn AsRef<BaseInferenceTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        InferVarOr::Known(base_data).intern(tables)
    }
}

pub type BaseTy = crate::Ty<BaseInference>;

indices::index_type! {
    pub struct Base { .. }
}

impl Inferable<BaseInferenceTables> for Base {
    type KnownData = BaseData<BaseInference>;
    type Data = InferVarOr<BaseData<BaseInference>>;

    /// Check if this is an inference variable and return the inference
    /// index if so.
    fn as_infer_var(self, interners: &BaseInferenceTables) -> Option<InferVar> {
        match self.untern(interners) {
            InferVarOr::InferVar(var) => Some(var),
            InferVarOr::Known(_) => None,
        }
    }

    /// Create an inferable representing the inference variable `var`.
    fn from_infer_var(var: InferVar, interners: &BaseInferenceTables) -> Self {
        let i: InferVarOr<BaseData<BaseInference>> = InferVarOr::InferVar(var);
        i.intern(interners)
    }

    /// Asserts that this is not an inference variable and returns the
    /// "known data" that it represents.
    fn assert_known(self, interners: &BaseInferenceTables) -> Self::KnownData {
        self.untern(interners).assert_known()
    }
}

debug::debug_fallback_impl!(Base);

impl<Cx> debug::FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<BaseInferenceTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

intern::intern_tables! {
    pub struct BaseInferenceTables {
        struct BaseInferenceTablesData {
            base_inference_base: map(Base, InferVarOr<BaseData<BaseInference>>),
        }
    }
}
