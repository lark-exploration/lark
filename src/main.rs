type DefId = usize;
type VarId = usize;

struct Variable {
    ty: DefId,
    name: String,
    var_id: VarId,
}

struct Function {
    params: Vec<Variable>,
    body: Vec<Command>,
    ret_ty: DefId,
    name: String,
    vars: Vec<Variable>,
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

fn codegen_vardecl(c: &Context, v: &Variable) -> String {
    let mut output = String::new();

    output += &(v.name.clone() + ": " + &codegen_type(c, v.ty));

    output
}

//FIXME: there are more efficient ways to build strings than this
fn codegen(rust: &mut RustFile, c: &Context, f: &Function) {
    rust.output_raw(&("fn ".to_string() + &f.name + "("));
    for param in &f.params {
        rust.output_raw(&codegen_vardecl(c, &param));
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
    let f = Function {
        name: "bob".to_string(),
        params: vec![Variable {
            name: "foo".into(),
            ty: builtin_type::I32,
            var_id: 0,
        }],
        ret_ty: builtin_type::I32,
        vars: vec![],
        body: vec![
            Command::VarUse(0),
            Command::VarUse(0),
            Command::Add,
            Command::ReturnLastStackValue,
        ],
    };

    let c = Context::new();
    let mut rust = RustFile::new();

    codegen(&mut rust, &c, &f);

    println!("{}", rust.output_src);
}
