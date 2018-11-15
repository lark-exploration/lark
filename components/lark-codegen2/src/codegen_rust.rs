use ast::AstDatabase;
use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData, ItemKind, LangItem, MemberKind};
use lark_hir::HirDatabase;
use lark_mir2::{
    BasicBlock, FnBytecode, MirDatabase, Operand, OperandData, Rvalue, RvalueData, Statement,
    StatementKind,
};
use lark_query_system::LarkDatabase;
use lark_ty::Ty;
use parser::ReaderDatabase;

pub fn build_type(db: &mut LarkDatabase, ty: &Ty<lark_ty::declaration::Declaration>) -> String {
    let boolean_entity = EntityData::LangItem(LangItem::Boolean).intern(db);
    let uint_entity = EntityData::LangItem(LangItem::Uint).intern(db);
    let void_entity = EntityData::LangItem(LangItem::Tuple(0)).intern(db);

    match ty.base.untern(db) {
        lark_ty::BoundVarOr::BoundVar(_) => unimplemented!("Bound variables not yet supported"),
        lark_ty::BoundVarOr::Known(ty) => match ty.kind {
            lark_ty::BaseKind::Named(entity) => {
                if entity == boolean_entity {
                    "bool".into()
                } else if entity == uint_entity {
                    "u32".into()
                } else if entity == void_entity {
                    "()".into()
                } else {
                    unimplemented!("Unknown type: {:#?}", entity)
                }
            }
            _ => unimplemented!("Unknown base kind"),
        },
    }
}

fn build_var_name(
    db: &mut LarkDatabase,
    fn_bytecode: &std::sync::Arc<FnBytecode>,
    variable: lark_mir2::Variable,
) -> String {
    let variable_data = fn_bytecode.tables[variable];
    let identifier = fn_bytecode.tables[variable_data.name];
    identifier.text.untern(db).to_string()
}

fn build_entity_name(
    db: &mut LarkDatabase,
    fn_bytecode: &std::sync::Arc<FnBytecode>,
    entity: Entity,
) -> String {
    let entity_data = entity.untern(db);
    match entity_data {
        EntityData::LangItem(LangItem::False) => "false".into(),
        EntityData::LangItem(LangItem::True) => "true".into(),
        EntityData::LangItem(LangItem::Debug) => "println!".into(),
        _ => unimplemented!("Unsupported entity name"),
    }
}

fn build_operand(
    db: &mut LarkDatabase,
    fn_bytecode: &std::sync::Arc<FnBytecode>,
    operand: Operand,
) -> String {
    match &fn_bytecode.tables[operand] {
        OperandData::ConstantInt(i) => format!("{}", i),
        OperandData::ConstantString(s) => format!("\"{}\"", s),
        OperandData::Copy(place) | OperandData::Move(place) => {
            let place_data = &fn_bytecode.tables[*place];
            match place_data {
                lark_mir2::PlaceData::Variable(variable) => {
                    build_var_name(db, fn_bytecode, *variable)
                }
                lark_mir2::PlaceData::Entity(entity) => build_entity_name(db, fn_bytecode, *entity),
                x => unimplemented!("Unsupported place data: {:#?}", x),
            }
        }
    }
}

pub fn codegen_struct(
    db: &mut LarkDatabase,
    entity: Entity,
    id: lark_string::global::GlobalIdentifier,
    output: &mut String,
) {
    let name = id.untern(db);
    let members = db.members(entity).unwrap();

    output.push_str(&format!("struct {} {{\n", name));

    for member in members.iter() {
        let member_name = member.name.untern(db);
        let member_ty = db.ty(member.entity);
        output.push_str(&format!(
            "{}: {},\n",
            member_name,
            build_type(db, &member_ty.value)
        ));
    }

    output.push_str("}\n");
}

pub fn codegen_rvalue(
    db: &mut LarkDatabase,
    rvalue: Rvalue,
    fn_bytecode: &std::sync::Arc<FnBytecode>,
    output: &mut String,
) {
    match &fn_bytecode.tables[rvalue] {
        RvalueData::Use(operand) => output.push_str(&build_operand(db, fn_bytecode, *operand)),
        RvalueData::Call(entity, args) => {
            output.push_str(&build_entity_name(db, fn_bytecode, *entity));

            output.push_str("(");
            let mut first = true;

            match entity.untern(db) {
                EntityData::LangItem(LangItem::Debug) => {
                    output.push_str("\"{}\"");
                    first = false;
                }
                _ => {}
            }

            for arg in args.iter(fn_bytecode) {
                if !first {
                    output.push_str(", ");
                } else {
                    first = false;
                }

                output.push_str(&build_operand(db, fn_bytecode, arg));
            }
            output.push_str(")");
        }
        x => unimplemented!("Rvalue value not supported: {:?}", x),
    }
}

pub fn codegen_statement(
    db: &mut LarkDatabase,
    statement: Statement,
    fn_bytecode: &std::sync::Arc<FnBytecode>,
    output: &mut String,
) {
    let statement_data = &fn_bytecode.tables[statement];

    match &statement_data.kind {
        StatementKind::Expression(rvalue) => {
            codegen_rvalue(db, *rvalue, fn_bytecode, output);
        }
        _ => unimplemented!("Unsupported statement kind"),
    }

    output.push_str(";\n")
}

pub fn codegen_basic_block(
    db: &mut LarkDatabase,
    basic_block: BasicBlock,
    fn_bytecode: &std::sync::Arc<FnBytecode>,
    output: &mut String,
) {
    let basic_block_data = &fn_bytecode.tables[basic_block];

    for statement in basic_block_data.statements.iter(&fn_bytecode) {
        codegen_statement(db, statement, fn_bytecode, output);
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
        let variable_data = fn_body.tables[argument];
        let identifier = fn_body.tables[variable_data.name];

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
    for basic_block in mir_bytecode.value.basic_blocks.iter(&mir_bytecode.value) {
        codegen_basic_block(db, basic_block, &mir_bytecode.value, output);
    }
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
                EntityData::ItemName {
                    kind: ItemKind::Struct,
                    id,
                    ..
                } => {
                    codegen_struct(db, entity, id, &mut output);
                }
                x => unimplemented!("Can not codegen {:#?}", x),
            }
        }
    }

    output
}
