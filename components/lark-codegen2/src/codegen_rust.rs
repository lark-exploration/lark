use ast::AstDatabase;
use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData, ItemKind, LangItem, MemberKind};
use lark_hir::HirDatabase;
use lark_mir2::MirDatabase;
use lark_query_system::LarkDatabase;
use lark_ty::Ty;
use parser::ReaderDatabase;

pub fn build_type(db: &mut LarkDatabase, ty: &Ty<lark_ty::declaration::Declaration>) -> String {
    let boolean_entity = EntityData::LangItem(LangItem::Boolean).intern(db);
    let uint_entity = EntityData::LangItem(LangItem::Uint).intern(db);

    match ty.base.untern(db) {
        lark_ty::BoundVarOr::BoundVar(_) => unimplemented!("Bound variables not yet supported"),
        lark_ty::BoundVarOr::Known(ty) => match ty.kind {
            lark_ty::BaseKind::Named(entity) => {
                if entity == boolean_entity {
                    "bool".into()
                } else if entity == uint_entity {
                    "u32".into()
                } else {
                    unimplemented!("Unknown type")
                }
            }
            _ => unimplemented!("Unknown base kind"),
        },
    }
}

pub fn codegen_function(
    db: &mut LarkDatabase,
    entity: Entity,
    id: lark_string::global::GlobalIdentifier,
    output: &mut String,
) {
    let mir_bytecode = db.fn_bytecode(entity);
    let signature = db.signature(entity).value.unwrap();
    let fn_body = db.fn_body(entity).value;

    let name = id.untern(db);

    output.push_str(&format!("fn {}(", name));

    let mut first = true;
    for (argument, argument_type) in fn_body
        .arguments
        .iter(&fn_body)
        .zip(signature.inputs.iter())
    {
        let variable = fn_body.tables[argument];
        let identifier = fn_body.tables[variable.name];
        let argument_name = identifier.text.untern(db);

        if !first {
            output.push_str(", ");
        } else {
            first = false;
        }

        output.push_str(&format!("{}: ", argument_name));
        output.push_str(&format!("{}", build_type(db, argument_type)));
    }

    output.push_str(") -> ");
    output.push_str(&format!("{}", build_type(db, &signature.output)));
    output.push_str(" {\n");

    output.push_str("}\n");
}

/// Converts the MIR context of definitions into Rust source
pub fn codegen_rust(db: &mut LarkDatabase) -> String {
    let mut output = String::new();
    let input_files = db.paths();

    for &input_file in &*input_files {
        let entities = db.items_in_file(input_file);

        for &entity in &*entities {
            match entity.untern(&db) {
                EntityData::ItemName {
                    kind: ItemKind::Function,
                    id,
                    ..
                } => {
                    codegen_function(db, entity, id, &mut output);
                }
                _ => {}
            }
        }
    }

    output
}
