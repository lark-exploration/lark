use crate::intern::{Intern, InternTable, Untern};
use crate::ty::debug::TyDebugContext;
use crate::ty::Generic;
use crate::ty::{Base, BaseData};
use crate::ty::{Generics, GenericsData};
use crate::ty::{InferVar, Inferable};
use crate::ty::{Perm, PermData};
use std::cell::RefCell;
use std::rc::Rc;

/// The "type context" is a global resource that interns types and
/// other type-related things. Types are allocates in the various
/// global arenas so that they can be freely copied around.
#[derive(Clone)]
crate struct TyInterners {
    data: Rc<TyInternersData>,
}

struct TyInternersData {
    perms: RefCell<InternTable<Perm, Inferable<PermData>>>,
    bases: RefCell<InternTable<Base, Inferable<BaseData>>>,
    generics: RefCell<InternTable<Generics, GenericsData>>,
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

    fn intern_generics(&self, iter: impl Iterator<Item = Generic>) -> Generics
    where
        Self: Sized,
    {
        let generics_data = GenericsData {
            elements: Rc::new(iter.collect()),
        };
        self.intern(generics_data)
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
        let mut perms = InternTable::new();
        let bases = InternTable::new();
        let mut generics = InternTable::new();

        let own = perms.intern(Inferable::Known(PermData::Own));

        let empty_generics = generics.intern(GenericsData {
            elements: Rc::new(vec![]),
        });

        let common = Common {
            own,
            empty_generics,
        };

        TyInterners {
            data: Rc::new(TyInternersData {
                perms: RefCell::new(perms),
                bases: RefCell::new(bases),
                generics: RefCell::new(generics),
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
                interner.data.$field.borrow_mut().intern(self)
            }
        }

        impl Untern<TyInterners> for $key {
            type Data = $data;

            fn untern(self, interner: &TyInterners) -> $data {
                interner.data.$field.borrow().get(self)
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
intern_ty!(generics, Generics, GenericsData);

impl TyDebugContext for TyInterners {}
