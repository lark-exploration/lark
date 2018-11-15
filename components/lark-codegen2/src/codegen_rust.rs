use ast::AstDatabase;
use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData, ItemKind, MemberKind};
use lark_mir2::MirDatabase;
use lark_query_system::LarkDatabase;
use parser::ReaderDatabase;

pub fn codegen_function(db: &mut LarkDatabase, entity: Entity, output: &mut String) {
    let mir_bytecode = db.fn_bytecode(entity);

    output.push_str(&format!("{:#?}", mir_bytecode))
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
                    base,
                } => {
                    println!("{:?}", id.untern(db));
                    codegen_function(db, entity, &mut output);
                }
                _ => {}
            }
        }
    }

    output
}
