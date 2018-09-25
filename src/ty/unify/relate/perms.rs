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
use crate::ty::RegionDirection;
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
    crate fn relate_tys(
        &mut self,
        direction: Variance,
        owner_perm1: Perm,
        ty1: Ty,
        owner_perm2: Perm,
        ty2: Ty,
    ) -> Result<(), Error> {
        debug!("ty_perm_eq(ty1={:?}, ty2={:?})", ty1, ty2);

        let Ty {
            perm: perm1,
            base: base1,
        } = ty1;

        let Ty {
            perm: perm2,
            base: base2,
        } = ty2;

        let perm_min1 = self.intersect_perms(owner_perm1, perm1);
        let perm_min2 = self.intersect_perms(owner_perm2, perm2);

        self.relate_perms(direction.permits(), perm_min1, perm_min2)?;

        self.relate_bases(direction, perm_min1, ty1, base1, perm_min2, ty2, base2)?;

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
    ) -> Result<Perm, Error> {
        match (
            self.unify.shallow_resolve_data(perm1),
            self.unify.shallow_resolve_data(perm2),
            direction,
        ) {
            (Ok(data1), Ok(data2), _) => match (data1, data2) {
                (PermData::Shared(region1), PermData::Shared(region2))
                | (PermData::Borrow(region1), PermData::Borrow(region2)) => {
                    self.relate_regions(direction.region_direction(), region1, region2);
                }

                (PermData::Own, PermData::Own) => (),

                (PermData::Placeholder(_), _) | (_, PermData::Placeholder(_)) => unimplemented!(),

                (PermData::Shared(_), _) | (PermData::Borrow(_), _) | (PermData::Own, _) => {
                    return Err(Error);
                }
            },

            // If we are equating permissions, and we have an
            // unresolved inference variable, we can go ahead and
            // unify.
            (Err(var1), Err(var2), Permits::Equals) => self.unify.unify_unbound_vars(var1, var2),
            (Err(var1), Ok(_), Permits::Equals) => {
                self.unify.bind_unbound_var_to_value(var1, perm2)
            }
            (Ok(_), Err(var2), Permits::Equals) => {
                self.unify.bind_unbound_var_to_value(var2, perm1)
            }

            // If either of the permissions is not known, then file an obligation for later.
            (Err(_), _, Permits::Permits)
            | (_, Err(_), Permits::Permits)
            | (Err(_), _, Permits::PermittedBy)
            | (_, Err(_), Permits::PermittedBy) => {
                self.predicates.push(Predicate::RelatePerms {
                    direction,
                    perm1,
                    perm2,
                });
            }
        }

        Ok(self.pick_min_perm_from_direction(direction, perm1, perm2))
    }

    fn relate_regions(&mut self, direction: RegionDirection, region1: Region, region2: Region) {
        // Enforce that the regions have an appropriate
        // relationship to one another.
        if region1 != region2 {
            self.predicates.push(Predicate::RelateRegions {
                direction,
                region1,
                region2,
            });
        }
    }

    fn pick_min_perm_from_direction(
        &mut self,
        direction: Permits,
        perm1: Perm,
        perm2: Perm,
    ) -> Perm {
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

    fn intersect_perms(&mut self, perm1: Perm, perm2: Perm) -> Perm {
        match (
            self.unify.shallow_resolve_data(perm1),
            self.unify.shallow_resolve_data(perm2),
        ) {
            (Ok(PermData::Own), _) => perm2,
            (_, Ok(PermData::Own)) => perm1,

            (Ok(PermData::Shared(r1)), Ok(PermData::Shared(r2)))
            | (Ok(PermData::Borrow(r1)), Ok(PermData::Shared(r2)))
            | (Ok(PermData::Shared(r1)), Ok(PermData::Borrow(r2))) => {
                let r3 = self.union_region(r1, r2);
                self.intern(PermData::Shared(r3))
            }

            // All things can be shared within the function.
            (Ok(PermData::Shared(_)), Ok(PermData::Placeholder(_))) => perm1,
            (Ok(PermData::Placeholder(_)), Ok(PermData::Shared(_))) => perm2,

            (Ok(PermData::Borrow(r1)), Ok(PermData::Borrow(r2))) => {
                let r3 = self.union_region(r1, r2);
                self.intern(PermData::Borrow(r3))
            }

            // We can permit borrows within the function, but only if
            // we know that the placeholder permits borrows at all.
            (Ok(PermData::Borrow(_)), Ok(PermData::Placeholder(_)))
            | (Ok(PermData::Placeholder(_)), Ok(PermData::Borrow(_))) => unimplemented!(),

            // Two placeholders can be compared but only if `P1 permits P2`
            // or `P2 permits P1` is declared on the function. Otherwise,
            // we can't know their intersection.
            (Ok(PermData::Placeholder(_)), Ok(PermData::Placeholder(_))) => unimplemented!(),

            (Err(_), _) | (_, Err(_)) => {
                let perm3: Perm = self.unify.new_inferable();
                self.predicates.push(Predicate::IntersectPerms {
                    perm1,
                    perm2,
                    perm3,
                });
                perm3
            }
        }
    }

    fn union_region(&mut self, region1: Region, region2: Region) -> Region {
        if region1 == region2 {
            region1
        } else {
            let region3 = self.unify.next_region();
            self.predicates.push(Predicate::UnionRegions {
                region1,
                region2,
                region3,
            });
            region3
        }
    }

    /// Check that the two base types are repr-eq. If both of the base
    /// types are unknown, this will file a predicate for later processing.
    fn relate_bases(
        &mut self,
        direction: Variance,
        perm_min1: Perm,
        ty1: Ty,
        base1: Base,
        perm_min2: Perm,
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

                let mut error = Ok(());
                for (generic1, generic2) in generics_data1.iter().zip(&generics_data2) {
                    match self.relate_generics(direction, perm_min1, generic1, perm_min2, generic2)
                    {
                        Ok(()) => (),
                        Err(Error) => error = Err(Error),
                    }
                }

                error
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
        direction: Variance,
        perm_min1: Perm,
        generic1: Generic,
        perm_min2: Perm,
        generic2: Generic,
    ) -> Result<(), Error> {
        match (generic1, generic2) {
            (Generic::Ty(ty1), Generic::Ty(ty2)) => {
                self.relate_tys(direction, perm_min1, ty1, perm_min2, ty2)
            }
        }
    }
}
