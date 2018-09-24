//! This module contains functions that assert "base equality",
//! which means that two types have equal "base types" information but
//! which does not relate their permissions.

use crate::ty::intern::Interners;
use crate::ty::unify::relate::Error;
use crate::ty::unify::relate::Relate;
use crate::ty::Base;
use crate::ty::Generic;
use crate::ty::Permits;
use crate::ty::Predicate;
use crate::ty::Region;
use crate::ty::Ty;
use crate::ty::Variance;
use crate::ty::{Perm, PermData};
use log::debug;

impl Relate<'me> {
    /// Checks that two types are "perm-equal", which
    /// means that they have the same set of permitted operations.
    ///
    /// Note that types can be "perm-equal" which are not
    /// "repr-equal": for example, `shared Vec<borrow String>`
    /// and `shared Vec<shared String>` permit the same
    /// operations, but they have a different representation.
    crate fn relate_tys(&mut self, direction: Variance, ty1: Ty, ty2: Ty) -> Result<(), Error> {
        debug!("ty_perm_eq(ty1={:?}, ty2={:?})", ty1, ty2);

        let Ty {
            perm: perm1,
            base: base1,
        } = ty1;

        let Ty {
            perm: perm2,
            base: base2,
        } = ty2;

        match self.relate_perms(direction.permits(), perm1, perm2)? {
            None => {
                // We do not know enough about the permissions of ty1 and ty2
                // to figure out how the permissions of their generics relate
                // to one another. This happens when either ty1 or ty2 is an
                // as-yet-uninferred type variable, for exampe. In that case,
                // we'll file a predicate to try again later.
                self.predicates.push(Predicate::RelateTypes {
                    direction,
                    ty1,
                    ty2,
                });
            }

            Some(min_perm) => {
                // `owner_perm` is the minimum of `perm1` and `perm2`. It will be

                self.relate_bases(direction, min_perm, ty1, base1, ty2, base2)?;
            }
        }

        Ok(())
    }

    /// Check that the two permissions are "repr-equal" -- basically this means
    /// that if one of them is a borrow, the other must be. If either of them
    /// is not yet inferred, we file a predicate for later processing.
    fn relate_perms(
        &mut self,
        direction: Permits,
        perm1: Perm,
        perm2: Perm,
    ) -> Result<Option<Perm>, Error> {
        match (
            self.unify.shallow_resolve_data(perm1),
            self.unify.shallow_resolve_data(perm2),
        ) {
            (Ok(data1), Ok(data2)) => match (data1, data2) {
                (PermData::Shared(region1), PermData::Shared(region2))
                | (PermData::Borrow(region1), PermData::Borrow(region2)) => Ok(Some(
                    self.relate_regions(direction, perm1, region1, perm2, region2),
                )),

                (PermData::Own, PermData::Own) => Ok(Some(perm1)),

                (PermData::Placeholder(_), _) | (_, PermData::Placeholder(_)) => unimplemented!(),

                (PermData::Shared(_), _) | (PermData::Borrow(_), _) | (PermData::Own, _) => {
                    Err(Error)
                }
            },

            // If either of the permissions is not known, then file an obligation for later.
            (Err(_), _) | (_, Err(_)) => {
                self.predicates.push(Predicate::RelatePerms {
                    direction,
                    perm1,
                    perm2,
                });
                Ok(None)
            }
        }
    }

    fn relate_regions(
        &mut self,
        direction: Permits,
        perm1: Perm,
        region1: Region,
        perm2: Perm,
        region2: Region,
    ) -> Perm {
        // Enforce that the regions have an appropriate
        // relationship to one another.
        if region1 != region2 {
            self.predicates.push(Predicate::RelateRegions {
                direction,
                region1,
                region2,
            });
        }

        // We want to return the more restrictive permission.
        // We just added a constraint that they have a certain
        // relationship (above), so we can deduce which one
        // that must be based on the `direction`.
        match direction {
            // If `perm1 permits perm2`, then `perm2` must be more restrictive
            // (or they are equal, of course).
            Permits::Permits => perm2,

            // The opposite.
            Permits::PermittedBy => perm1,

            // They are equal.
            Permits::Equals => perm1,
        }
    }

    fn min_region(&mut self, region1: Region, region2: Region) -> Region {
        if region1 == region2 {
            region1
        } else {
            let region3 = self.unify.next_region();
            self.predicates.push(Predicate::RelateRegions {
                direction: Permits::PermittedBy,
                region1: region3,
                region2: region1,
            });
            self.predicates.push(Predicate::RelateRegions {
                direction: Permits::PermittedBy,
                region1: region3,
                region2: region2,
            });
            region3
        }
    }

    /// Check that the two base types are repr-eq. If both of the base
    /// types are unknown, this will file a predicate for later processing.
    fn relate_bases(
        &mut self,
        direction: Variance,
        min_perm: Perm,
        ty1: Ty,
        base1: Base,
        ty2: Ty,
        base2: Base,
    ) -> Result<(), Error> {
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
                    self.relate_generics(direction, min_perm, generic1, generic2)?;
                }

                Ok(())
            }

            (Err(_), _) | (_, Err(_)) => {
                // XXX can we move "repr eq" and "base eq" into this as an optimization?

                // Meanwhile, push a predicate for later to revisit the permissions.
                self.predicates.push(Predicate::RelateTypes {
                    direction,
                    ty1,
                    ty2,
                });

                Ok(())
            }
        }
    }

    /// Check that the two generics are repr-eq.
    fn relate_generics(
        &mut self,
        _direction: Variance,
        _min_perm: Perm,
        generic1: Generic,
        generic2: Generic,
    ) -> Result<(), Error> {
        match (generic1, generic2) {
            (Generic::Ty(_ty1), Generic::Ty(_ty2)) => unimplemented!(),
        }
    }
}
