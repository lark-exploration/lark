use lark_debug_with::DebugWith;
use lark_entity::{EntityData, ItemKind, LangItem};
use lark_intern::{Intern, Untern};
use lark_mir::{
    BasicBlock, BinOp, FnBytecode, IdentifierData, MirDatabase, Operand, OperandData, Place,
    PlaceData, Rvalue, RvalueData, Statement, StatementKind, Variable,
};
use lark_parser::{ParserDatabase, ParserDatabaseExt};
use lark_query_system::LarkDatabase;
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug)]
pub enum Value {
    Void,
    Bool(bool),
    U32(u32),
    Str(String),
    Struct(HashMap<lark_string::GlobalIdentifier, Value>),
    Reference(usize), // a reference into the value stack
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Value::U32(u) => u.to_string(),
                Value::Str(s) => s.clone(),
                Value::Bool(b) => b.to_string(),
                Value::Reference(r) => format!("reference to {}", r),
                Value::Void => "<void>".into(),
                Value::Struct(s) => format!("{:?}", s),
            }
        )
    }
}

pub struct IOHandler {
    pub redirect: Option<String>,
}

impl IOHandler {
    pub fn new(redirect_output: bool) -> IOHandler {
        if redirect_output {
            IOHandler {
                redirect: Some(String::new()),
            }
        } else {
            IOHandler { redirect: None }
        }
    }

    pub fn println(&mut self, output: String) {
        if let Some(redirect_output) = &mut self.redirect {
            redirect_output.push_str(&output);
            redirect_output.push_str("\n");
        } else {
            println!("{}", output);
        }
    }
}

pub fn eval_place(
    db: &LarkDatabase,
    fn_bytecode: &FnBytecode,
    place: Place,
    variables: &mut HashMap<Variable, Vec<Value>>,
) -> Value {
    let place_data = &fn_bytecode.tables[place];

    match place_data {
        PlaceData::Entity(entity) => match entity.untern(db) {
            EntityData::LangItem(LangItem::True) => Value::Bool(true),
            EntityData::LangItem(LangItem::False) => Value::Bool(false),
            _ => unimplemented!("EntityData not yet support in eval"),
        },
        PlaceData::Variable(variable) => {
            let stack = variables.get(variable).unwrap();
            stack.last().unwrap().clone()
        }
        PlaceData::Field { owner, name } => {
            let target = eval_place(db, fn_bytecode, *owner, variables);
            match target {
                Value::Struct(s) => match fn_bytecode.tables[*name] {
                    IdentifierData { text } => s.get(&text).unwrap().clone(),
                },
                _ => panic!("Member access (.) into value that is not a struct"),
            }
        }
    }
}

pub fn eval_operand(
    db: &LarkDatabase,
    fn_bytecode: &FnBytecode,
    operand: Operand,
    variables: &mut HashMap<Variable, Vec<Value>>,
) -> Value {
    let operand_data = &fn_bytecode.tables[operand];

    match operand_data {
        OperandData::Copy(place) | OperandData::Move(place) => {
            eval_place(db, fn_bytecode, *place, variables)
        }
        OperandData::ConstantInt(u) => Value::U32(*u),
        _ => unimplemented!("Operand not yet supported"),
    }
}

pub fn eval_rvalue(
    db: &LarkDatabase,
    fn_bytecode: &FnBytecode,
    rvalue: Rvalue,
    variables: &mut HashMap<Variable, Vec<Value>>,
    io_handler: &mut IOHandler,
) -> Value {
    let rvalue_data = &fn_bytecode.tables[rvalue];

    match rvalue_data {
        RvalueData::Use(operand) => eval_operand(db, fn_bytecode, *operand, variables),
        RvalueData::Call(entity, operands) => match entity.untern(db) {
            EntityData::LangItem(LangItem::Debug) => {
                for operand in operands.iter(fn_bytecode) {
                    let result = eval_operand(db, fn_bytecode, operand, variables);
                    io_handler.println(format!("{}", result));
                }

                Value::Void
            }
            EntityData::ItemName { .. } => {
                let bytecode = db.fn_bytecode(*entity).value;

                for (arg, param) in operands
                    .iter(fn_bytecode)
                    .zip(bytecode.arguments.iter(&bytecode))
                {
                    let arg_value = eval_operand(db, fn_bytecode, arg, variables);
                    create_variable(variables, param);
                    assign_to_variable(variables, param, arg_value);
                }

                let return_value = eval_function(db, &bytecode, variables, io_handler);

                for argument in fn_bytecode.arguments.iter(&fn_bytecode) {
                    pop_variable(variables, argument);
                }

                return_value
            }
            x => unimplemented!(
                "EntityData not yet supported in eval: {:#?}",
                x.debug_with(db)
            ),
        },
        RvalueData::Aggregate(entity, args) => {
            let members = db.members(*entity).unwrap();
            let mut result_struct = HashMap::new();

            for (member, arg) in members.iter().zip(args.iter(fn_bytecode)) {
                let arg_result = eval_operand(db, fn_bytecode, arg, variables);
                result_struct.insert(member.name, arg_result);
            }

            Value::Struct(result_struct)
        }
        RvalueData::BinaryOp(binop, lhs, rhs) => {
            let lhs_eval = eval_operand(db, fn_bytecode, *lhs, variables);
            let rhs_eval = eval_operand(db, fn_bytecode, *rhs, variables);

            match binop {
                BinOp::Add => match (lhs_eval, rhs_eval) {
                    (Value::U32(l), Value::U32(r)) => Value::U32(l + r),
                    _ => panic!("Addition of non-numeric values"),
                },
                BinOp::Sub => match (lhs_eval, rhs_eval) {
                    (Value::U32(l), Value::U32(r)) => Value::U32(l - r),
                    _ => panic!("Subtraction of non-numeric values"),
                },
            }
        }
    }
}

pub fn create_variable(variables: &mut HashMap<Variable, Vec<Value>>, variable: Variable) {
    let variable_stack = variables.entry(variable).or_insert(Vec::new());
    variable_stack.push(Value::Void);
}

pub fn pop_variable(variables: &mut HashMap<Variable, Vec<Value>>, variable: Variable) {
    let variable_stack = variables.get_mut(&variable).unwrap();
    variable_stack.pop();
}

pub fn assign_to_variable(
    variables: &mut HashMap<Variable, Vec<Value>>,
    variable: Variable,
    value: Value,
) {
    let variable_stack = variables.get_mut(&variable).unwrap();
    *variable_stack.last_mut().unwrap() = value;
}

pub fn eval_statement(
    db: &LarkDatabase,
    fn_bytecode: &FnBytecode,
    statement: Statement,
    variables: &mut HashMap<Variable, Vec<Value>>,
    io_handler: &mut IOHandler,
) -> Value {
    let statement_data = &fn_bytecode.tables[statement];

    match &statement_data.kind {
        StatementKind::Expression(rvalue) => {
            eval_rvalue(db, fn_bytecode, *rvalue, variables, io_handler)
        }
        StatementKind::Assign(place, rvalue) => {
            let rhs = eval_rvalue(db, fn_bytecode, *rvalue, variables, io_handler);
            match &fn_bytecode.tables[*place] {
                PlaceData::Variable(variable) => assign_to_variable(variables, *variable, rhs),
                _ => unimplemented!("PlaceData not yet supported in eval"),
            }
            Value::Void
        }
        _ => {
            // If we get here, something went wrong. Statements like StorageLive/StorageDead
            // Need to get handled where they won't effect the return value of the block
            panic!("Unexpected StatementKind");
        }
    }
}

pub fn eval_basic_block(
    db: &LarkDatabase,
    fn_bytecode: &FnBytecode,
    basic_block: BasicBlock,
    variables: &mut HashMap<Variable, Vec<Value>>,
    io_handler: &mut IOHandler,
) -> Value {
    let basic_block_data = &fn_bytecode.tables[basic_block];
    let mut return_value = Value::Void;

    for statement in basic_block_data.statements.iter(&fn_bytecode) {
        let statement_data = &fn_bytecode.tables[statement];

        match &statement_data.kind {
            StatementKind::StorageLive(variable) => {
                create_variable(variables, *variable);
            }
            StatementKind::StorageDead(variable) => {
                pop_variable(variables, *variable);
            }
            _ => return_value = eval_statement(db, fn_bytecode, statement, variables, io_handler),
        }
    }

    return_value
}

pub fn eval_function(
    db: &LarkDatabase,
    fn_bytecode: &FnBytecode,
    variables: &mut HashMap<Variable, Vec<Value>>,
    io_handler: &mut IOHandler,
) -> Value {
    let mut return_value = Value::Void;
    for basic_block in fn_bytecode.basic_blocks.iter(fn_bytecode) {
        return_value = eval_basic_block(db, fn_bytecode, basic_block, variables, io_handler);
    }

    return_value
}

pub fn eval(db: &LarkDatabase, io_handler: &mut IOHandler) {
    let input_files = db.file_names();
    //let mut errors: Vec<Diagnostic> = vec![];

    let mut variables: HashMap<Variable, Vec<Value>> = HashMap::new();
    let main_name = "main".intern(&db);

    for &input_file in &*input_files {
        let entities = db.top_level_entities_in_file(input_file);

        for &entity in &*entities {
            match entity.untern(&db) {
                EntityData::ItemName {
                    kind: ItemKind::Function,
                    id,
                    ..
                } => {
                    if id == main_name {
                        let bytecode = db.fn_bytecode(entity);

                        eval_function(db, &bytecode.value, &mut variables, io_handler);
                    }
                }
                _ => {}
            }
        }
    }
}
