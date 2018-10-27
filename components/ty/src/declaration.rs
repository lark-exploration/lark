//! A type family where we preserve what the user wrote in all cases.
//! We do not support inference and bases and things may map to bound
//! variables from generic declarations.

use crate::interners::TyInternTables;
use crate::BaseData;
use crate::BoundVarOr;
use crate::Erased;
use crate::Ty;
use crate::TypeFamily;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_error::ErrorSentinel;
use parser::pos::Span;
use std::fmt;

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

debug::debug_fallback_impl!(Base);

impl<Cx> DebugWith<Cx> for Base
where
    Cx: AsRef<TyInternTables>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

impl<DB> ErrorSentinel<&DB> for Ty<Declaration>
where
    DB: AsRef<TyInternTables>,
{
    fn error_sentinel(db: &DB, _spans: &[Span]) -> Self {
        Declaration::error_type(db)
    }
}
