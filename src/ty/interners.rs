use crate::intern::Intern;
use crate::intern::InternTable;
use crate::intern::Untern;
use crate::ty::base_inferred::{self, BaseInferred};
use crate::ty::base_only::{self, BaseOnly};
use crate::ty::declaration::{self, Declaration};
use crate::ty::BaseData;
use crate::ty::BoundVarOr;
use crate::ty::InferVarOr;
use std::sync::Arc;

#[derive(Clone, Default)]
crate struct TyInternTables {
    data: Arc<TyInternTablesData>,
}

crate trait Has<Tables> {
    fn intern_tables(&self) -> &Tables;

    fn intern<V>(&self, value: V) -> V::Key
    where
        Self: Sized,
        V: Intern<Tables>,
    {
        value.intern(self.intern_tables())
    }

    fn untern<K>(&self, key: K) -> K::Data
    where
        Self: Sized,
        K: Untern<Tables>,
    {
        key.untern(self.intern_tables())
    }
}

impl Has<TyInternTables> for TyInternTables {
    fn intern_tables(&self) -> &TyInternTables {
        self
    }
}

macro_rules! intern_tables_data {
    (struct $name:ident for $tables:ty {
        $(
            $field:ident : map($key:ty, $data:ty),
        )*
    }) => {
        #[derive(Default)]
        struct $name {
            $(
                $field: parking_lot::RwLock<InternTable<$key, $data>>,
            )*
        }

        $(
            impl $crate::intern::Intern<$tables> for $data {
                type Key = $key;

                fn intern(self, tables: &$tables) -> $key {
                    tables.data.$field.write().intern(self)
                }
            }

            impl $crate::intern::Untern<$tables> for $key {
                type Data = $data;

                fn untern(self, tables: &$tables) -> $data {
                    tables.data.$field.read().get(self)
                }
            }
        )*
    }
}

intern_tables_data! {
    struct TyInternTablesData for TyInternTables {
        base_only_base: map(base_only::Base, InferVarOr<BaseData<BaseOnly>>),
        base_inferred_base: map(base_inferred::Base, BaseData<BaseInferred>),
        declaration_base: map(declaration::Base, BoundVarOr<BaseData<Declaration>>),
    }
}
