use crate::full_inference::perm::Perm;
use crate::full_inference::perm::PermVar;
use crate::full_inference::FullInference;
use crate::full_inference::FullInferenceTables;
use crate::full_inference::PermData;
use derive_new::new;
use lark_collections::FxIndexMap;
use lark_hir as hir;
use lark_intern::Intern;
use lark_intern::Untern;
use lark_ty::full_inferred::FullInferred;
use lark_ty::full_inferred::FullInferredTables;
use lark_ty::map_family::FamilyMapper;
use lark_ty::map_family::Map;
use lark_ty::BaseData;
use lark_ty::Erased;
use lark_ty::PermKind;
use lark_ty::Placeholder;
use lark_ty::Ty;
use lark_ty::TypeFamily;
use lark_unify::InferVar;
use lark_unify::UnificationTable;

#[derive(new)]
crate struct ResolveToFullInferred<'me> {
    unify: &'me mut UnificationTable<FullInferenceTables, hir::MetaIndex>,
    input_tables: &'me FullInferenceTables,
    output_tables: &'me FullInferredTables,
    unresolved: &'me mut Vec<InferVar>,
    perm_kinds: &'me FxIndexMap<PermVar, PermKind>,
}

impl FamilyMapper<FullInference, FullInferred> for ResolveToFullInferred<'_> {
    fn map_ty(&mut self, ty: Ty<FullInference>) -> Ty<FullInferred> {
        let Ty {
            repr: Erased,
            perm,
            base,
        } = ty;

        let perm = self.map_perm(perm);

        match self.unify.shallow_resolve_data(base) {
            Ok(BaseData { kind, generics }) => {
                let kind = kind.map(self);
                let generics = generics.map(self);
                let base = BaseData { kind, generics }.intern(self.output_tables);
                Ty {
                    repr: Erased,
                    perm,
                    base,
                }
            }

            Err(infer_var) => {
                self.unresolved.push(infer_var);
                FullInferred::error_type(self.output_tables)
            }
        }
    }

    fn map_placeholder(&mut self, placeholder: Placeholder) -> Placeholder {
        placeholder
    }

    fn map_perm(&mut self, perm: Perm) -> PermKind {
        match perm.untern(self.input_tables) {
            PermData::Known(k) => k,
            PermData::Placeholder(_k) => unimplemented!("placeholder perm"),
            PermData::Inferred(v) => self.perm_kinds.get(&v).cloned().unwrap_or(PermKind::Share),
        }
    }
}
