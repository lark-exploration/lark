use crate::ty::base_inferred::{self, BaseInferred};
use crate::ty::base_only::{self, BaseOnly};
use crate::ty::declaration::{self, Declaration};
use crate::ty::BaseData;
use crate::ty::BoundVarOr;
use crate::ty::InferVarOr;
use intern::InternTable;
use std::sync::Arc;

intern::intern_tables! {
    crate struct TyInternTables {
        struct TyInternTablesData {
            base_only_base: map(base_only::Base, InferVarOr<BaseData<BaseOnly>>),
            base_inferred_base: map(base_inferred::Base, BaseData<BaseInferred>),
            declaration_base: map(declaration::Base, BoundVarOr<BaseData<Declaration>>),
        }
    }
}
