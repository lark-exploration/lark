#![deny(rust_2018_idioms)]
#![feature(in_band_lifetimes)]
#![feature(box_patterns)]
#![feature(crate_visibility_modifier)]
#![feature(nll)]
#![feature(min_const_fn)]
#![feature(try_from)]
#![allow(unused)]

#[macro_use]
mod lexer;

#[macro_use]
mod indices;

mod codegen;
mod eval;
mod ir;
mod parser;
mod ty;

use crate::codegen::{codegen, RustFile};
use crate::eval::Eval;
use crate::ir::{builtin_type, Command, Context, Definition, Function, Struct};

fn main() {
    let mut c = Context::new();

    let mut bob = Function::new("bob".into(), builtin_type::I32)
        .param("x".into(), builtin_type::I32)
        .param("y".into(), builtin_type::I32);

    bob.body.push(Command::VarUse(0));
    bob.body.push(Command::VarUse(1));
    bob.body.push(Command::Sub);
    bob.body.push(Command::VarDeclWithInit(2));
    bob.body.push(Command::VarUse(2));
    bob.body.push(Command::ReturnLastStackValue);

    let bob_def_id = c.add_definition(Definition::Fn(bob));

    let person = Struct::new("Person".into())
        .field("height".into(), builtin_type::I32)
        .field("id".into(), builtin_type::I32);

    let person_def_id = c.add_definition(Definition::Struct(person));

    let mut m = Function::new("main".into(), builtin_type::VOID);
    m.body.push(Command::ConstInt(11));
    m.body.push(Command::ConstInt(8));
    m.body.push(Command::Call(bob_def_id));
    m.body.push(Command::ConstString("Hello, world {}".into()));
    m.body.push(Command::Call(101)); //built-in string interpolation
    m.body.push(Command::DebugPrint);

    m.body.push(Command::ConstInt(17));
    m.body.push(Command::ConstInt(18));
    m.body.push(Command::Call(person_def_id));

    m.body.push(Command::VarDeclWithInit(0));
    m.body.push(Command::VarUse(0));
    m.body.push(Command::DebugPrint);
    m.body.push(Command::VarUse(0));
    m.body.push(Command::Dot("id".into()));
    m.body.push(Command::DebugPrint);

    c.definitions.push(Definition::Fn(m));

    let mut rust = RustFile::new();

    codegen(&mut rust, &c);
    println!("{}", rust.render());

    let mut eval = Eval::new();
    eval.eval(&c);
}
