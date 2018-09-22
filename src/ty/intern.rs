use crate::ty::query::TyQueries;
use crate::ty::Generic;
use crate::ty::InferVar;
use crate::ty::Ty;
use crate::ty::{Base, BaseData, BaseKind};
use crate::ty::{Generics, GenericsData};
use crate::ty::{Perm, PermData};
use indexed_vec::{Idx, IndexVec};
use rustc_hash::FxHashMap;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

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

    fn intern(&mut self, data: Data) -> Key {
        let Interner { vec, map } = self;
        map.entry(data.clone())
            .or_insert_with(|| vec.push(data))
            .clone()
    }
}

/// The "type context" is a global resource that interns types and
/// other type-related things. Types are allocates in the various
/// global arenas so that they can be freely copied around.
#[derive(Clone)]
crate struct TyInterners {
    data: Rc<TyInternersData>,
}

struct TyInternersData {
    perms: RefCell<Interner<Perm, PermData>>,
    bases: RefCell<Interner<Base, BaseData>>,
    generics: RefCell<Interner<Generics, GenericsData>>,
    common: Common,
}

crate struct Common {
    crate empty_generics: Generics,
    crate own: Perm,
}

impl TyInterners {
    crate fn new() -> Self {
        let mut perms = Interner::new();
        let bases = Interner::new();
        let mut generics = Interner::new();

        let common = Common {
            own: perms.intern(PermData::Own),
            empty_generics: generics.intern(GenericsData {
                elements: Rc::new(vec![]),
            }),
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

    crate fn common(&self) -> &Common {
        &self.data.common
    }

    crate fn intern<D>(&self, data: D) -> D::Key
    where
        D: Intern,
    {
        data.intern(self)
    }

    crate fn intern_generics(&self, iter: impl Iterator<Item = Generic>) -> Generics {
        let generics_data = GenericsData {
            elements: Rc::new(iter.collect()),
        };
        self.intern(generics_data)
    }

    crate fn intern_base_var(&self, var: InferVar) -> Base {
        let data = BaseData {
            kind: BaseKind::Infer { var },
            generics: self.data.common.empty_generics,
        };
        self.intern(data)
    }

    crate fn untern<K>(&self, key: K) -> K::Data
    where
        K: Untern,
    {
        key.untern(self)
    }
}

crate trait Intern {
    type Key;

    fn intern(self, interner: &TyInterners) -> Self::Key;
}

crate trait Untern {
    type Data;

    fn untern(self, interner: &TyInterners) -> Self::Data;
}

macro_rules! intern_ty {
    ($field:ident, $key:ty, $data:ty) => {
        impl Intern for $data {
            type Key = $key;

            fn intern(self, interner: &TyInterners) -> $key {
                interner.data.$field.borrow_mut().intern(self)
            }
        }

        impl Untern for $key {
            type Data = $data;

            fn untern(self, interner: &TyInterners) -> $data {
                interner.data.$field.borrow().vec[self].clone()
            }
        }
    };
}

intern_ty!(bases, Base, BaseData);
intern_ty!(perms, Perm, PermData);
intern_ty!(generics, Generics, GenericsData);
