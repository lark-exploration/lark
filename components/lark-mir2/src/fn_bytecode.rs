use crate as mir;
use crate::MirDatabase;
use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData, LangItem};
use lark_error::Diagnostic;
use lark_error::WithError;
use lark_hir as hir;
use lark_string::global::GlobalIdentifier;
use map::FxIndexMap;
use std::sync::Arc;

crate fn fn_bytecode(
    db: &impl MirDatabase,
    item_entity: Entity,
) -> WithError<Arc<crate::FnBytecode>> {
    let mut errors = vec![];
    let fn_bytecode = MirLower::new(db, item_entity, &mut errors).lower_typecheck_of_item();
    WithError {
        value: Arc::new(fn_bytecode),
        errors,
    }
}

struct MirLower<'me, DB: MirDatabase> {
    db: &'me DB,
    item_entity: Entity,
    fn_bytecode_tables: mir::FnBytecodeTables,
    variables: FxIndexMap<GlobalIdentifier, mir::Variable>,
    errors: &'me mut Vec<Diagnostic>,
}

impl<'me, DB> MirLower<'me, DB>
where
    DB: MirDatabase,
{
    fn new(db: &'me DB, item_entity: Entity, errors: &'me mut Vec<Diagnostic>) -> Self {
        MirLower {
            db,
            errors,
            item_entity,
            fn_bytecode_tables: Default::default(),
            variables: Default::default(),
        }
    }

    fn lower_typecheck_of_item(mut self) -> mir::FnBytecode {
        let typed_expressions = self.db.base_type_check(self.item_entity);
        let fn_body = self.db.fn_body(self.item_entity);

        println!("typed_expressions: {:#?}", typed_expressions);
        println!("fn_body: {:#?}", fn_body);

        match fn_body.value.tables[fn_body.value.root_expression] {
            hir::ExpressionData::Place { place, perm } => match fn_body.value.tables[place] {
                hir::PlaceData::Variable(variable) => {
                    let variable_data = fn_body.value.tables[variable];
                    let variable_type = typed_expressions.value.ty(variable).base.untern(self.db);

                    let boolean_entity = EntityData::LangItem(LangItem::Boolean).intern(self.db);
                    let uint_entity = EntityData::LangItem(LangItem::Uint).intern(self.db);

                    match variable_type.kind {
                        lark_ty::BaseKind::Named(entity) => {
                            if entity == boolean_entity {
                                println!("boolean");
                            } else if entity == uint_entity {
                                println!("uint");
                            }
                        }
                        _ => println!("Unknown basetype kind"),
                    }
                }
                _ => unimplemented!("Not a variable"),
            },
            _ => unimplemented!("Not a variable"),
        }

        mir::FnBytecode {
            basic_blocks: mir::List::default(),
            local_decls: mir::List::default(),
            arg_count: 0,
            name: String::new(),
            tables: self.fn_bytecode_tables,
        }
    }
}
