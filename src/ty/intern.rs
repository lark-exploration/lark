use crate::ty::query::TyQueries;
use crate::ty::{Base, BaseData, Generic, Generics, GenericsData, Perm, PermData, Ty};
use indexed_vec::{Idx, IndexVec};
use rustc_hash::FxHashMap;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};

#[derive(Debug)]
struct Interner<Key, Data>
where
    Key: Copy + Idx,
    Data: Clone + Hash + Eq,
{
    vec: IndexVec<Key, Data>,
    map: FxHashMap<Data, Key>,
}

impl<Key, Data> Interner<Key, Data>
where
    Key: Copy + Idx,
    Data: Clone + Hash + Eq,
{
    fn new() -> Self {
        Self {
            vec: IndexVec::default(),
            map: FxHashMap::default(),
        }
    }

    fn add(&mut self, data: Data) -> Key {
        let Interner { vec, map } = self;
        map.entry(data.clone())
            .or_insert_with(|| vec.push(data))
            .clone()
    }
}

/// The "type context" is a global resource that interns types and
/// other type-related things. Types are allocates in the various
/// global arenas so that they can be freely copied around.
crate struct TyInterners {
    perms: RefCell<Interner<Perm, PermData>>,
    bases: RefCell<Interner<Base, BaseData>>,
    generics: RefCell<Interner<Generics, GenericsData>>,
}

impl TyInterners {
    crate fn new() -> Self {
        TyInterners {
            perms: RefCell::new(Interner::new()),
            bases: RefCell::new(Interner::new()),
            generics: RefCell::new(Interner::new()),
        }
    }

    crate fn get<K>(&self, key: K) -> K::Data
    where
        K: GetFrom,
    {
        key.get_from(self)
    }
}

crate trait GetFrom {
    type Data;

    fn get_from(self, interner: &TyInterners) -> Self::Data;
}

macro_rules! intern_ty {
    ($field:ident, $key:ty, $data:ty) => {
        impl TyInterners {
            crate fn perm(&self, data: $data) -> $key {
                self.$field.borrow_mut().add(data)
            }
        }

        impl GetFrom for $key {
            type Data = $data;

            fn get_from(self, interner: &TyInterners) -> $data {
                interner.$field.borrow().vec[self].clone()
            }
        }
    };
}

intern_ty!(bases, Base, BaseData);
intern_ty!(perms, Perm, PermData);
intern_ty!(generics, Generics, GenericsData);
