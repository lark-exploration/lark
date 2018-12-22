use lark_debug_with::DebugWith;
use lark_entity::{Entity, EntityData, ItemKind, LangItem};
use lark_error::{Diagnostic, WithError};
use lark_hir as hir;
use lark_intern::{Intern, Untern};
use lark_parser::{ParserDatabase, ParserDatabaseExt};
use lark_query_system::LarkDatabase;
use lark_ty::Ty;

fn build_variable_name(
    db: &LarkDatabase,
    fn_body: &std::sync::Arc<hir::FnBody>,
    variable: lark_hir::Variable,
) -> String {
    let variable_data = fn_body.tables[variable];
    let identifier = fn_body.tables[variable_data.name];
    identifier.text.untern(db).to_string()
}

fn build_entity_name(db: &LarkDatabase, entity: Entity) -> String {
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
    db: &LarkDatabase,
    fn_body: &std::sync::Arc<hir::FnBody>,
    place: hir::Place,
) -> String {
    match &fn_body.tables[place] {
        hir::PlaceData::Variable(variable) => build_variable_name(db, fn_body, *variable),
        hir::PlaceData::Entity(entity) => build_entity_name(db, *entity),
        hir::PlaceData::Field { owner, name } => {
            let identifier = fn_body.tables[*name];

            format!(
                "{}.{}",
                build_place(db, fn_body, *owner),
                identifier.text.untern(db).to_string()
            )
        }
        hir::PlaceData::Temporary(expression) => build_expression(db, fn_body, *expression),
    }
}

pub fn build_type(db: &LarkDatabase, ty: &Ty<lark_ty::declaration::Declaration>) -> String {
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

pub fn codegen_struct(
    db: &LarkDatabase,
    entity: Entity,
    id: lark_string::GlobalIdentifier,
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

pub fn build_expression(
    db: &LarkDatabase,
    fn_body: &std::sync::Arc<hir::FnBody>,
    expression: hir::Expression,
) -> String {
    match fn_body.tables[expression] {
        hir::ExpressionData::Let {
            variable,
            initializer,
            body,
        } => match initializer {
            Some(init_expression) => format!(
                "{{ let {} = {};\n{}}}",
                build_variable_name(db, fn_body, variable),
                build_expression(db, fn_body, init_expression),
                build_expression(db, fn_body, body),
            ),
            None => format!("let {};\n", build_variable_name(db, fn_body, variable)),
        },

        hir::ExpressionData::Place { place } => build_place(db, fn_body, place),

        hir::ExpressionData::Assignment { place, value } => format!(
            "{} = {};\n",
            build_place(db, fn_body, place),
            build_expression(db, fn_body, value)
        ),

        hir::ExpressionData::MethodCall { method, arguments } => {
            let mut arguments = arguments.iter(fn_body);
            let mut output = String::new();

            output.push_str(&build_expression(db, fn_body, arguments.next().unwrap()));

            let method_name = fn_body.tables[method].text.untern(db);
            output.push_str(&format!(".{}(", method_name));

            let mut first = true;
            for argument in arguments {
                if !first {
                    output.push_str(", ");
                } else {
                    first = false;
                }
                output.push_str(&build_expression(db, fn_body, argument));
            }
            output.push_str(")");

            output
        }

        hir::ExpressionData::Call {
            function,
            arguments,
        } => {
            let mut output = String::new();

            output.push_str(&build_expression(db, fn_body, function));

            output.push_str("(");

            let mut first = true;

            match fn_body[function] {
                hir::ExpressionData::Place {
                    place: function_place,
                } => match fn_body[function_place] {
                    hir::PlaceData::Entity(entity) => match entity.untern(db) {
                        EntityData::LangItem(LangItem::Debug) => {
                            output.push_str("\"{}\"");
                            first = false;
                        }
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }

            for argument in arguments.iter(fn_body) {
                if !first {
                    output.push_str(", ");
                } else {
                    first = false;
                }
                output.push_str(&build_expression(db, fn_body, argument));
            }
            output.push_str(")");

            output
        }

        hir::ExpressionData::Sequence { first, second } => format!(
            "{};\n {}",
            build_expression(db, fn_body, first),
            build_expression(db, fn_body, second)
        ),

        hir::ExpressionData::If {
            condition,
            if_true,
            if_false,
        } => format!(
            "if {} {{ {} \n}} else {{ {} \n}}",
            build_expression(db, fn_body, condition),
            build_expression(db, fn_body, if_true),
            build_expression(db, fn_body, if_false)
        ),

        hir::ExpressionData::Binary {
            operator,
            left,
            right,
        } => format!(
            "({} {} {})",
            build_expression(db, fn_body, left),
            match operator {
                hir::BinaryOperator::Add => "+",
                hir::BinaryOperator::Subtract => "-",
                hir::BinaryOperator::Multiply => "*",
                hir::BinaryOperator::Divide => "/",
                hir::BinaryOperator::Equals => "==",
                hir::BinaryOperator::NotEquals => "!=",
            },
            build_expression(db, fn_body, right),
        ),

        hir::ExpressionData::Unary { operator, value } => format!(
            "{}({})",
            match operator {
                hir::UnaryOperator::Not => "!",
            },
            build_expression(db, fn_body, value)
        ),

        hir::ExpressionData::Literal { data } => match data {
            hir::LiteralData {
                kind: hir::LiteralKind::String,
                value,
            } => format!("\"{}\"", value.untern(db)),
            hir::LiteralData {
                kind: hir::LiteralKind::UnsignedInteger,
                value,
            } => format!("{}", value.untern(db)),
        },

        hir::ExpressionData::Unit {} => "()".to_string(),

        hir::ExpressionData::Aggregate { entity, fields } => {
            let mut output = String::new();

            output.push_str(&build_entity_name(db, entity));
            output.push_str("{");
            for field in fields.iter(fn_body) {
                let identified_expression = fn_body.tables[field];
                output.push_str(&format!(
                    "{}: {}",
                    fn_body.tables[identified_expression.identifier]
                        .text
                        .untern(db),
                    build_expression(db, fn_body, identified_expression.expression),
                ));
            }
            output.push_str("}");
            output
        }

        hir::ExpressionData::Error { .. } => {
            panic!("Can not codegen in the presence of errors");
        }
    }
}

pub fn codegen_function(
    db: &LarkDatabase,
    entity: Entity,
    id: lark_string::GlobalIdentifier,
) -> WithError<String> {
    let mut output = String::new();
    let mut errors: Vec<Diagnostic> = vec![];

    let fn_body = db.fn_body(entity).accumulate_errors_into(&mut errors);

    let signature = db
        .signature(entity)
        .accumulate_errors_into(&mut errors)
        .unwrap();

    let arguments = fn_body.arguments.unwrap();

    let name = id.untern(db);

    output.push_str(&format!("fn {}(", name));

    let mut first = true;
    for (argument, argument_type) in arguments.iter(&fn_body).zip(signature.inputs.iter()) {
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
    output.push_str(&format!(
        " {{\n{} }}\n",
        build_expression(db, &fn_body, fn_body.root_expression)
    ));

    WithError {
        value: output,
        errors,
    }
}

/// Converts the MIR context of definitions into Rust source
pub fn codegen_rust(db: &LarkDatabase) -> WithError<String> {
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
                x => unimplemented!("Can not codegen {:#?}", x.debug_with(db)),
            }
        }
    }

    WithError {
        value: output,
        errors,
    }
}
