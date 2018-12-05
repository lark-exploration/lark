use crate::base_inference::{BaseInference, BaseInferenceTables};
use derive_new::new;
use lark_hir as hir;
use lark_intern::Intern;
use lark_ty::base_inferred::{BaseInferred, BaseInferredTables};
use lark_ty::map_family::FamilyMapper;
use lark_ty::map_family::Map;
use lark_ty::BaseData;
use lark_ty::Erased;
use lark_ty::Placeholder;
use lark_ty::Ty;
use lark_ty::TypeFamily;
use lark_unify::InferVar;
use lark_unify::UnificationTable;

#[derive(new)]
crate struct ResolveToBaseInferred<'me> {
    unify: &'me mut UnificationTable<BaseInferenceTables, hir::MetaIndex>,
    output_tables: &'me BaseInferredTables,
    unresolved: &'me mut Vec<InferVar>,
}

impl FamilyMapper<BaseInference, BaseInferred> for ResolveToBaseInferred<'_> {
    fn map_ty(&mut self, ty: Ty<BaseInference>) -> Ty<BaseInferred> {
        let Ty {
            repr: Erased,
            perm: Erased,
            base,
        } = ty;

        match self.unify.shallow_resolve_data(base) {
            Ok(BaseData { kind, generics }) => {
                let kind = kind.map(self);
                let generics = generics.map(self);
                let base = BaseData { kind, generics }.intern(self.output_tables);
                Ty {
                    repr: Erased,
                    perm: Erased,
                    base,
                }
            }

            Err(infer_var) => {
                self.unresolved.push(infer_var);
                BaseInferred::error_type(self.output_tables)
            }
        }
    }

    fn map_placeholder(&mut self, placeholder: Placeholder) -> Placeholder {
        placeholder
    }

    fn map_perm(&mut self, _perm: Erased) -> Erased {
        Erased
    }
}
