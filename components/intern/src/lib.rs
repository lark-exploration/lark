use indices::U32Index;
use map::{Equivalent, FxIndexMap};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use std::hash::Hash;

pub trait Has<Tables> {
    fn intern_tables(&self) -> &Tables;
}

/// Generate a "intern tables" struct that can intern one or more
/// types. Input looks like:
///
/// ```ignore
/// struct MyTables {
///     struct MyTablesData {
///         field1: map(Key1, Value1),
///         field2: map(Key2, Value2),
///         ...
///         fieldN: map(KeyN, ValueN),
///     }
/// }
/// ```
///
/// This will generate the `MyTables` struct which will (internally)
/// hold a arc to a `MyTablesData` which has N interners. It will also
/// generate `Intern` and `Untern` impls for each Key/Value type.
#[macro_export]
macro_rules! intern_tables {
    (
        $v:vis struct $InternTables:ident {
            struct $InternTablesData:ident {
                $(
                    $field:ident : map($key:ty, $data:ty),
                )*
            }
        }
    ) => {
        #[derive(Clone, Default)]
        $v struct $InternTables {
            data: std::sync::Arc<$InternTablesData>,
        }

        impl $crate::Has<$InternTables> for $InternTables {
            fn intern_tables(&self) -> &$InternTables {
                self
            }
        }

        #[derive(Default)]
        struct $InternTablesData {
            $(
                $field: parking_lot::RwLock<$crate::InternTable<$key, $data>>,
            )*
        }

        $(
            impl $crate::InternDirect<$InternTables> for $data {
                fn table(
                    tables: &dyn $crate::Has<$InternTables>,
                ) -> &parking_lot::RwLock<$crate::InternTable<$key, $data>> {
                    let tables = $crate::Has::<$InternTables>::intern_tables(tables);
                    &tables.data.$field
                }
            }

            impl $crate::Intern<$InternTables> for $data {
                type Key = $key;

                fn intern(self, tables: &dyn $crate::Has<$InternTables>) -> $key {
                    $crate::intern_impl(self, tables, |v| v, |v| v)
                }
            }

            impl $crate::Untern<$InternTables> for $key {
                type Data = $data;

                fn untern(self, tables: &dyn $crate::Has<$InternTables>) -> $data {
                    let tables = $crate::Has::<$InternTables>::intern_tables(tables);
                    tables.data.$field.read().get(self)
                }
            }
        )*
    }
}

/// An "intern table" defines a single interner for
/// one key-value pair. They're meant to be grouped
/// into a larger `Interners` struct, a la
/// `crate::ty::TyInterners`, that define a series
/// of interners related to some particular area.
#[derive(Debug)]
pub struct InternTable<Key, Data>
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
    pub fn get(&self, key: Key) -> Data {
        match self.map.get_index(key.as_usize()) {
            Some((key, &())) => key.clone(),
            None => panic!("invalid intern index: `{:?}`", key),
        }
    }

    pub fn intern_check<D>(&self, data: &D) -> Option<Key>
    where
        D: ?Sized + Equivalent<Data> + Hash,
    {
        let InternTable { map, key: _ } = self;
        let (index, _, _) = map.get_full(data)?;
        Some(Key::from_usize(index))
    }

    pub fn intern(&mut self, data: Data) -> Key {
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
pub trait Intern<Interners> {
    type Key: U32Index;

    fn intern(self, interner: &dyn Has<Interners>) -> Self::Key;
}

/// Reverse trait: implemented by the key (`crate::ty::Perm`)
/// and permits lookup in some `Interners` struct.
pub trait Untern<Interners>: Clone {
    type Data;

    fn untern(self, interner: &dyn Has<Interners>) -> Self::Data;
}

/// Trait for something that is *directly* interned into an interning
/// table. For example, this might be implemented by `String`.
pub trait InternDirect<Interners>: Clone + Hash + Eq + Intern<Interners> {
    fn table(interner: &dyn Has<Interners>) -> &RwLock<InternTable<Self::Key, Self>>;
}

/// Helper for `Intern` implementations: interns `data` into `table`,
/// returning the intern key. Note that the data stored in table is of
/// type `TableData`, which may not be the same as `Data` -- the
/// `convert` closure will be used to convert `Data` into `TableData`
/// as needed.
pub fn intern_impl<Data, Interners, EquivData, TableData>(
    data: Data,
    interners: &dyn Has<Interners>,
    to_lookup_data: impl FnOnce(&Data) -> &EquivData,
    to_table_data: impl FnOnce(Data) -> TableData,
) -> TableData::Key
where
    EquivData: ?Sized + Equivalent<TableData> + Hash,
    TableData: InternDirect<Interners>,
{
    let table = TableData::table(interners);

    let table = table.upgradable_read();
    if let Some(key) = table.intern_check(to_lookup_data(&data)) {
        return key;
    }

    let mut table = RwLockUpgradableReadGuard::upgrade(table);
    table.intern(to_table_data(data))
}
