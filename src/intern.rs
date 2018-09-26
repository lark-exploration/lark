//! Basic interner type. This type itself is meant to be composed
//! into a larger interner for many diffrent values (see e.g.
//! `ty::intern::TyInterners`).

use indexed_vec::{Idx, IndexVec};
use rustc_hash::FxHashMap;
use std::hash::Hash;
use std::rc::Rc;

#[derive(Debug)]
crate struct Interner<Key, Data>
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
    crate fn new() -> Self {
        Self {
            vec: IndexVec::default(),
            map: FxHashMap::default(),
        }
    }

    crate fn get(&self, key: Key) -> Data {
        self.vec[key].clone()
    }

    crate fn intern(&mut self, data: Data) -> Key {
        let Interner { vec, map } = self;
        map.entry(data.clone())
            .or_insert_with(|| vec.push(data))
            .clone()
    }
}

/// Trait used for data that can be interned into `Interners`,
/// giving back a `Self::Key` type.
///
/// Example: implemented for `crate::ty::PermData` with
/// key type `crate::ty::Perm`
crate trait Intern<Interners>: Clone {
    type Key;

    fn intern(self, interner: &Interners) -> Self::Key;
}

/// Reverse trait: implemented by the key (`crate::ty::Perm`)
/// and permits lookup in some `Interners` struct.
crate trait Untern<Interners>: Clone {
    type Data;

    fn untern(self, interner: &Interners) -> Self::Data;
}
