type DefId = usize;
type VarId = usize;

struct Variable {
    ty: DefId,
    name: String,
}

struct Param {
    ty: DefId,
    name: String,
    var_id: VarId,
}

struct Function {
    params: Vec<Param>,
    body: Vec<Command>,
    ret_ty: DefId,
    name: String,
    vars: Vec<Variable>,
}

impl Function {
    pub fn param(mut self, name: String, ty: DefId) -> Self {
        self.vars.push(Variable {
            ty,
            name: name.clone(),
        });
        let var_id = self.vars.len() - 1;
        self.params.push(Param { ty, name, var_id });
        self
    }

    pub fn new(name: String, ret_ty: DefId) -> Function {
        Function {
            params: vec![],
            body: vec![],
            ret_ty,
            name,
            vars: vec![],
        }
    }
}

mod builtin_type {
    pub const UNKNOWN: usize = 0;
    pub const VOID: usize = 1;
    pub const I32: usize = 2;
    pub const ERROR: usize = 100;
}

enum Definition {
    Builtin,
    Fn(Function),
}

enum Command {
    VarUse(VarId),
    Add,
    ReturnLastStackValue,
}

struct Context {
    definitions: Vec<Definition>,
}

impl Context {
    pub fn new() -> Context {
        let mut definitions = vec![];

        for _ in 0..(builtin_type::ERROR + 1) {
            definitions.push(Definition::Builtin); // UNKNOWN
        }

        Context { definitions }
    }
}

struct RustFile {
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
}

fn codegen_type(c: &Context, ty: DefId) -> String {
    match ty {
        builtin_type::I32 => "i32".into(),
        builtin_type::VOID => "()".into(),
        _ => unimplemented!("Unsupported type"),
    }
}

//FIXME: there are more efficient ways to build strings than this
fn codegen(rust: &mut RustFile, c: &Context, f: &Function) {
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

fn main() {
    let mut f = Function::new("bob".into(), builtin_type::I32).param("x".into(), builtin_type::I32);

    f.body.push(Command::VarUse(0));
    f.body.push(Command::VarUse(0));
    f.body.push(Command::Add);
    f.body.push(Command::ReturnLastStackValue);

    let c = Context::new();
    let mut rust = RustFile::new();

    codegen(&mut rust, &c, &f);

    println!("{}", rust.output_src);
}
