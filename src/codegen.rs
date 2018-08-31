use ir::{builtin_type, Command, Context, DefId, Function};

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

//FIXME: there are more efficient ways to build strings than this
pub fn codegen(rust: &mut RustFile, c: &Context, f: &Function) {
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
            Command::Add => {
                let rhs_expr = rust.expression_stack.pop().unwrap();
                let lhs_expr = rust.expression_stack.pop().unwrap();
                rust.delay_expr(format!("({}+{})", lhs_expr, rhs_expr));
            }
            Command::ReturnLastStackValue => {
                let ret_val = rust.expression_stack.pop().unwrap();
                rust.output_raw(&format!("return {};\n", ret_val));
            }
        }
    }
    rust.output_raw("}\n");
}
