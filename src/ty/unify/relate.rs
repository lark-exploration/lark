use crate::ty::intern::Untern;
use crate::ty::unify::UnificationTable;
use crate::ty::AsInferVar;
use crate::ty::Region;
use crate::ty::Ty;
use crate::ty::{Base, BaseData};
use crate::ty::{Generics, GenericsData};
use crate::ty::{Perm, PermData};
use std::convert::TryFrom;

enum Direction {
    LessThan,
    GreaterThan,
    Equal,
}

struct TyRelation<'me> {
    unify: &'me mut UnificationTable,
    direction: Direction,
    constriants: Vec<Constraint>,
}

enum Constraint {
    RegionConstraint {},
}

struct Error;

impl TyRelation<'me> {
    /// Checks that two types are "base equal", which
    /// means that their bases are deeply equal but which
    /// says nothing about their permissions.
    crate fn ty_base_eq(&mut self, ty1: Ty, ty2: Ty) {
        let Ty {
            perm: _,
            base: base1,
            generics: generics1,
        } = ty1;

        let Ty {
            perm: _,
            base: base2,
            generics: generics2,
        } = ty2;

        self.base_eq(base1, base2);
    }

    fn untern<K: Untern>(&self, key: K) -> K::Data {
        self.unify.intern.untern(key)
    }

    /// No matter what the "variance" is, for two types
    /// to be related, their bases must be equal. So
    /// for example `p1 String` and `p2 Vec` can never
    /// be related, but `p1 String` and `p2 String` can be.
    fn base_eq(&mut self, base1: Base, base2: Base) -> Result<(), Error> {
        match (self.untern(base1), self.untern(base2)) {
            (BaseData::Infer { var: var1 }, BaseData::Infer { var: var2 }) => match (
                self.unify.probe_data::<Base>(var1),
                self.unify.probe_data::<Base>(var2),
            ) {
                (Some(data1), Some(data2)) => self.base_data_eq(base1, data1, base2, data2),
                (Some(_), None) => Ok(self.unify.bind_unbound_var_to_bound_var(var2, var1)),
                (None, Some(_)) => Ok(self.unify.bind_unbound_var_to_bound_var(var1, var2)),
                (None, None) => Ok(self.unify.unify_unbound_vars(var1, var2)),
            },

            (data1, data2) => self.base_data_eq(base1, data1, base2, data2),
        }
    }

    fn base_data_eq(
        &mut self,
        _base1: Base,
        data1: BaseData,
        _base2: Base,
        data2: BaseData,
    ) -> Result<(), Error> {
        if data1 != data2 {
            Err(Error)
        } else {
            Ok(())
        }
    }
}
