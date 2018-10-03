use crate::map::FxIndexMap;
use indexed_vec::{Idx, IndexVec};
use std::hash::Hash;
use std::rc::Rc;

/// An "intern table" defines a single interner for
/// one key-value pair. They're meant to be grouped
/// into a larger `Interners` struct, a la
/// `crate::ty::TyInterners`, that define a series
/// of interners related to some particular area.
#[derive(Debug)]
crate struct InternTable<Key, Data>
where
    Key: Copy + Idx,
    Data: Clone + Hash + Eq,
{
    vec: IndexVec<Key, Data>,
    map: FxIndexMap<Data, Key>,
}

impl<Key, Data> Default for InternTable<Key, Data>
where
    Key: Copy + Idx,
    Data: Clone + Hash + Eq,
{
    fn default() -> Self {
        InternTable {
            vec: IndexVec::default(),
            map: FxIndexMap::default(),
        }
    }
}

impl<Key, Data> InternTable<Key, Data>
where
    Key: Copy + Idx,
    Data: Clone + Hash + Eq,
{
    crate fn get(&self, key: Key) -> Data {
        self.vec[key].clone()
    }

    crate fn intern(&mut self, data: Data) -> Key {
        let InternTable { vec, map } = self;
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
