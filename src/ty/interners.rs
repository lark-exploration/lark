use crate::intern::InternTable;
use crate::ty::base_only;
use crate::ty::declaration;
use crate::ty::BaseData;
use crate::ty::InferVarOr;
use std::sync::Arc;

#[derive(Clone, Default)]
crate struct TyInternTables {
    data: Arc<TyInternTablesData>,
}

crate trait HasTyInternTables {
    fn ty_intern_tables(&self) -> &TyInternTables;
}

impl HasTyInternTables for TyInternTables {
    fn ty_intern_tables(&self) -> &TyInternTables {
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
        base_ty: map(base_only::Base, InferVarOr<BaseData<base_only::BaseOnly>>),
        declaration_ty: map(declaration::Base, BaseData<declaration::Declaration>),
    }
}
