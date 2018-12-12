use lark_debug_with::DebugWith;
use lark_entity::{EntityData, ItemKind, LangItem};
use lark_hir as hir;
use lark_intern::{Intern, Untern};
use lark_parser::{ParserDatabase, ParserDatabaseExt};
use lark_query_system::LarkDatabase;
use std::collections::HashMap;
use std::fmt;

pub struct EvalState {
    pub variables: HashMap<hir::Variable, Vec<Value>>,
    pub skip_until: Option<hir::Expression>,
    pub current_expression: Option<hir::Expression>,
    pub is_repl: bool,
}

impl EvalState {
    pub fn create_variable(&mut self, variable: hir::Variable) {
        let variable_stack = self.variables.entry(variable).or_insert(Vec::new());
        variable_stack.push(Value::Void);
    }

    pub fn pop_variable(&mut self, variable: hir::Variable) {
        let variable_stack = self.variables.get_mut(&variable).unwrap();
        variable_stack.pop();
    }

    pub fn assign_to_variable(&mut self, variable: hir::Variable, value: Value) {
        let variable_stack = self.variables.get_mut(&variable).unwrap();
        *variable_stack.last_mut().unwrap() = value;
    }

    pub fn new() -> EvalState {
        EvalState {
            variables: HashMap::new(),
            skip_until: None,
            current_expression: None,
            is_repl: false,
        }
    }

    pub fn set_current_expression(&mut self, expression: hir::Expression) {
        self.current_expression = Some(expression);

        if self.skip_until.is_some() {
            if let Some(skip_until_expr) = self.skip_until {
                if skip_until_expr == expression {
                    self.skip_until = None;
                }
            }
        }
    }

    pub fn ready_to_execute(&self) -> bool {
        self.skip_until.is_none()
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

pub fn eval_place(
    db: &LarkDatabase,
    fn_body: &hir::FnBody,
    place: hir::Place,
    state: &mut EvalState,
) -> Value {
    let place_data = &fn_body.tables[place];

    match place_data {
        hir::PlaceData::Entity(entity) => match entity.untern(db) {
            EntityData::LangItem(LangItem::True) => Value::Bool(true),
            EntityData::LangItem(LangItem::False) => Value::Bool(false),
            _ => unimplemented!("EntityData not yet support in eval"),
        },
        hir::PlaceData::Variable(variable) => {
            let stack = state.variables.get(variable).unwrap();
            stack.last().unwrap().clone()
        }
        hir::PlaceData::Field { owner, name } => {
            let target = eval_place(db, fn_body, *owner, state);
            match target {
                Value::Struct(s) => match fn_body.tables[*name] {
                    hir::IdentifierData { text } => s.get(&text).unwrap().clone(),
                },
                _ => panic!("Member access (.) into value that is not a struct"),
            }
        }
        hir::PlaceData::Temporary { .. } => unimplemented!("Can't yet eval temporary places"),
    }
}

pub fn eval_expression(
    db: &LarkDatabase,
    fn_body: &hir::FnBody,
    expression: hir::Expression,
    state: &mut EvalState,
    io_handler: &mut IOHandler,
) -> Value {
    match fn_body.tables[expression] {
        hir::ExpressionData::Unit { .. } => {}
        _ => state.set_current_expression(expression),
    }

    let ready_to_execute = state.ready_to_execute();

    match fn_body.tables[expression] {
        hir::ExpressionData::Let {
            variable,
            initializer,
            body,
        } => {
            if ready_to_execute {
                state.create_variable(variable);
            }

            if let Some(expression) = initializer {
                let result = eval_expression(db, fn_body, expression, state, io_handler);

                if ready_to_execute {
                    state.assign_to_variable(variable, result);
                }
            }

            let body_result = eval_expression(db, fn_body, body, state, io_handler);

            if !state.is_repl {
                state.pop_variable(variable);
            }

            body_result
        }

        hir::ExpressionData::Place { place } => {
            if ready_to_execute {
                eval_place(db, fn_body, place, state)
            } else {
                Value::Void
            }
        }

        hir::ExpressionData::Assignment { place, value } => {
            let rhs = eval_expression(db, fn_body, value, state, io_handler);
            match &fn_body.tables[place] {
                hir::PlaceData::Variable(variable) => {
                    if ready_to_execute {
                        println!("Assigning {:?} to variable", rhs);
                        state.assign_to_variable(*variable, rhs);
                    }
                }
                _ => unimplemented!("PlaceData not yet supported in eval"),
            }
            Value::Void
        }

        hir::ExpressionData::Call {
            function,
            arguments,
        } => match &fn_body.tables[function] {
            hir::PlaceData::Entity(entity) => match entity.untern(db) {
                EntityData::LangItem(LangItem::Debug) => {
                    for argument in arguments.iter(fn_body) {
                        let result = eval_expression(db, fn_body, argument, state, io_handler);

                        if ready_to_execute {
                            io_handler.println(format!("{}", result));
                        }
                    }

                    Value::Void
                }
                EntityData::ItemName { .. } => {
                    let target = db.fn_body(*entity).value;

                    for (arg, param) in arguments
                        .iter(fn_body)
                        .zip(target.arguments.unwrap().iter(&target))
                    {
                        let arg_value = eval_expression(db, fn_body, arg, state, io_handler);
                        state.create_variable(param);
                        state.assign_to_variable(param, arg_value);
                    }

                    let return_value = if ready_to_execute {
                        eval_function(db, &target, state, io_handler)
                    } else {
                        Value::Void
                    };

                    for argument in target.arguments.unwrap().iter(&target) {
                        state.pop_variable(argument);
                    }

                    return_value
                }
                x => unimplemented!(
                    "Call entity not yet supported in eval: {:#?}",
                    x.debug_with(db)
                ),
            },
            x => unimplemented!("Call not yet supported in eval: {:#?}", x.debug_with(db)),
        },

        hir::ExpressionData::Sequence { first, second } => {
            eval_expression(db, fn_body, first, state, io_handler);
            eval_expression(db, fn_body, second, state, io_handler)
        }

        hir::ExpressionData::Binary {
            operator,
            left,
            right,
        } => {
            let lhs_eval = eval_expression(db, fn_body, left, state, io_handler);
            let rhs_eval = eval_expression(db, fn_body, right, state, io_handler);

            if ready_to_execute {
                match operator {
                    hir::BinaryOperator::Add => match (lhs_eval, rhs_eval) {
                        (Value::U32(l), Value::U32(r)) => Value::U32(l + r),
                        _ => panic!("Addition of non-numeric values"),
                    },
                    hir::BinaryOperator::Subtract => match (lhs_eval, rhs_eval) {
                        (Value::U32(l), Value::U32(r)) => Value::U32(l - r),
                        _ => panic!("Subtraction of non-numeric values"),
                    },
                    _ => unimplemented!("Operator not yet supported"),
                }
            } else {
                Value::Void
            }
        }

        hir::ExpressionData::Literal { data } => match data {
            hir::LiteralData {
                kind: hir::LiteralKind::UnsignedInteger,
                value,
            } => {
                if ready_to_execute {
                    let string = value.untern(db);
                    let value: u32 = string.parse().unwrap();
                    Value::U32(value)
                } else {
                    Value::Void
                }
            }
            _ => unimplemented!("Unsupported literal value"),
        },

        hir::ExpressionData::Aggregate { fields, .. } => {
            let mut result_struct = HashMap::new();

            for identified_expression in fields.iter(fn_body) {
                let hir::IdentifiedExpressionData {
                    identifier,
                    expression,
                } = fn_body.tables[identified_expression];
                let arg_result = eval_expression(db, fn_body, expression, state, io_handler);

                result_struct.insert(fn_body.tables[identifier].text, arg_result);
            }

            if ready_to_execute {
                Value::Struct(result_struct)
            } else {
                Value::Void
            }
        }

        hir::ExpressionData::Unit {} => Value::Void,

        ref x => unimplemented!(
            "Eval does not yet support this expression type: {:#?}",
            x.debug_with(db)
        ),
    }
}

pub fn eval_function(
    db: &LarkDatabase,
    fn_body: &hir::FnBody,
    state: &mut EvalState,
    io_handler: &mut IOHandler,
) -> Value {
    eval_expression(db, fn_body, fn_body.root_expression, state, io_handler)
}

pub fn eval(db: &LarkDatabase, io_handler: &mut IOHandler) {
    let input_files = db.file_names();

    let mut eval_state = EvalState::new();

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
                        let fn_body = db.fn_body(entity);

                        eval_function(db, &fn_body.value, &mut eval_state, io_handler);
                    }
                }
                _ => {}
            }
        }
    }
}
