use crate::ty::intern::{Interners, TyInterners};
use crate::ty::InferVar;
use crate::ty::Inferable;
use crate::ty::{Base, BaseData};
use crate::ty::{Perm, PermData};
use crate::unify;

impl unify::Inferable<TyInterners> for Perm {
    type KnownData = PermData;
    type Data = Inferable<PermData>;

    fn as_infer_var(self, interners: &TyInterners) -> Option<InferVar> {
        if let Inferable::Infer(v) = interners.untern(self) {
            Some(v)
        } else {
            None
        }
    }

    /// Create an inferable representing the inference variable `var`.
    fn from_infer_var(var: InferVar, interners: &TyInterners) -> Self {
        interners.intern::<Self::Data>(Inferable::Infer(var))
    }

    /// Asserts that this is not an inference variable and returns the
    /// "known data" that it represents.
    fn assert_known(self, interners: &TyInterners) -> Self::KnownData {
        interners.untern(self).assert_known()
    }
}

impl unify::Inferable<TyInterners> for Base {
    type KnownData = BaseData;
    type Data = Inferable<BaseData>;

    fn as_infer_var(self, interners: &TyInterners) -> Option<InferVar> {
        if let Inferable::Infer(v) = interners.untern(self) {
            Some(v)
        } else {
            None
        }
    }

    /// Create an inferable representing the inference variable `var`.
    fn from_infer_var(var: InferVar, interners: &TyInterners) -> Self {
        interners.intern::<Self::Data>(Inferable::Infer(var))
    }

    /// Asserts that this is not an inference variable and returns the
    /// "known data" that it represents.
    fn assert_known(self, interners: &TyInterners) -> Self::KnownData {
        interners.untern(self).assert_known()
    }
}
