//! This module contains functions that assert "base equality",
//! which means that two types have equal "base types" information but
//! which does not relate their permissions.

use crate::ty::intern::Interners;
use crate::ty::map::Map;
use crate::ty::unify::relate::spine::SpineInstantiator;
use crate::ty::unify::relate::Error;
use crate::ty::unify::relate::Relate;
use crate::ty::Base;
use crate::ty::Generic;
use crate::ty::InferVar;
use crate::ty::Predicate;
use crate::ty::Ty;
use crate::ty::{Perm, PermData};
use log::debug;

impl Relate<'me> {
    /// Checks that two types are "base equal", which
    /// means that their bases are deeply equal but which
    /// says nothing about their permissions.
    crate fn ty_repr_eq(&mut self, ty1: Ty, ty2: Ty) -> Result<(), Error> {
        debug!("ty_repr_eq(ty1={:?}, ty2={:?})", ty1, ty2);

        let Ty {
            perm: perm1,
            base: base1,
        } = ty1;

        let Ty {
            perm: perm2,
            base: base2,
        } = ty2;

        // NB: We intentionally keep going even if `perm_repr_eq` gets
        // an error. This is useful for inference.
        let r_perm = self.perm_repr_eq(perm1, perm2);
        debug!("ty_repr_eq: r_perm = {:?}", r_perm);
        let r_base = self.base_repr_eq(base1, base2);
        debug!("ty_repr_eq: r_base = {:?}", r_base);

        r_perm?;
        r_base?;
        Ok(())
    }

    fn perm_repr_eq(&mut self, perm1: Perm, perm2: Perm) -> Result<(), Error> {
        match (
            self.unify.shallow_resolve_data(perm1),
            self.unify.shallow_resolve_data(perm2),
        ) {
            (Ok(data1), Ok(data2)) => match (data1, data2) {
                // Shared + own have the same representation.
                (PermData::Shared { .. }, PermData::Shared { .. })
                | (PermData::Shared { .. }, PermData::Own)
                | (PermData::Own, PermData::Own)
                | (PermData::Own, PermData::Shared { .. }) => Ok(()),

                // Borrows are represented by a pointer
                // and hence are only compatible with themselves.
                (PermData::Borrow { .. }, PermData::Borrow { .. }) => Ok(()),
                (PermData::Borrow { .. }, _) | (_, PermData::Borrow { .. }) => {
                    return Err(Error);
                }

                // Placeholders might be represented by a pointer
                // and hence are only compatible with themselves.
                (
                    PermData::Placeholder { placeholder: p1 },
                    PermData::Placeholder { placeholder: p2 },
                ) => {
                    if p1 == p2 {
                        Ok(())
                    } else {
                        return Err(Error);
                    }
                }
                (PermData::Placeholder { .. }, _) | (_, PermData::Placeholder { .. }) => {
                    return Err(Error);
                }
            },

            // If either of the permissions is not known, then file an obligation for later.
            (Err(_), _) | (_, Err(_)) => {
                self.predicates.push(Predicate::PermReprEq(perm1, perm2));
                Ok(())
            }
        }
    }

    /// No matter what the "variance" is, for two types
    /// to be related, their bases must be equal. So
    /// for example `p1 String` and `p2 Vec` can never
    /// be related, but `p1 String` and `p2 String` can be.
    fn base_repr_eq(&mut self, base1: Base, base2: Base) -> Result<(), Error> {
        match (
            self.unify.shallow_resolve_data(base1),
            self.unify.shallow_resolve_data(base2),
        ) {
            (Ok(data1), Ok(data2)) => {
                if data1.kind != data2.kind {
                    debug!("base_data_eq: error: kind mismatch");
                    return Err(Error);
                }

                let generics_data1 = self.untern(data1.generics);
                let generics_data2 = self.untern(data2.generics);
                assert_eq!(generics_data1.len(), generics_data2.len());

                for (generic1, generic2) in generics_data1.iter().zip(&generics_data2) {
                    self.generic_repr_eq(generic1, generic2)?;
                }

                Ok(())
            }

            (Ok(_), Err(var2)) => {
                let base2 = self.instantiate_spine(var2, base1);
                self.base_repr_eq(base1, base2)
            }

            (Err(var1), Ok(_)) => {
                let base1 = self.instantiate_spine(var1, base2);
                self.base_repr_eq(base1, base2)
            }

            (Err(_), Err(_)) => {
                self.predicates.push(Predicate::BaseReprEq(base1, base2));
                Ok(())
            }
        }
    }

    fn generic_repr_eq(&mut self, generic1: Generic, generic2: Generic) -> Result<(), Error> {
        match (generic1, generic2) {
            (Generic::Ty(ty1), Generic::Ty(ty2)) => self.ty_repr_eq(ty1, ty2),
        }
    }

    fn instantiate_spine(&mut self, var: InferVar, base: Base) -> Base {
        assert!(self.unify.probe(var).is_none());
        let new_spine = base.map_with(&mut SpineInstantiator {
            unify: &mut self.unify,
            predicates: &mut self.predicates,
        });
        self.unify.bind_unbound_var_to_value(var, new_spine);
        new_spine
    }
}
