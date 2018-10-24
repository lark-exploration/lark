use lark_mir::{
    builtin_type, BasicBlock, BinOp, Context, Definition, Function, Operand, Place, Rvalue,
    StatementKind, Struct, Terminator, TerminatorKind, Ty, VarId,
};

pub struct CFile {
    output_src: String,
}

impl CFile {
    pub fn output_raw(&mut self, output: &str) {
        self.output_src += output;
    }

    pub fn new() -> CFile {
        CFile {
            output_src: String::new(),
        }
    }

    pub fn render(self) -> String {
        self.output_src
    }
}

fn build_type(context: &Context, ty: Ty) -> String {
    match context.get_def_id_for_ty(ty) {
        Some(builtin_type::I32) => "int".into(),
        Some(builtin_type::VOID) => "void".into(),
        Some(builtin_type::STRING) => "char*".into(),
        Some(def_id) => match &context.definitions[def_id] {
            Definition::Struct(s) => format!("struct struct_{}", s.name),
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

fn build_print_based_on_type(context: &Context, name: &str, ty: Ty) -> String {
    match context.get_def_id_for_ty(ty) {
        Some(builtin_type::I32) => format!("printf(\"%i\\n\", {});\n", name),
        Some(builtin_type::STRING) => format!("printf(\"%s\\n\", {});\n", name),
        _ => unimplemented!("Unsupported type for debug print in C: {:?}", ty),
    }
}

fn codegen_block(c_file: &mut CFile, context: &Context, f: &Function, b: &BasicBlock) {
    for stmt in &b.statements {
        match &stmt.kind {
            StatementKind::Assign(lhs, rhs) => {
                match lhs {
                    Place::Local(var_id) => {
                        c_file.output_raw(&format!("{} = ", build_var_name(f, *var_id)));
                    }
                    Place::Static(_) => unimplemented!("Assignment into static place"),
                    Place::Field(var_id, field_name) => {
                        c_file.output_raw(&format!(
                            "{}.{} = ",
                            build_var_name(f, *var_id),
                            field_name
                        ));
                    }
                };
                match rhs {
                    Rvalue::Use(operand) => {
                        let operand_name = build_operand(f, operand);
                        c_file.output_raw(&operand_name)
                    }
                    Rvalue::BinaryOp(bin_op, lhs, rhs) => {
                        let op = match bin_op {
                            BinOp::Add => "+",
                            BinOp::Sub => "-",
                        };

                        c_file.output_raw(&format!(
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
                                c_file.output_raw(&format!("{}(", f.name));
                                let mut first = true;
                                for processed_arg in processed_args {
                                    if !first {
                                        c_file.output_raw(", ");
                                    } else {
                                        first = false;
                                    }
                                    c_file.output_raw(&processed_arg);
                                }
                                c_file.output_raw(")");
                            }
                            Definition::Struct(s) => {
                                c_file.output_raw(&format!("_init_struct_{}(", s.name));
                                let mut first = true;
                                for processed_arg in processed_args {
                                    if !first {
                                        c_file.output_raw(", ");
                                    } else {
                                        first = false;
                                    }
                                    c_file.output_raw(&processed_arg);
                                }
                                c_file.output_raw(")");
                            }
                            _ => {}
                        }
                    }
                }
                c_file.output_raw(";\n");
            }
            StatementKind::DebugPrint(place) => match place {
                Place::Local(var_id) => {
                    let var_name = build_var_name(f, *var_id);
                    c_file.output_raw(&build_print_based_on_type(
                        context,
                        &var_name,
                        f.local_decls[*var_id].ty,
                    ));
                }
                Place::Field(var_id, field_name) => {
                    match context.get_def_id_for_ty(f.local_decls[*var_id].ty) {
                        Some(def_id) => match &context.definitions[def_id] {
                            lark_mir::Definition::Struct(s) => {
                                let mut found = false;
                                for field in &s.fields {
                                    if &field.name == field_name {
                                        c_file.output_raw(&build_print_based_on_type(
                                            context,
                                            &format!(
                                                "{}.{}",
                                                build_var_name(f, *var_id),
                                                field_name
                                            ),
                                            field.ty,
                                        ));
                                        found = true;
                                        break;
                                    }
                                }
                                if !found {
                                    panic!("Can not find matching field for {}", field_name);
                                }
                            }
                            _ => panic!("Field access on non-struct"),
                        },
                        None => panic!("Can't find struct for field access"),
                    }
                }
                _ => unimplemented!("Can't output: {:?}", place),
            },
        }
    }
    match b.terminator {
        Some(Terminator {
            kind: TerminatorKind::Return,
            ..
        }) => match context.get_def_id_for_ty(f.local_decls[0].ty) {
            Some(builtin_type::VOID) => c_file.output_raw("return;\n"),
            _ => c_file.output_raw(&format!("return({});\n", build_var_name(f, 0))),
        },
        None => {}
    }
}

fn codegen_fn_predecl(c_file: &mut CFile, context: &Context, f: &Function) {
    c_file.output_raw(&format!(
        "{} {} (",
        build_type(context, f.local_decls[0].ty),
        f.name
    ));
    let mut after_first = false;
    for param in f.local_decls.iter().skip(1).take(f.arg_count) {
        if after_first {
            c_file.output_raw(", ");
        } else {
            after_first = true;
        }
        c_file.output_raw(&build_type(context, param.ty));
        c_file.output_raw(" ");
        c_file.output_raw(&param.name.clone().unwrap());
    }
    c_file.output_raw(");\n");
}

fn codegen_struct_predecl(c_file: &mut CFile, _context: &Context, s: &Struct) {
    c_file.output_raw(&format!("struct struct_{};\n", s.name));
    c_file.output_raw(&format!(
        "struct struct_{} _init_struct_{}();\n",
        s.name, s.name
    ));
}

fn codegen_fn(c_file: &mut CFile, context: &Context, f: &Function) {
    c_file.output_raw(&format!(
        "{} {} (",
        build_type(context, f.local_decls[0].ty),
        f.name
    ));
    let mut after_first = false;
    for param in f.local_decls.iter().skip(1).take(f.arg_count) {
        if after_first {
            c_file.output_raw(", ");
        } else {
            after_first = true;
        }
        c_file.output_raw(&build_type(context, param.ty));
        c_file.output_raw(" ");
        c_file.output_raw(&param.name.clone().unwrap());
    }
    c_file.output_raw(")");

    c_file.output_raw("\n{\n");
    for (idx, local_decl) in f.local_decls.iter().enumerate().skip(1 + f.arg_count) {
        c_file.output_raw(&format!(
            "{} {};\n",
            build_type(context, local_decl.ty),
            build_var_name(f, idx),
        ));
    }

    match context.get_def_id_for_ty(f.local_decls[0].ty) {
        Some(builtin_type::VOID) => {}
        _ => {
            c_file.output_raw(&format!(
                "{} {};\n",
                build_type(context, f.local_decls[0].ty),
                build_var_name(f, 0),
            ));
        }
    }

    for block in &f.basic_blocks {
        codegen_block(c_file, context, f, block);
    }

    c_file.output_raw("}\n");
}

fn codegen_struct(c_file: &mut CFile, context: &Context, s: &Struct) {
    c_file.output_raw(&format!("struct struct_{} {{\n", s.name));
    for field in &s.fields {
        c_file.output_raw(&format!(
            "{} {};\n",
            build_type(context, field.ty),
            field.name,
        ));
    }
    c_file.output_raw("};\n");
    c_file.output_raw(&format!(
        "struct struct_{} _init_struct_{}(",
        s.name, s.name
    ));
    let mut first = true;
    for field in &s.fields {
        if !first {
            c_file.output_raw(", ");
        } else {
            first = false;
        }
        c_file.output_raw(&format!("{} {}", build_type(context, field.ty), field.name,));
    }
    c_file.output_raw(")\n{\n");
    c_file.output_raw(&format!("struct struct_{} temp = {{", s.name));
    let mut first = true;
    for field in &s.fields {
        c_file.output_raw(&format!("{}{}", if !first { ", " } else { "" }, field.name,));
        first = false;
    }
    c_file.output_raw("};\n");
    c_file.output_raw("return temp;\n");
    c_file.output_raw("}\n");
}

pub fn codegen_c(context: &Context) -> String {
    let mut c_file = CFile::new();

    for definition in &context.definitions {
        match definition {
            Definition::Fn(f) => {
                codegen_fn_predecl(&mut c_file, context, f);
            }
            Definition::Struct(s) => {
                codegen_struct_predecl(&mut c_file, context, s);
            }
            _ => {}
        }
    }
    for definition in &context.definitions {
        match definition {
            Definition::Fn(f) => {
                codegen_fn(&mut c_file, context, f);
            }
            Definition::Struct(s) => {
                codegen_struct(&mut c_file, context, s);
            }
            _ => {}
        }
    }

    c_file.render()
}
