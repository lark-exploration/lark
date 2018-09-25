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
        let mut relate = Relate {
            unify: self,
            predicates: vec![],
        };
        relate.ty_repr_eq(ty1, ty2)?;
        Ok(relate.predicates)
    }

    crate fn relate_tys(
        &mut self,
        direction: Variance,
        ty1: Ty,
        ty2: Ty,
    ) -> Result<Vec<Predicate>, Error> {
        let mut relate = Relate {
            unify: self,
            predicates: vec![],
        };
        relate.ty_repr_eq(ty1, ty2)?;
        let perm_own = relate.common().own;
        relate.relate_tys(direction, perm_own, ty1, perm_own, ty2)?;
        Ok(relate.predicates)
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
