use crate::ty::intern::{Interners, TyInterners};
use crate::ty::map::{Map, Mapper};
use crate::ty::unify::UnificationTable;
use crate::ty::Perm;
use crate::ty::Predicate;
use crate::ty::{Base, BaseData};

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
pub(super) struct SpineInstantiator<'me> {
    pub(super) unify: &'me mut UnificationTable,
    pub(super) predicates: &'me mut Vec<Predicate>,
}

impl Interners for SpineInstantiator<'me> {
    fn interners(&self) -> &TyInterners {
        self.unify.interners()
    }
}

impl Mapper for SpineInstantiator<'me> {
    fn map_perm(&mut self, _: Perm) -> Perm {
        self.unify.new_inferable::<Perm>()
    }

    fn map_base(&mut self, base: Base) -> Base {
        match self.unify.shallow_resolve_data(base) {
            Ok(data) => {
                let data1 = BaseData {
                    kind: data.kind,
                    generics: data.generics.map_with(self),
                };

                self.intern(data1)
            }

            Err(_) => {
                let new_variable = self.unify.new_inferable::<Base>();
                self.predicates.push(Predicate::BaseEq(base, new_variable));
                new_variable
            }
        }
    }
}
