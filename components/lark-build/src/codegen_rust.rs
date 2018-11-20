use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData, ItemKind, LangItem};
use lark_error::{Diagnostic, WithError};
use lark_mir::{
    BasicBlock, FnBytecode, MirDatabase, Operand, OperandData, Place, PlaceData, Rvalue,
    RvalueData, Statement, StatementKind,
};
use lark_parser::{ParserDatabase, ParserDatabaseExt};
use lark_query_system::LarkDatabase;
use lark_ty::Ty;

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

fn build_variable_name(
    db: &mut LarkDatabase,
    fn_bytecode: &std::sync::Arc<FnBytecode>,
    variable: lark_mir::Variable,
) -> String {
    let variable_data = fn_bytecode.tables[variable];
    let identifier = fn_bytecode.tables[variable_data.name];
    identifier.text.untern(db).to_string()
}

fn build_entity_name(
    db: &mut LarkDatabase,
    _fn_bytecode: &std::sync::Arc<FnBytecode>,
    entity: Entity,
) -> String {
    let entity_data = entity.untern(db);
    match entity_data {
        EntityData::LangItem(LangItem::False) => "false".into(),
        EntityData::LangItem(LangItem::True) => "true".into(),
        EntityData::LangItem(LangItem::Debug) => "println!".into(),
        EntityData::ItemName { id, .. } => id.untern(db).to_string(),
        x => unimplemented!("Unsupported entity name: {:#?}", x),
    }
}

pub fn build_place(
    db: &mut LarkDatabase,
    fn_bytecode: &std::sync::Arc<FnBytecode>,
    place: Place,
) -> String {
    match &fn_bytecode.tables[place] {
        PlaceData::Variable(variable) => build_variable_name(db, fn_bytecode, *variable),
        PlaceData::Entity(entity) => build_entity_name(db, fn_bytecode, *entity),
        x => unimplemented!("Unsupported place data: {:#?}", x),
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
            //FIXME: separate copy and move
            build_place(db, fn_bytecode, *place)
        }
    }
}

pub fn codegen_struct(
    db: &mut LarkDatabase,
    entity: Entity,
    id: lark_string::global::GlobalIdentifier,
) -> WithError<String> {
    let name = id.untern(db);
    let members = db.members(entity).unwrap();
    let mut output = String::new();
    let mut errors: Vec<Diagnostic> = vec![];

    output.push_str(&format!("struct {} {{\n", name));

    for member in members.iter() {
        let member_name = member.name.untern(db);
        let member_ty = db.ty(member.entity).accumulate_errors_into(&mut errors);
        output.push_str(&format!(
            "{}: {},\n",
            member_name,
            build_type(db, &member_ty)
        ));
    }

    output.push_str("}\n");

    WithError {
        value: output,
        errors,
    }
}

pub fn codegen_rvalue(
    db: &mut LarkDatabase,
    fn_bytecode: &std::sync::Arc<FnBytecode>,
    rvalue: Rvalue,
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
            codegen_rvalue(db, fn_bytecode, *rvalue, output);
        }
        StatementKind::Assign(lvalue, rvalue) => {
            output.push_str(&format!("{} = ", build_place(db, fn_bytecode, *lvalue)));
            codegen_rvalue(db, fn_bytecode, *rvalue, output);
        }
        StatementKind::StorageLive(variable) => {
            output.push_str(&format!(
                "{{\nlet {};\n",
                build_variable_name(db, fn_bytecode, *variable)
            ));
        }
        StatementKind::StorageDead(_) => {
            output.push_str("\n}\n");
        }
    }
}

pub fn codegen_basic_block(
    db: &mut LarkDatabase,
    fn_bytecode: &std::sync::Arc<FnBytecode>,
    basic_block: BasicBlock,
    output: &mut String,
) {
    let basic_block_data = &fn_bytecode.tables[basic_block];

    let mut first = true;

    for statement in basic_block_data.statements.iter(&fn_bytecode) {
        if !first {
            output.push_str(";\n")
        } else {
            first = false;
        }
        codegen_statement(db, statement, fn_bytecode, output);
    }
}

pub fn codegen_function(
    db: &mut LarkDatabase,
    entity: Entity,
    id: lark_string::global::GlobalIdentifier,
) -> WithError<String> {
    let mut output = String::new();
    let mut errors: Vec<Diagnostic> = vec![];

    let fn_bytecode = db.fn_bytecode(entity).accumulate_errors_into(&mut errors);
    let signature = db
        .signature(entity)
        .accumulate_errors_into(&mut errors)
        .unwrap();

    let name = id.untern(db);

    output.push_str(&format!("fn {}(", name));

    let mut first = true;
    for (argument, argument_type) in fn_bytecode
        .arguments
        .iter(&fn_bytecode)
        .zip(signature.inputs.iter())
    {
        let variable_data = fn_bytecode.tables[argument];
        let identifier = fn_bytecode.tables[variable_data.name];

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
    for basic_block in fn_bytecode.basic_blocks.iter(&fn_bytecode) {
        codegen_basic_block(db, &fn_bytecode, basic_block, &mut output);
    }
    output.push_str("}\n");

    WithError {
        value: output,
        errors,
    }
}

/// Converts the MIR context of definitions into Rust source
pub fn codegen_rust(db: &mut LarkDatabase) -> WithError<String> {
    let mut output = String::new();
    let input_files = db.file_names();
    let mut errors: Vec<Diagnostic> = vec![];

    for &input_file in &*input_files {
        let entities = db.top_level_entities_in_file(input_file);

        for &entity in &*entities {
            match entity.untern(&db) {
                EntityData::ItemName {
                    kind: ItemKind::Function,
                    id,
                    ..
                } => {
                    let mut result = codegen_function(db, entity, id);
                    if result.errors.len() > 0 {
                        errors.append(&mut result.errors);
                    } else {
                        output.push_str(&result.value);
                    }
                }
                EntityData::ItemName {
                    kind: ItemKind::Struct,
                    id,
                    ..
                } => {
                    let mut result = codegen_struct(db, entity, id);
                    if result.errors.len() > 0 {
                        errors.append(&mut result.errors);
                    } else {
                        output.push_str(&result.value);
                    }
                }
                x => unimplemented!("Can not codegen {:#?}", x),
            }
        }
    }

    WithError {
        value: output,
        errors,
    }
}
