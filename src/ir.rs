pub type DefId = usize;
pub type VarId = usize;

pub struct Variable {
    pub ty: DefId,
    pub name: String,
}

pub struct Param {
    pub ty: DefId,
    pub name: String,
    pub var_id: VarId,
}

pub struct Function {
    pub params: Vec<Param>,
    pub body: Vec<Command>,
    pub ret_ty: DefId,
    pub name: String,
    pub vars: Vec<Variable>,
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

pub mod builtin_type {
    pub const UNKNOWN: usize = 0;
    pub const VOID: usize = 1;
    pub const I32: usize = 2;
    pub const STRING: usize = 3;
    pub const ERROR: usize = 100;
}

pub enum BuiltinFn {
    StringInterpolate,
}

pub enum Definition {
    Builtin,
    BuiltinFn(BuiltinFn),
    Fn(Function),
    Borrow(DefId),
    Move(DefId),
}

pub enum Command {
    VarUse(VarId),
    VarDeclWithInit(VarId),
    ConstInt(i32),
    ConstString(String),
    Call(DefId, usize), //(target, num_args)
    Borrow,
    Move,
    Add,
    Sub,
    ReturnLastStackValue,
    DebugPrint,
}

pub struct Context {
    pub definitions: Vec<Definition>,
}

impl Context {
    pub fn new() -> Context {
        let mut definitions = vec![];

        for _ in 0..(builtin_type::ERROR + 1) {
            definitions.push(Definition::Builtin); // UNKNOWN
        }

        definitions.push(Definition::BuiltinFn(BuiltinFn::StringInterpolate));

        Context { definitions }
    }

    pub fn add_definition(&mut self, def: Definition) -> usize {
        self.definitions.push(def);
        self.definitions.len() - 1
    }
}
