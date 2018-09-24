use crate::ir::{
    builtin_type, BinOp, BuiltinFn, Context, DefId, Definition, Function, Operand, Place, Rvalue,
    Statement, StatementKind,
};
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug)]
pub enum Value {
    Void,
    I32(i32),
    Str(String),
    Struct(HashMap<String, Value>),
    Reference(usize), // a reference into the value stack
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Value::I32(i) => i.to_string(),
                Value::Str(s) => s.clone(),
                Value::Reference(r) => format!("reference to {}", r),
                Value::Void => "<void>".into(),
                Value::Struct(s) => format!("{:?}", s),
            }
        )
    }
}

#[derive(Debug)]
pub struct CallFrame {
    locals: Vec<Value>,
}

impl CallFrame {
    fn new() -> CallFrame {
        CallFrame { locals: vec![] }
    }
}

pub fn eval_operand(_context: &Context, frame: &mut CallFrame, operand: &Operand) -> Value {
    match operand {
        Operand::ConstantInt(i) => Value::I32(*i),
        Operand::ConstantString(s) => Value::Str(s.clone()),
        Operand::Move(m) => match m {
            Place::Local(source_var_id) => frame.locals[*source_var_id].clone(),
            Place::Static(_) => unimplemented!("Moving from static data not currently supported"),
            Place::Field(source_var_id, field_name) => match &frame.locals[*source_var_id] {
                Value::Struct(s) => s[field_name].clone(),
                _ => unimplemented!("Field access of non-struct value"),
            },
        },
        Operand::Copy(m) => match m {
            Place::Local(source_var_id) => frame.locals[*source_var_id].clone(),
            Place::Static(_) => unimplemented!("Moving from static data not currently supported"),
            Place::Field(source_var_id, field_name) => match &frame.locals[*source_var_id] {
                Value::Struct(s) => s[field_name].clone(),
                _ => unimplemented!("Field access of non-struct value"),
            },
        },
    }
}

fn eval_rvalue(context: &Context, frame: &mut CallFrame, rvalue: &Rvalue) -> Value {
    match rvalue {
        Rvalue::Use(ref operand) => eval_operand(context, frame, operand),
        Rvalue::Call(def_id, args) => {
            match &context.definitions[*def_id] {
                Definition::Fn(f) => {
                    let mut new_frame = CallFrame::new();
                    new_frame.locals.push(Value::Void); // return value
                    for arg in args {
                        new_frame.locals.push(eval_operand(context, frame, arg));
                    }
                    let num_temps = f.local_decls.len() - 1 - f.arg_count;
                    for _ in 0..num_temps {
                        new_frame.locals.push(Value::Void);
                    }
                    eval_fn(context, &mut new_frame, f);
                    let result = new_frame.locals[0].clone();
                    result
                }
                Definition::BuiltinFn(BuiltinFn::StringInterpolate) => {
                    let mut arg_eval = vec![];
                    for arg in args {
                        arg_eval.push(eval_operand(context, frame, arg));
                    }

                    let format_string = &arg_eval[0];

                    match format_string {
                        Value::Str(s) => {
                            let sections: Vec<&str> = s.split("{}").collect();
                            let sections_len = sections.len();

                            let mut result = String::new();

                            for i in 0..(sections_len - 1) {
                                result += sections[i];
                                result += &format!("{}", arg_eval[i + 1]);
                            }

                            result += sections[sections_len - 1];

                            Value::Str(result)
                        }
                        _ => unimplemented!("String interpolation without a format string"),
                    }
                }
                Definition::Struct(ref s) => {
                    let mut new_obj = HashMap::new();
                    for i in 0..s.fields.len() {
                        new_obj.insert(
                            s.fields[i].name.clone(),
                            eval_operand(context, frame, &args[i]),
                        );
                    }
                    Value::Struct(new_obj)
                }
                _ => unimplemented!("Unsupported call of non-function"),
            }
        }
        Rvalue::BinaryOp(bin_op, lhs_var_id, rhs_var_id) => {
            let lhs = &frame.locals[*lhs_var_id];
            let rhs = &frame.locals[*rhs_var_id];

            match bin_op {
                BinOp::Add => match (lhs, rhs) {
                    (Value::I32(lhs_i32), Value::I32(rhs_i32)) => Value::I32(lhs_i32 + rhs_i32),
                    _ => unimplemented!("Unsupported add of non-integers"),
                },
                BinOp::Sub => match (lhs, rhs) {
                    (Value::I32(lhs_i32), Value::I32(rhs_i32)) => Value::I32(lhs_i32 - rhs_i32),
                    _ => unimplemented!("Unsupported add of non-integers"),
                },
            }
        }
    }
}

pub fn eval_stmt(context: &Context, frame: &mut CallFrame, stmt: &Statement) {
    match &stmt.kind {
        StatementKind::Assign(place, rvalue) => {
            let rval = eval_rvalue(context, frame, rvalue);
            match place {
                Place::Local(target_var_id) => frame.locals[*target_var_id] = rval,
                Place::Static(_) => unimplemented!("Assigning into static currently not supported"),
                Place::Field(source_var_id, field_name) => {
                    match &mut frame.locals[*source_var_id] {
                        Value::Struct(s) => {
                            let _ = s.insert(field_name.clone(), rval);
                        }
                        _ => unimplemented!("Field access of non-struct value"),
                    }
                }
            }
        }
        StatementKind::DebugPrint(place) => match place {
            Place::Local(var_id) => {
                println!("{}", frame.locals[*var_id]);
            }
            Place::Static(_) => unimplemented!("Debug print of value other than local variable"),
            Place::Field(source_var_id, field_name) => match &frame.locals[*source_var_id] {
                Value::Struct(s) => {
                    println!("{}", s[field_name]);
                }
                _ => unimplemented!("Field access of non-struct value"),
            },
        },
    }
}

pub fn eval_fn(context: &Context, frame: &mut CallFrame, fun: &Function) {
    for block in &fun.basic_blocks {
        for stmt in &block.statements {
            eval_stmt(context, frame, stmt);
        }
    }
}

pub fn eval_context(context: &Context, starting_fn: DefId) {
    match context.definitions[starting_fn] {
        Definition::Fn(ref f) => {
            let mut frame = CallFrame::new();
            let num_temps = f.local_decls.len() - 1 - f.arg_count;
            frame.locals.push(Value::Void); // return value
            for _ in 0..num_temps {
                frame.locals.push(Value::Void);
            }

            eval_fn(context, &mut frame, f);
        }
        _ => unimplemented!("Starting function is not a function definition"),
    }
}
