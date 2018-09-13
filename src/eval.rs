use crate::ir::{builtin_type, BuiltinFn, Command, Context, DefId, Definition, Function, Struct};
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

pub struct Eval {
    pub stack: Vec<Value>,
}

impl Eval {
    pub fn new() -> Eval {
        Eval { stack: vec![] }
    }

    pub fn eval_block(
        &mut self,
        c: &Context,
        vars: &mut HashMap<usize, usize>,
        commands: &Vec<Command>,
    ) -> Value {
        for command in commands {
            match command {
                Command::ConstInt(i) => self.stack.push(Value::I32(*i)),
                Command::ConstString(s) => self.stack.push(Value::Str(s.clone())),
                Command::Add => {
                    let rhs = self.stack.pop().unwrap();
                    let lhs = self.stack.pop().unwrap();
                    match (lhs, rhs) {
                        (Value::I32(l), Value::I32(r)) => {
                            self.stack.push(Value::I32(l + r));
                        }
                        _ => unimplemented!("Unsupported add of non-integers"),
                    }
                }
                Command::Sub => {
                    let rhs = self.stack.pop().unwrap();
                    let lhs = self.stack.pop().unwrap();
                    match (lhs, rhs) {
                        (Value::I32(l), Value::I32(r)) => {
                            self.stack.push(Value::I32(l - r));
                        }
                        _ => unimplemented!("Unsupported subtract of non-integers"),
                    }
                }
                Command::DebugPrint => {
                    let arg = self.stack.pop().unwrap();

                    println!("{}", arg);
                }
                Command::VarUse(var_id) => {
                    let stack_pos = vars[var_id];
                    let var_use = self.stack[stack_pos].clone();
                    self.stack.push(var_use);
                }
                Command::VarDeclWithInit(var_id) => {
                    vars.insert(*var_id, self.stack.len() - 1);
                }
                Command::Call(def_id) => match &c.definitions[*def_id] {
                    Definition::Fn(ref f) => {
                        let result = self.eval_fn(c, f);
                        self.stack.push(result);
                    }
                    Definition::Struct(ref s) => {
                        let mut new_obj = HashMap::new();
                        for field in s.fields.iter().rev() {
                            let val = self.stack.pop().unwrap();
                            new_obj.insert(field.name.clone(), val);
                        }
                        self.stack.push(Value::Struct(new_obj));
                    }
                    Definition::BuiltinFn(BuiltinFn::StringInterpolate) => {
                        let format_string = self.stack.pop().unwrap();

                        match format_string {
                            Value::Str(s) => {
                                let sections: Vec<&str> = s.split("{}").collect();
                                let sections_len = sections.len();

                                let mut result = String::new();
                                let mut pos = 0;

                                for i in 0..(sections_len - 1) {
                                    result += sections[i];
                                    result += &format!(
                                        "{}",
                                        self.stack[self.stack.len() - sections_len + i]
                                    );
                                }

                                result += sections[sections_len - 1];

                                for _ in 0..sections_len {
                                    self.stack.pop();
                                }

                                self.stack.push(Value::Str(result));
                            }
                            _ => unimplemented!("String interpolation without a format string"),
                        }
                    }
                    x => unimplemented!("Call of a non-function: {:#?}", x),
                },
                Command::ReturnLastStackValue => {
                    let result = self.stack.pop().unwrap();
                    return result;
                }
                Command::Dot(field) => {
                    let lhs = self.stack.pop().unwrap();
                    match lhs {
                        Value::Struct(s) => {
                            self.stack.push(s[field].clone());
                        }
                        _ => unimplemented!("Dot into non-struct value")
                    }
                }
                //_ => unimplemented!("Incomplete eval of commands in eval_fn"),
            }
        }

        Value::Void
    }

    pub fn eval_fn(&mut self, c: &Context, f: &Function) -> Value {
        let mut vars = HashMap::new();

        //Rather than popping the values only to push them back on so the
        //function body can see it, let's instead capture where the arguments are in the stack
        //by setting up the function's variables ahead of processing the body
        let mut offset = 0;
        let param_len = f.params.len();
        for param in &f.params {
            vars.insert(param.var_id, self.stack.len() - param_len + offset);
            offset += 1;
        }

        self.eval_block(c, &mut vars, &f.body)
    }

    pub fn eval(&mut self, c: &Context) {
        for definition in &c.definitions {
            match definition {
                Definition::Fn(f) => {
                    if f.name == "main" {
                        self.eval_fn(c, f);
                    }
                }
                _ => {}
            }
        }
    }
}
