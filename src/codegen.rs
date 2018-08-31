use ir::{builtin_type, BuiltinFn, Command, Context, DefId, Definition, Function};

pub struct RustFile {
    output_src: String,
    expression_stack: Vec<String>,
}

impl RustFile {
    pub fn output_raw(&mut self, output: &str) {
        self.output_src += output;
    }

    pub fn delay_expr(&mut self, expr: String) {
        self.expression_stack.push(expr);
    }

    pub fn new() -> RustFile {
        RustFile {
            output_src: String::new(),
            expression_stack: vec![],
        }
    }

    pub fn render(self) -> String {
        self.output_src
    }
}

fn codegen_type(c: &Context, ty: DefId) -> String {
    match ty {
        builtin_type::I32 => "i32".into(),
        builtin_type::VOID => "()".into(),
        _ => unimplemented!("Unsupported type"),
    }
}

pub fn codegen_fn(rust: &mut RustFile, c: &Context, f: &Function) {
    rust.output_raw(&("fn ".to_string() + &f.name + "("));
    let mut after_first = false;
    for param in &f.params {
        if after_first {
            rust.output_raw(", ");
        } else {
            after_first = true;
        }
        rust.output_raw(&param.name);
        rust.output_raw(": ");
        rust.output_raw(&codegen_type(c, param.ty));
    }
    rust.output_raw(") -> ");
    rust.output_raw(&codegen_type(c, f.ret_ty));

    rust.output_raw(" {\n");

    for param in &f.params {
        rust.output_raw(&format!("let v{} = {};\n", param.var_id, param.name));
    }

    for command in &f.body {
        match command {
            Command::VarUse(id) => rust.delay_expr(format!("v{}", id)),
            Command::VarDeclWithInit(id) => {
                let init_expr = rust.expression_stack.pop().unwrap();
                rust.output_raw(&format!("let v{} = {};\n", id, init_expr));
            }
            Command::ConstInt(i) => rust.delay_expr(format!("{}", i)),
            Command::ConstString(s) => rust.delay_expr(format!("\"{}\"", s)),
            Command::Add => {
                let rhs_expr = rust.expression_stack.pop().unwrap();
                let lhs_expr = rust.expression_stack.pop().unwrap();
                rust.delay_expr(format!("({})+({})", lhs_expr, rhs_expr));
            }
            Command::Sub => {
                let rhs_expr = rust.expression_stack.pop().unwrap();
                let lhs_expr = rust.expression_stack.pop().unwrap();
                rust.delay_expr(format!("({})-({})", lhs_expr, rhs_expr));
            }
            Command::Call(def_id, num_args) => {
                if let Definition::Fn(target) = &c.definitions[*def_id] {
                    let mut args_expr = String::new();
                    let mut after_first = false;
                    for _ in 0..*num_args {
                        if after_first {
                            args_expr = ", ".to_string() + &args_expr;
                        } else {
                            after_first = true;
                        }

                        let arg_expr = rust.expression_stack.pop().unwrap();
                        args_expr = arg_expr + &args_expr;
                    }

                    rust.delay_expr(format!("{}({})", target.name, args_expr));
                } else if let Definition::BuiltinFn(builtin_fn) = &c.definitions[*def_id] {
                    match builtin_fn {
                        BuiltinFn::StringInterpolate => {
                            let format_string = rust.expression_stack.pop().unwrap();
                            let mut args_expr = String::new();
                            let mut after_first = false;
                            for _ in 0..*num_args {
                                if after_first {
                                    args_expr = ", ".to_string() + &args_expr;
                                } else {
                                    after_first = true;
                                }

                                let arg_expr = rust.expression_stack.pop().unwrap();
                                args_expr = arg_expr + &args_expr;
                            }
                            rust.delay_expr(format!("format!({}, {})", format_string, args_expr));
                        }
                    }
                } else {
                    unimplemented!("Only calls to functions are currently supported)");
                }
            }
            Command::ReturnLastStackValue => {
                let ret_val = rust.expression_stack.pop().unwrap();
                rust.output_raw(&format!("return {};\n", ret_val));
            }
            Command::DebugPrint => {
                let print_val = rust.expression_stack.pop().unwrap();
                rust.output_raw(&format!("println!(\"{{}}\", {});\n", print_val));
            }
        }
    }
    rust.output_raw("}\n");
}

//FIXME: there are more efficient ways to build strings than this
pub fn codegen(rust: &mut RustFile, c: &Context) {
    for definition in &c.definitions {
        match definition {
            Definition::Fn(f) => {
                codegen_fn(rust, c, f);
            }
            _ => {}
        }
    }
}
