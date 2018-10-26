//! A type family where we preserve what the user wrote in all cases.
//! We do not support inference and bases and things may map to bound
//! variables from generic declarations.

use crate::interners::TyInternTables;
use crate::BaseData;
use crate::BoundVarOr;
use crate::Erased;
use crate::Ty;
use crate::TypeFamily;
use intern::Intern;
use lark_error::ErrorSentinel;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Declaration;

impl TypeFamily for Declaration {
    type Perm = Erased; // Not Yet Implemented
    type Base = Base;
    type Placeholder = !;

    fn own_perm(_tables: &dyn AsRef<TyInternTables>) -> Erased {
        Erased
    }

    fn intern_base_data(
        tables: &dyn AsRef<TyInternTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        BoundVarOr::Known(base_data).intern(tables)
    }
}

pub type DeclarationTy = crate::Ty<Declaration>;

indices::index_type! {
    pub struct Base { .. }
}

impl<DB> ErrorSentinel<&DB> for Ty<Declaration>
where
    DB: AsRef<TyInternTables>,
{
    fn error_sentinel(db: &DB) -> Self {
        Declaration::error_type(db)
    }
}
