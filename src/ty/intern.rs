use crate::intern::{Intern, InternTable, Untern};
use crate::ty::debug::TyDebugContext;
use crate::ty::Generics;
use crate::ty::{Base, BaseData};
use crate::ty::{InferVar, Inferable};
use crate::ty::{Perm, PermData};
use parking_lot::RwLock;
use std::iter::FromIterator;
use std::sync::Arc;

/// The "type context" is a global resource that interns types and
/// other type-related things. Types are allocates in the various
/// global arenas so that they can be freely copied around.
#[derive(Clone)]
crate struct TyInterners {
    data: Arc<TyInternersData>,
}

struct TyInternersData {
    perms: RwLock<InternTable<Perm, Inferable<PermData>>>,
    bases: RwLock<InternTable<Base, Inferable<BaseData>>>,
    common: Common,
}

crate struct Common {
    crate empty_generics: Generics,
    crate own: Perm,
}

crate trait Interners {
    fn interners(&self) -> &TyInterners;

    fn intern<D>(&self, data: D) -> D::Key
    where
        D: Intern<TyInterners>,
        Self: Sized,
    {
        data.intern(self.interners())
    }

    fn untern<K>(&self, key: K) -> K::Data
    where
        K: Untern<TyInterners>,
        Self: Sized,
    {
        key.untern(self.interners())
    }

    fn common(&self) -> &Common
    where
        Self: Sized,
    {
        &self.interners().data.common
    }

    fn intern_infer_var<T, V>(&self, var: InferVar) -> T
    where
        T: Untern<TyInterners, Data = Inferable<V>>,
        Inferable<V>: Intern<TyInterners, Key = T>,
        Self: Sized,
    {
        self.intern(Inferable::Infer(var))
    }
}

impl<T: ?Sized> Interners for &T
where
    T: Interners,
{
    fn interners(&self) -> &TyInterners {
        T::interners(self)
    }
}

impl TyInterners {
    crate fn new() -> Self {
        let mut perms = InternTable::default();
        let bases = InternTable::default();

        let own = perms.intern(Inferable::Known(PermData::Own));

        let empty_generics = Generics::from_iter(None);

        let common = Common {
            own,
            empty_generics,
        };

        TyInterners {
            data: Arc::new(TyInternersData {
                perms: RwLock::new(perms),
                bases: RwLock::new(bases),
                common,
            }),
        }
    }
}

impl Interners for TyInterners {
    fn interners(&self) -> &TyInterners {
        self
    }
}

macro_rules! intern_ty {
    ($field:ident, $key:ty, $data:ty) => {
        impl Intern<TyInterners> for $data {
            type Key = $key;

            fn intern(self, interner: &TyInterners) -> $key {
                interner.data.$field.write().intern(self)
            }
        }

        impl Untern<TyInterners> for $key {
            type Data = $data;

            fn untern(self, interner: &TyInterners) -> $data {
                interner.data.$field.read().get(self)
            }
        }
    };
}

macro_rules! intern_inferable_ty {
    ($field:ident, $key:ty, $data:ty) => {
        // Add the canonical impls between `$key` and `Interable<$data>`.
        intern_ty!($field, $key, Inferable<$data>);

        // Add a convenience impl that lets you intern directly
        // from `$data` without writing `Inferable::Known`.`
        impl Intern<TyInterners> for $data {
            type Key = $key;

            fn intern(self, interner: &TyInterners) -> $key {
                let value = Inferable::Known(self);
                value.intern(interner)
            }
        }
    };
}

intern_inferable_ty!(bases, Base, BaseData);
intern_inferable_ty!(perms, Perm, PermData);

impl TyDebugContext for TyInterners {}
