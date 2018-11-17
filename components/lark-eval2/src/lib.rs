use ast::AstDatabase;
use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData, ItemKind, LangItem};
use lark_error::{Diagnostic, WithError};
use lark_hir::HirDatabase;
use lark_mir2::{
    BasicBlock, FnBytecode, MirDatabase, Operand, OperandData, Rvalue, RvalueData, Statement,
    StatementKind,
};
use lark_query_system::LarkDatabase;
use lark_ty::Ty;
use parser::ReaderDatabase;
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

#[derive(Debug)]
pub struct CallFrame {
    locals: Vec<Value>,
}

impl CallFrame {
    fn new() -> CallFrame {
        CallFrame { locals: vec![] }
    }
}

pub struct IOHandler {
    pub redirect: Option<String>,
}

impl IOHandler {
    pub fn new(redirect_output: bool) -> IOHandler {
        if redirect_output {
            IOHandler {
                redirect: Some(String::new()),
            }
        } else {
            IOHandler { redirect: None }
        }
    }

    pub fn println(&mut self, output: String) {
        if let Some(redirect_output) = &mut self.redirect {
            redirect_output.push_str(&output);
            redirect_output.push_str("\n");
        } else {
            println!("{}", output);
        }
    }
}

pub fn eval_statement(
    db: &mut LarkDatabase,
    fn_bytecode: &FnBytecode,
    statement: Statement,
    io_handler: &mut IOHandler,
) {
    let statement_data = &fn_bytecode.tables[statement];

    /*
    match &statement_data.kind {
        StatementKind::Expression(rvalue) => {}
    }
    */
}

pub fn eval_basic_block(
    db: &mut LarkDatabase,
    fn_bytecode: &FnBytecode,
    basic_block: BasicBlock,
    io_handler: &mut IOHandler,
) {
    let basic_block_data = &fn_bytecode.tables[basic_block];

    for statement in basic_block_data.statements.iter(&fn_bytecode) {
        eval_statement(db, fn_bytecode, statement, io_handler);
    }
}

pub fn eval_function(db: &mut LarkDatabase, fn_bytecode: &FnBytecode, io_handler: &mut IOHandler) {
    for basic_block in fn_bytecode.basic_blocks.iter(fn_bytecode) {
        eval_basic_block(db, fn_bytecode, basic_block, io_handler);
    }
}

pub fn eval(db: &mut LarkDatabase, io_handler: &mut IOHandler) {
    let input_files = db.paths();
    let mut errors: Vec<Diagnostic> = vec![];

    for &input_file in &*input_files {
        let entities = db.items_in_file(input_file);

        for &entity in &*entities {
            match entity.untern(&db) {
                EntityData::ItemName {
                    kind: ItemKind::Function,
                    id,
                    ..
                } => {
                    let main_name = "main".intern(&db);

                    if id == main_name {
                        let bytecode = db.fn_bytecode(entity);
                        println!("Found main!");
                        eval_function(db, &bytecode.value, io_handler);
                    }
                }
                _ => {}
            }
        }
    }
}
