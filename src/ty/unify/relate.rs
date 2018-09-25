use crate::ty::intern::{Interners, TyInterners};
use crate::ty::unify::UnificationTable;
use crate::ty::Predicate;
use crate::ty::Ty;
use crate::ty::Variance;

mod perms;
mod repr_eq;
mod spine;
mod test;

impl UnificationTable {
    crate fn ty_repr_eq(&mut self, ty1: Ty, ty2: Ty) -> Result<Vec<Predicate>, Error> {
        self.transactionally(|unify| {
            let mut relate = Relate {
                unify: unify,
                predicates: vec![],
            };
            // FIXME transaction
            relate.ty_repr_eq(ty1, ty2)?;
            Ok(relate.predicates)
        })
    }

    crate fn relate_tys(
        &mut self,
        direction: Variance,
        ty1: Ty,
        ty2: Ty,
    ) -> Result<Vec<Predicate>, Error> {
        self.transactionally(|unify| {
            let mut relate = Relate {
                unify,
                predicates: vec![],
            };
            // FIXME transaction
            relate.ty_repr_eq(ty1, ty2)?;
            let perm_own = relate.common().own;
            relate.relate_tys(direction, perm_own, ty1, perm_own, ty2)?;
            Ok(relate.predicates)
        })
    }

    fn transactionally<T, E>(
        &mut self,
        op: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        let saved_state = self.clone();
        match op(self) {
            Ok(r) => Ok(r),
            Err(e) => {
                *self = saved_state;
                Err(e)
            }
        }
    }
}

struct Relate<'me> {
    unify: &'me mut UnificationTable,
    predicates: Vec<Predicate>,
}

#[derive(Copy, Clone, Debug)]
crate struct Error;

impl Interners for Relate<'_> {
    fn interners(&self) -> &TyInterners {
        self.unify.interners()
    }
}
