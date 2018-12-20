//! Definition of a type family + type-checker methods for doing "base
//! only" inference. This is inference where we ignore permissions and
//! representations and focus only on the base types.

use lark_debug_derive::DebugWith;
use lark_intern::Intern;
use lark_ty::BaseData;
use lark_ty::Erased;
use lark_ty::InferVarOr;
use lark_ty::PermKind;
use lark_ty::Placeholder;
use lark_ty::ReprKind;
use lark_ty::TypeFamily;

crate mod apply_perm;

crate mod analysis;

/// Defines the `Base` type that represents base types.
crate mod base;
use base::Base;

crate mod constraint;

/// Defines the `Perm` type that represents permissions.
crate mod perm;
use perm::Perm;
use perm::PermData;

crate mod query_definition;

mod resolve_to_full_inferred;

/// Implements the `TypeCheckerFamilyDependentExt` methods along with substitution.
crate mod type_checker;

/// Type family for "base inference" -- inferring just the base types.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
crate struct FullInference;

impl TypeFamily for FullInference {
    type InternTables = FullInferenceTables;
    type Repr = Erased;
    type Perm = perm::Perm;
    type Base = Base;
    type Placeholder = Placeholder;

    fn own_perm(tables: &dyn AsRef<FullInferenceTables>) -> Self::Perm {
        PermData::Known(PermKind::Own).intern(tables)
    }

    fn known_repr(_tables: &dyn AsRef<FullInferenceTables>, _repr_kind: ReprKind) -> Self::Repr {
        Erased
    }

    fn intern_base_data(
        tables: &dyn AsRef<FullInferenceTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        InferVarOr::Known(base_data).intern(tables)
    }
}

lark_intern::intern_tables! {
    crate struct FullInferenceTables {
        struct FullInferenceTablesData {
            bases: map(Base, InferVarOr<BaseData<FullInference>>),
            perms: map(Perm, PermData),
        }
    }
}
