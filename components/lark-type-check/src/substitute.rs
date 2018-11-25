use intern::Untern;
use lark_ty::declaration::Declaration;
use lark_ty::declaration::DeclarationTables;
use lark_ty::map_family::FamilyMapper;
use lark_ty::map_family::Map;
use lark_ty::BoundVar;
use lark_ty::BoundVarOr;
use lark_ty::Erased;
use lark_ty::Generic;
use lark_ty::Ty;
use lark_ty::TypeFamily;

crate struct Substitution<'me, F, V>
where
    F: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<F>>,
{
    declaration_tables: &'me DeclarationTables,
    output_tables: &'me F::InternTables,
    values: &'me V,
}

impl<F, V> Substitution<'me, F, V>
where
    F: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<F>>,
{
    crate fn new(
        declaration_tables: &'me dyn AsRef<DeclarationTables>,
        output_tables: &'me dyn AsRef<F::InternTables>,
        values: &'me V,
    ) -> Self {
        Substitution {
            declaration_tables: declaration_tables.as_ref(),
            output_tables: output_tables.as_ref(),
            values,
        }
    }
}

impl<F, V> AsRef<DeclarationTables> for Substitution<'me, F, V>
where
    F: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<F>>,
{
    fn as_ref(&self) -> &DeclarationTables {
        &self.declaration_tables
    }
}

impl<F, V> FamilyMapper<Declaration, F> for Substitution<'me, F, V>
where
    F: TypeFamily<Repr = Erased, Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<F>>,
{
    fn map_ty(&mut self, ty: Ty<Declaration>) -> Ty<F> {
        let Ty {
            repr: _, // not yet used
            perm: _, // not yet used
            base,
        } = ty;

        match base.untern(self) {
            BoundVarOr::BoundVar(var) => self.values[var].assert_ty(),

            BoundVarOr::Known(base_data) => {
                let base_data1 = base_data.map(self);
                Ty {
                    repr: Erased,
                    perm: Erased,
                    base: F::intern_base_data(self.output_tables, base_data1),
                }
            }
        }
    }

    fn map_placeholder(&mut self, placeholder: !) -> F::Placeholder {
        placeholder
    }
}
