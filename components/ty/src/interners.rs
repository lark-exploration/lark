use crate::base_inferred::{self, BaseInferred};
use crate::base_only::{self, BaseOnly};
use crate::declaration::{self, Declaration};
use crate::BaseData;
use crate::BoundVarOr;
use crate::InferVarOr;

intern::intern_tables! {
    pub struct TyInternTables {
        struct TyInternTablesData {
            base_only_base: map(base_only::Base, InferVarOr<BaseData<BaseOnly>>),
            base_inferred_base: map(base_inferred::Base, BaseData<BaseInferred>),
            declaration_base: map(declaration::Base, BoundVarOr<BaseData<Declaration>>),
        }
    }
}
