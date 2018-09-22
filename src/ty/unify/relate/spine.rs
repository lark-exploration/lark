use crate::ty::intern::Untern;
use crate::ty::unify::relate::Relate;
use crate::ty::unify::UnificationTable;
use crate::ty::AsInferVar;
use crate::ty::Generic;
use crate::ty::InferVar;
use crate::ty::Predicate;
use crate::ty::Region;
use crate::ty::Ty;
use crate::ty::{Base, BaseData};
use crate::ty::{Generics, GenericsData};
use crate::ty::{Perm, PermData};
use std::convert::TryFrom;

/// Instantiating the "spine" of a value means to create a value
/// that is identical except that, for every permission, there is
/// a fresh variable. This is useful in inference because
/// for two types to be relatable (subtypes, equal, etc) they must
/// share a common spine.
///
/// If, during instantiation, we encounter unbound inference variables,
/// we will create new inference variables for them and push a `BaseEq`
/// constraint.
///
/// Examples, where `?Pn` represents a permission variable and `?Bn`
/// represents a base inference variable:
///
/// - Given a `share Vec<own String>`, you would get back a
///   `?P0 Vec<?P1 String>`.
/// - Given a `share Vec<?P0 ?B0>`, you would get back a
///   `?P1 Vec<?P2 ?B1>` and the constraint `BaseEq(?B0, ?B1)`.
pub(super) trait InstantiateSpine {
    fn instantiate_spine(self, relate: &mut Relate<'_>) -> Self;
}

impl InstantiateSpine for Base {
    fn instantiate_spine(self, relate: &mut Relate<'_>) -> Self {
        match relate.unify.shallow_resolve_data(self) {
            Ok(data) => {
                let data1 = data.instantiate_spine(relate);
                relate.intern(data1)
            }

            Err(_) => {
                let new_variable = relate.unify.new_inferable::<Base>();
                relate
                    .predicates
                    .push(Predicate::BaseEq(self, new_variable));
                new_variable
            }
        }
    }
}

impl InstantiateSpine for BaseData {
    fn instantiate_spine(self, relate: &mut Relate<'_>) -> BaseData {
        assert!(self.as_infer_var().is_none());
        BaseData {
            kind: self.kind,
            generics: self.generics.instantiate_spine(relate),
        }
    }
}

impl InstantiateSpine for Ty {
    fn instantiate_spine(self, relate: &mut Relate<'_>) -> Self {
        let Ty { perm: _, base } = self;
        let perm = relate.unify.new_inferable::<Perm>();
        let base = base.instantiate_spine(relate);
        Ty { perm, base }
    }
}

impl InstantiateSpine for Generics {
    fn instantiate_spine(self, relate: &mut Relate<'_>) -> Self {
        let data = relate.untern(self);
        let intern = relate.unify.intern.clone();
        intern.intern_generics(data.iter().map(|generic| generic.instantiate_spine(relate)))
    }
}

impl InstantiateSpine for Generic {
    fn instantiate_spine(self, relate: &mut Relate<'_>) -> Self {
        match self {
            Generic::Ty(ty) => Generic::Ty(ty.instantiate_spine(relate)),
        }
    }
}
