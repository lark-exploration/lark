use crate::intern::InternTable;
use crate::ty::base_inferred::{self, BaseInferred};
use crate::ty::base_only::{self, BaseOnly};
use crate::ty::declaration::{self, Declaration};
use crate::ty::BaseData;
use crate::ty::BoundVarOr;
use crate::ty::InferVarOr;
use std::sync::Arc;

macro_rules! intern_tables {
    (struct $InternTables:ident {
        struct $InternTablesData:ident {
            $(
                $field:ident : map($key:ty, $data:ty),
            )*
        }
    }) => {
        #[derive(Clone, Default)]
        crate struct $InternTables {
            data: Arc<$InternTablesData>,
        }

        impl $crate::intern::Has<$InternTables> for $InternTables {
            fn intern_tables(&self) -> &$InternTables {
                self
            }
        }

        #[derive(Default)]
        struct $InternTablesData {
            $(
                $field: parking_lot::RwLock<InternTable<$key, $data>>,
            )*
        }

        $(
            impl $crate::intern::Intern<$InternTables> for $data {
                type Key = $key;

                fn intern(self, tables: &$InternTables) -> $key {
                    tables.data.$field.write().intern(self)
                }
            }

            impl $crate::intern::Untern<$InternTables> for $key {
                type Data = $data;

                fn untern(self, tables: &$InternTables) -> $data {
                    tables.data.$field.read().get(self)
                }
            }
        )*
    }
}

intern_tables! {
    struct TyInternTables {
        struct TyInternTablesData {
            base_only_base: map(base_only::Base, InferVarOr<BaseData<BaseOnly>>),
            base_inferred_base: map(base_inferred::Base, BaseData<BaseInferred>),
            declaration_base: map(declaration::Base, BoundVarOr<BaseData<Declaration>>),
        }
    }
}
