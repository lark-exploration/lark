use lark_mir::{
    builtin_type, BasicBlock, BinOp, Context, Definition, Function, Operand, Place, Rvalue,
    StatementKind, Struct, Terminator, TerminatorKind, Ty, VarId,
};

pub struct RustFile {
    output_src: String,
}

impl RustFile {
    pub fn output_raw(&mut self, output: &str) {
        self.output_src += output;
    }

    pub fn new() -> RustFile {
        RustFile {
            output_src: String::new(),
        }
    }

    pub fn render(self) -> String {
        self.output_src
    }
}

fn build_type(context: &Context, ty: Ty) -> String {
    match context.get_def_id_for_ty(ty) {
        Some(builtin_type::I32) => "i32".into(),
        Some(builtin_type::VOID) => "()".into(),
        Some(builtin_type::STRING) => "String".into(),
        Some(def_id) => match &context.definitions[def_id] {
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

fn codegen_block(rust: &mut RustFile, context: &Context, f: &Function, b: &BasicBlock) {
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
                        match &context.definitions[*def_id] {
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
        }) => match context.get_def_id_for_ty(f.local_decls[0].ty) {
            Some(builtin_type::VOID) => rust.output_raw("return;\n"),
            _ => rust.output_raw(&format!("return {};\n", build_var_name(f, 0))),
        },
        None => {}
    }
}

fn codegen_fn(rust: &mut RustFile, context: &Context, f: &Function) {
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
        rust.output_raw(&build_type(context, param.ty));
    }
    rust.output_raw(") -> ");
    rust.output_raw(&build_type(context, f.local_decls[0].ty));

    rust.output_raw(" {\n");

    for (idx, local_decl) in f.local_decls.iter().enumerate().skip(1 + f.arg_count) {
        rust.output_raw(&format!(
            "let {}: {};\n",
            build_var_name(f, idx),
            build_type(context, local_decl.ty)
        ));
    }

    match context.get_def_id_for_ty(f.local_decls[0].ty) {
        Some(builtin_type::VOID) => {}
        _ => {
            rust.output_raw(&format!(
                "let {}: {};\n",
                build_var_name(f, 0),
                build_type(context, f.local_decls[0].ty)
            ));
        }
    }

    for block in &f.basic_blocks {
        codegen_block(rust, context, f, block);
    }

    rust.output_raw("}\n");
}

fn codegen_struct(rust: &mut RustFile, context: &Context, s: &Struct) {
    rust.output_raw(&format!("#[derive(Debug)]\n"));
    rust.output_raw(&format!("struct {} {{\n", s.name));
    for field in &s.fields {
        rust.output_raw(&format!(
            "{}: {},\n",
            field.name,
            build_type(context, field.ty)
        ));
    }
    rust.output_raw("}\n");
}

/// Converts the MIR context of definitions into Rust source
pub fn codegen_rust(context: &Context) -> String {
    let mut rust_file = RustFile::new();
    for definition in &context.definitions {
        match definition {
            Definition::Fn(f) => {
                codegen_fn(&mut rust_file, context, f);
            }
            Definition::Struct(s) => {
                codegen_struct(&mut rust_file, context, s);
            }
            _ => {}
        }
    }

    rust_file.render()
}
