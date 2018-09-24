use crate::ty::intern::{Interners, TyInterners};
use crate::ty::unify::UnificationTable;
use crate::ty::Predicate;
use crate::ty::Ty;

mod perms;
mod repr_eq;
mod spine;
mod test;

impl UnificationTable {
    crate fn ty_repr_eq(&mut self, ty1: Ty, ty2: Ty) -> Result<Vec<Predicate>, Error> {
        let mut relate = Relate {
            unify: self,
            predicates: vec![],
        };
        // FIXME transaction
        relate.ty_repr_eq(ty1, ty2)?;
        Ok(relate.predicates)
    }
}

struct Relate<'me> {
    unify: &'me mut UnificationTable,
    predicates: Vec<Predicate>,
}

///
crate enum Variance {
    Covariant,
    Contravariant,
    Invariant,
}

impl Variance {
    /// Returns a "permits" relation R1 such that `P1 B1 (R) P2 B1` if `P1 (R1) P2`.
    crate fn permits(self) -> Permits {
        match self {
            Variance::Covariant => {
                // p1 T1 <: p2 T2 if p1: p2
                Permits::Permits
            }

            Variance::Contravariant => {
                // p1 T1 <: p2 T2 if p2: p1
                Permits::PermittedBy
            }

            Variance::Invariant => {
                // p1 T1 == p2 T2 if p2 == p1
                Permits::Equals
            }
        }
    }
}

crate enum Permits {
    /// `P1 permits P2` if, everything you can do with P2, you can also do with P1.
    ///  Alternatively, in terms of the permission lattice, if `P1 >= P2`.
    Permits,

    /// Inverse of `permits`.
    PermittedBy,

    /// Both permits and permitted by.
    Equals,
}

#[derive(Copy, Clone, Debug)]
crate struct Error;

impl Interners for Relate<'_> {
    fn interners(&self) -> &TyInterners {
        self.unify.interners()
    }
}
