pub type DefId = usize;
pub type VarId = usize;

// Dummy for now
#[derive(Copy, Clone, Debug)]
pub struct Ty {
    def_id: DefId,
}

#[derive(Debug)]
pub struct SourceInfo;

//Lark MIR representation of a single function
#[derive(Debug)]
pub struct Function {
    pub basic_blocks: Vec<BasicBlock>,

    //First local = return value pointer
    //Followed by arg_count parameters to the function
    //Followed by user defined variables and temporaries
    pub local_decls: Vec<LocalDecl>,

    pub arg_count: usize,

    pub name: String,
}

impl Function {
    pub fn new(return_ty: Ty, mut args: Vec<LocalDecl>, name: String) -> Function {
        let arg_count = args.len();
        let mut local_decls = vec![LocalDecl::new_return_place(return_ty)];
        local_decls.append(&mut args);

        Function {
            basic_blocks: vec![],
            local_decls,
            arg_count,
            name,
        }
    }

    pub fn new_temp(&mut self, ty: Ty) -> VarId {
        self.local_decls.push(LocalDecl::new_temp(ty));
        self.local_decls.len() - 1
    }

    pub fn push_block(&mut self, block: BasicBlock) {
        self.basic_blocks.push(block);
    }
}

#[derive(Debug)]
pub struct Struct {
    pub fields: Vec<Field>,
    pub name: String,
}

impl Struct {
    pub fn field(mut self, name: String, ty: Ty) -> Self {
        self.fields.push(Field { ty, name });
        self
    }

    pub fn new(name: String) -> Self {
        Struct {
            name,
            fields: vec![],
        }
    }
}

#[derive(Debug)]
pub struct Field {
    pub ty: Ty,
    pub name: String,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub terminator: Option<Terminator>,
}

impl BasicBlock {
    pub fn new() -> BasicBlock {
        BasicBlock {
            statements: vec![],
            terminator: None,
        }
    }

    pub fn push_stmt(&mut self, kind: StatementKind) {
        self.statements.push(Statement {
            source_info: SourceInfo,
            kind,
        });
    }

    pub fn terminate(&mut self, terminator_kind: TerminatorKind) {
        self.terminator = Some(Terminator {
            source_info: SourceInfo,
            kind: terminator_kind,
        });
    }
}

#[derive(Debug)]
pub struct Statement {
    pub source_info: SourceInfo,
    pub kind: StatementKind,
}

#[derive(Debug)]
pub enum StatementKind {
    Assign(Place, Rvalue),
    DebugPrint(Place),
}

#[derive(Debug)]
pub struct Terminator {
    pub source_info: SourceInfo,
    pub kind: TerminatorKind,
}

#[derive(Debug)]
pub enum TerminatorKind {
    Return,
}

#[derive(Debug)]
pub enum Place {
    Local(VarId),
    Static(DefId),
    //FIXME: this is a simplifed projection for now
    Field(VarId, String),
}

#[derive(Debug)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinOp, VarId, VarId),
    //FIXME: MIR has this as a Terminator, presumably because stack can unwind
    Call(DefId, Vec<Operand>),
}

#[derive(Debug)]
pub enum Operand {
    Copy(Place),
    Move(Place),
    //FIXME: Move to Box<Constant>
    ConstantInt(i32),
    ConstantString(String),
}

#[derive(Debug)]
pub enum BinOp {
    Add,
    Sub,
}

#[derive(Debug)]
pub struct LocalDecl {
    pub ty: Ty,
    pub name: Option<String>,
}

impl LocalDecl {
    pub fn new_return_place(return_ty: Ty) -> LocalDecl {
        LocalDecl {
            ty: return_ty,
            name: None,
        }
    }

    pub fn new_temp(ty: Ty) -> LocalDecl {
        LocalDecl { ty, name: None }
    }

    pub fn new(ty: Ty, name: Option<String>) -> LocalDecl {
        LocalDecl { ty, name }
    }
}

pub mod builtin_type {
    #[allow(unused)]
    pub const UNKNOWN: usize = 0;
    pub const VOID: usize = 1;
    pub const I32: usize = 2;
    pub const STRING: usize = 3;
    pub const ERROR: usize = 100;
}

#[derive(Debug)]
pub enum BuiltinFn {
    StringInterpolate,
}

#[derive(Debug)]
pub enum Definition {
    Builtin,
    BuiltinFn(BuiltinFn),
    Fn(Function),
    Struct(Struct),
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

    pub fn simple_type_for_def_id(&self, def_id: DefId) -> Ty {
        Ty { def_id: def_id }
    }

    pub fn get_def_id_for_ty(&self, ty: Ty) -> Option<DefId> {
        Some(ty.def_id)
    }
}
