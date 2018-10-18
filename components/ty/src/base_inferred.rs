//! A type family where we just erase all permissions and we support inference.

use crate::interners::TyInternTables;
use crate::BaseData;
use crate::Erased;
use crate::Placeholder;
use crate::TypeFamily;
use intern::Intern;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BaseInferred;

impl TypeFamily for BaseInferred {
    type Perm = Erased;
    type Base = Base;
    type Placeholder = Placeholder;

    fn own_perm(_tables: &dyn AsRef<TyInternTables>) -> Erased {
        Erased
    }

    fn intern_base_data(
        tables: &dyn AsRef<TyInternTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        base_data.intern(tables)
    }
}

pub type BaseTy = crate::Ty<BaseInferred>;

indices::index_type! {
    pub struct Base { .. }
}
