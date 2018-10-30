use crate::base_inferred::{self, BaseInferred};
use crate::BaseData;

intern::intern_tables! {
    pub struct TyInternTables {
        struct TyInternTablesData {
            base_inferred_base: map(base_inferred::Base, BaseData<BaseInferred>),
        }
    }
}
