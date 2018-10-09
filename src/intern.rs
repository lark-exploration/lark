use indices::U32Index;
use map::FxIndexMap;
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
    Key: Copy + U32Index,
    Data: Clone + Hash + Eq,
{
    map: FxIndexMap<Data, ()>,
    key: std::marker::PhantomData<Key>,
}

impl<Key, Data> Default for InternTable<Key, Data>
where
    Key: Copy + U32Index,
    Data: Clone + Hash + Eq,
{
    fn default() -> Self {
        InternTable {
            map: FxIndexMap::default(),
            key: std::marker::PhantomData,
        }
    }
}

impl<Key, Data> InternTable<Key, Data>
where
    Key: Copy + U32Index,
    Data: Clone + Hash + Eq,
{
    crate fn get(&self, key: Key) -> Data {
        match self.map.get_index(key.as_usize()) {
            Some((key, &())) => key.clone(),
            None => panic!("invalid intern index: `{:?}`", key),
        }
    }

    crate fn intern(&mut self, data: Data) -> Key {
        let InternTable { map, key: _ } = self;
        let entry = map.entry(data);
        let index = entry.index();
        entry.or_insert(());
        Key::from_usize(index)
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
