use ir::{
    builtin_type, BasicBlock, BinOp, BuiltinFn, Context, Definition, Function, Operand, Place,
    Rvalue, StatementKind, Struct, Terminator, TerminatorKind, Ty, VarId,
};

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

fn build_type(c: &Context, ty: Ty) -> String {
    match c.get_def_id_for_ty(ty) {
        Some(builtin_type::I32) => "i32".into(),
        Some(builtin_type::VOID) => "()".into(),
        Some(builtin_type::STRING) => "String".into(),
        Some(def_id) => match &c.definitions[def_id] {
            Definition::Struct(s) => s.name.clone(),
            _ => unimplemented!("Can't build name for definition"),
        },
        _ => unimplemented!("Can't build name for definition"),
    }
}

fn build_var_name(f: &Function, var_id: VarId) -> String {
    match &f.local_decls[var_id].name {
        Some(n) => n.clone(),
        None => format!("_tmp_{}", var_id,),
    }
}

fn build_operand(f: &Function, operand: &Operand) -> String {
    match operand {
        Operand::ConstantInt(i) => format!("{}", i),
        Operand::ConstantString(s) => format!("\"{}\"", s),
        Operand::Copy(place) | Operand::Move(place) => match place {
            Place::Local(var_id) => format!("{}", build_var_name(f, *var_id)),
            _ => unimplemented!("Copy of non-local value"),
        },
    }
}

fn codegen_block(rust: &mut RustFile, c: &Context, f: &Function, b: &BasicBlock) {
    for stmt in &b.statements {
        match &stmt.kind {
            StatementKind::Assign(lhs, rhs) => {
                match lhs {
                    Place::Local(var_id) => {
                        rust.output_raw(&format!("{} = ", build_var_name(f, *var_id)));
                    }
                    Place::Static(_) => unimplemented!("Assignment into static place"),
                    Place::Field(var_id, field_name) => {
                        rust.output_raw(&format!(
                            "{}.{} = ",
                            build_var_name(f, *var_id),
                            field_name
                        ));
                    }
                };
                match rhs {
                    Rvalue::Use(operand) => rust.output_raw(&build_operand(f, operand)),
                    Rvalue::BinaryOp(bin_op, lhs, rhs) => {
                        let op = match bin_op {
                            BinOp::Add => "+",
                            BinOp::Sub => "-",
                        };

                        rust.output_raw(&format!(
                            "{} {} {}",
                            build_var_name(f, *lhs),
                            op,
                            build_var_name(f, *rhs)
                        ));
                    }
                    Rvalue::Call(def_id, args) => {
                        let mut processed_args = vec![];
                        for arg in args {
                            processed_args.push(build_operand(f, arg));
                        }
                        match &c.definitions[*def_id] {
                            Definition::Fn(f) => {
                                rust.output_raw(&format!("{}(", f.name));
                                let mut first = true;
                                for processed_arg in processed_args {
                                    if !first {
                                        rust.output_raw(", ");
                                    } else {
                                        first = false;
                                    }
                                    rust.output_raw(&processed_arg);
                                }
                                rust.output_raw(")");
                            }
                            Definition::BuiltinFn(builtin_fn) => match builtin_fn {
                                BuiltinFn::StringInterpolate => {
                                    rust.output_raw("format!(");
                                    let mut first = true;
                                    for processed_arg in processed_args {
                                        if !first {
                                            rust.output_raw(", ");
                                        } else {
                                            first = false;
                                        }
                                        rust.output_raw(&processed_arg);
                                    }
                                    rust.output_raw(")");
                                }
                            },
                            Definition::Struct(s) => {
                                rust.output_raw(&format!("{} {{", s.name));
                                for i in 0..s.fields.len() {
                                    rust.output_raw(&s.fields[i].name);
                                    rust.output_raw(": ");
                                    rust.output_raw(&processed_args[i]);
                                    rust.output_raw(", ");
                                }
                                rust.output_raw("}");
                            }
                            _ => {}
                        }
                    }
                }
                rust.output_raw(";\n");
            }
            StatementKind::DebugPrint(place) => match place {
                Place::Local(var_id) => {
                    rust.output_raw(&format!(
                        "println!(\"{{:?}}\", {});\n",
                        build_var_name(f, *var_id)
                    ));
                }
                Place::Static(_) => unimplemented!("Debug print of value that is not a local"),
                Place::Field(var_id, field_name) => {
                    rust.output_raw(&format!(
                        "println!(\"{{:?}}\", {}.{});\n",
                        build_var_name(f, *var_id),
                        field_name
                    ));
                }
            },
        }
    }
    match b.terminator {
        Some(Terminator {
            kind: TerminatorKind::Return,
            ..
        }) => match c.get_def_id_for_ty(f.local_decls[0].ty) {
            Some(builtin_type::VOID) => rust.output_raw("return;\n"),
            _ => rust.output_raw(&format!("return {};\n", build_var_name(f, 0))),
        },
        None => {}
    }
}

fn codegen_fn(rust: &mut RustFile, c: &Context, f: &Function) {
    rust.output_raw(&("fn ".to_string() + &f.name + "("));
    let mut after_first = false;
    for param in f.local_decls.iter().skip(1).take(f.arg_count) {
        if after_first {
            rust.output_raw(", ");
        } else {
            after_first = true;
        }
        rust.output_raw(&param.name.clone().unwrap());
        rust.output_raw(": ");
        rust.output_raw(&build_type(c, param.ty));
    }
    rust.output_raw(") -> ");
    rust.output_raw(&build_type(c, f.local_decls[0].ty));

    rust.output_raw(" {\n");

    for (idx, local_decl) in f.local_decls.iter().enumerate().skip(1 + f.arg_count) {
        rust.output_raw(&format!(
            "let {}: {};\n",
            build_var_name(f, idx),
            build_type(c, local_decl.ty)
        ));
    }

    match c.get_def_id_for_ty(f.local_decls[0].ty) {
        Some(builtin_type::VOID) => {}
        _ => {
            rust.output_raw(&format!(
                "let {}: {};\n",
                build_var_name(f, 0),
                build_type(c, f.local_decls[0].ty)
            ));
        }
    }

    for block in &f.basic_blocks {
        codegen_block(rust, c, f, block);
    }

    rust.output_raw("}\n");
}

fn codegen_struct(rust: &mut RustFile, c: &Context, s: &Struct) {
    rust.output_raw(&format!("#[derive(Debug)]\n"));
    rust.output_raw(&format!("struct {} {{\n", s.name));
    for field in &s.fields {
        rust.output_raw(&format!("{}: {},\n", field.name, build_type(c, field.ty)));
    }
    rust.output_raw("}\n");
}

pub fn codegen(rust: &mut RustFile, c: &Context) {
    for definition in &c.definitions {
        match definition {
            Definition::Fn(f) => {
                codegen_fn(rust, c, f);
            }
            Definition::Struct(s) => {
                codegen_struct(rust, c, s);
            }
            _ => {}
        }
    }
}

// use crate::ir::{builtin_type, BuiltinFn, Command, Context, DefId, Definition, Function, Struct};

// pub struct RustFile {
//     output_src: String,
//     expression_stack: Vec<String>,
// }

// impl RustFile {
//     pub fn output_raw(&mut self, output: &str) {
//         self.output_src += output;
//     }

//     pub fn delay_expr(&mut self, expr: String) {
//         self.expression_stack.push(expr);
//     }

//     pub fn new() -> RustFile {
//         RustFile {
//             output_src: String::new(),
//             expression_stack: vec![],
//         }
//     }

//     pub fn render(self) -> String {
//         self.output_src
//     }
// }

// fn build_type(c: &Context, ty: DefId) -> String {
//     match ty {
//         builtin_type::I32 => "i32".into(),
//         builtin_type::VOID => "()".into(),
//         builtin_type::STRING => "String".into(),
//         _ => {
//             let definition = &c.definitions[ty];
//             match definition {
//                 Definition::Borrow(builtin_type::STRING) => "&str".into(),
//                 Definition::Borrow(x) => format!("&{}", build_type(c, *x)),
//                 _ => unimplemented!("Cannot codegen type"),
//             }
//         }
//     }
// }

// pub fn codegen_fn(rust: &mut RustFile, c: &Context, f: &Function) {
//     rust.output_raw(&("fn ".to_string() + &f.name + "("));
//     let mut after_first = false;
//     for param in &f.params {
//         if after_first {
//             rust.output_raw(", ");
//         } else {
//             after_first = true;
//         }
//         rust.output_raw(&param.name);
//         rust.output_raw(": ");
//         rust.output_raw(&build_type(c, param.ty));
//     }
//     rust.output_raw(") -> ");
//     rust.output_raw(&build_type(c, f.ret_ty));

//     rust.output_raw(" {\n");

//     for param in &f.params {
//         rust.output_raw(&format!("let v{} = {};\n", param.var_id, param.name));
//     }

//     for command in &f.body {
//         match command {
//             Command::VarUse(id) => rust.delay_expr(format!("v{}", id)),
//             Command::VarDeclWithInit(id) => {
//                 let init_expr = rust.expression_stack.pop().unwrap();
//                 rust.output_raw(&format!("let v{} = {};\n", id, init_expr));
//             }
//             Command::ConstInt(i) => rust.delay_expr(format!("{}", i)),
//             Command::ConstString(s) => rust.delay_expr(format!("\"{}\"", s)),
//             Command::Add => {
//                 let rhs_expr = rust.expression_stack.pop().unwrap();
//                 let lhs_expr = rust.expression_stack.pop().unwrap();
//                 rust.delay_expr(format!("({})+({})", lhs_expr, rhs_expr));
//             }
//             Command::Sub => {
//                 let rhs_expr = rust.expression_stack.pop().unwrap();
//                 let lhs_expr = rust.expression_stack.pop().unwrap();
//                 rust.delay_expr(format!("({})-({})", lhs_expr, rhs_expr));
//             }
//             Command::Dot(rhs_field) => {
//                 let lhs_expr = rust.expression_stack.pop().unwrap();
//                 rust.delay_expr(format!("({}).{}", lhs_expr, rhs_field));
//             }
//             Command::Call(def_id) => {
//                 if let Definition::Fn(target) = &c.definitions[*def_id] {
//                     let mut args_expr = String::new();
//                     let mut after_first = false;
//                     for _ in 0..target.params.len() {
//                         if after_first {
//                             args_expr = ", ".to_string() + &args_expr;
//                         } else {
//                             after_first = true;
//                         }

//                         let arg_expr = rust.expression_stack.pop().unwrap();
//                         args_expr = arg_expr + &args_expr;
//                     }

//                     rust.delay_expr(format!("{}({})", target.name, args_expr));
//                 } else if let Definition::BuiltinFn(builtin_fn) = &c.definitions[*def_id] {
//                     match builtin_fn {
//                         BuiltinFn::StringInterpolate => {
//                             let format_string = rust.expression_stack.pop().unwrap();

//                             let num_args = format_string.matches("{}").count();

//                             let mut args_expr = String::new();
//                             let mut after_first = false;
//                             for _ in 0..num_args {
//                                 if after_first {
//                                     args_expr = ", ".to_string() + &args_expr;
//                                 } else {
//                                     after_first = true;
//                                 }

//                                 let arg_expr = rust.expression_stack.pop().unwrap();
//                                 args_expr = arg_expr + &args_expr;
//                             }
//                             rust.delay_expr(format!("format!({}, {})", format_string, args_expr));
//                         }
//                     }
//                 } else if let Definition::Struct(s) = &c.definitions[*def_id] {
//                     let mut field_values = vec![];
//                     for _ in 0..s.fields.len() {
//                         field_values.push(rust.expression_stack.pop().unwrap());
//                     }

//                     let mut struct_expr = String::new();

//                     struct_expr += &format!("{} {{", s.name);
//                     for field in &s.fields {
//                         struct_expr += &format!("{}: {},", field.name, field_values.pop().unwrap());
//                     }
//                     struct_expr += "} ";
//                     rust.delay_expr(struct_expr);
//                 } else {
//                     unimplemented!("Only calls to functions are currently supported)");
//                 }
//             }
//             Command::ReturnLastStackValue => {
//                 let ret_val = rust.expression_stack.pop().unwrap();
//                 rust.output_raw(&format!("return {};\n", ret_val));
//             }
//             Command::DebugPrint => {
//                 let print_val = rust.expression_stack.pop().unwrap();
//                 rust.output_raw(&format!("println!(\"{{:?}}\", {});\n", print_val));
//             }
//         }
//     }

//     if rust.expression_stack.len() > 0 {
//         let final_expr = rust.expression_stack.pop().unwrap();
//         rust.output_raw(&format!("{};\n", final_expr));
//     }

//     assert_eq!(rust.expression_stack.len(), 0);
//     rust.output_raw("}\n");
// }

// fn codegen_struct(rust: &mut RustFile, c: &Context, s: &Struct) {
//     rust.output_raw(&format!("#[derive(Debug)]\n"));
//     rust.output_raw(&format!("struct {} {{\n", s.name));
//     for field in &s.fields {
//         rust.output_raw(&format!("{}: {},\n", field.name, build_type(c, field.ty)));
//     }
//     rust.output_raw("}\n");
// }

// //FIXME: there are more efficient ways to build strings than this
// pub fn codegen(rust: &mut RustFile, c: &Context) {
//     for definition in &c.definitions {
//         match definition {
//             Definition::Fn(f) => {
//                 codegen_fn(rust, c, f);
//             }
//             Definition::Struct(s) => {
//                 codegen_struct(rust, c, s);
//             }
//             _ => {}
//         }
//     }
// }
