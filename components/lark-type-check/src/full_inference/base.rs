use crate::full_inference::FullInference;
use crate::full_inference::FullInferenceTables;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_ty::BaseData;
use lark_ty::InferVarOr;
use lark_unify::{InferVar, Inferable};

indices::index_type! {
    crate struct Base { .. }
}

impl Inferable<FullInferenceTables> for Base {
    type KnownData = BaseData<FullInference>;
    type Data = InferVarOr<BaseData<FullInference>>;

    /// Check if this is an inference variable and return the inference
    /// index if so.
    fn as_infer_var(self, interners: &FullInferenceTables) -> Option<InferVar> {
        match self.untern(interners) {
            InferVarOr::InferVar(var) => Some(var),
            InferVarOr::Known(_) => None,
        }
    }

    /// Create an inferable representing the inference variable `var`.
    fn from_infer_var(var: InferVar, interners: &FullInferenceTables) -> Self {
        let i: InferVarOr<BaseData<FullInference>> = InferVarOr::InferVar(var);
        i.intern(interners)
    }

    /// Asserts that this is not an inference variable and returns the
    /// "known data" that it represents.
    fn assert_known(self, interners: &FullInferenceTables) -> Self::KnownData {
        self.untern(interners).assert_known()
    }
}

debug::debug_fallback_impl!(Base);

impl<Cx> debug::FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<FullInferenceTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}
