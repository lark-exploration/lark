#![deny(rust_2018_idioms)]
#![feature(in_band_lifetimes)]
#![feature(box_patterns)]
#![feature(crate_visibility_modifier)]
#![feature(nll)]
#![feature(min_const_fn)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(try_from)]
#![feature(macro_at_most_once_rep)]
#![allow(dead_code)]
#![allow(unused_imports)]

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
use crate::ir::{
    builtin_type, BasicBlock, BinOp, Context, Definition, Function, LocalDecl, Operand, Place,
    Rvalue, StatementKind, TerminatorKind,
};

fn main() {
    let mut c = Context::new();

    let mut bob = Function::new(
        builtin_type::I32,
        vec![
            LocalDecl::new(builtin_type::I32, Some("x".into())),
            LocalDecl::new(builtin_type::I32, Some("y".into())),
        ],
    );

    let bob_tmp = bob.new_temp(builtin_type::I32);

    let mut bb1 = BasicBlock::new();

    bb1.push_stmt(StatementKind::Assign(
        Place::Local(bob_tmp),
        Rvalue::BinaryOp(BinOp::Sub, 1, 2),
    ));
    bb1.push_stmt(StatementKind::Assign(
        Place::Local(0),
        Rvalue::Use(Operand::Move(Place::Local(bob_tmp))),
    ));

    bb1.terminate(TerminatorKind::Return);

    bob.push_block(bb1);

    let bob_def_id = c.add_definition(Definition::Fn(bob));

    // bob.body.push(Command::VarUse(0));
    // bob.body.push(Command::VarUse(1));
    // bob.body.push(Command::Sub);
    // bob.body.push(Command::VarDeclWithInit(2));
    // bob.body.push(Command::VarUse(2));
    // bob.body.push(Command::ReturnLastStackValue);

    let mut m = Function::new(builtin_type::VOID, vec![]);
    let call_result_tmp = m.new_temp(builtin_type::I32);
    let interp_result_tmp = m.new_temp(builtin_type::STRING);

    let mut bb2 = BasicBlock::new();

    bb2.push_stmt(StatementKind::Assign(
        Place::Local(call_result_tmp),
        Rvalue::Use(Operand::Call(
            bob_def_id,
            vec![Operand::ConstantInt(11), Operand::ConstantInt(8)],
        )),
    ));

    bb2.push_stmt(StatementKind::Assign(
        Place::Local(interp_result_tmp),
        Rvalue::Use(Operand::Call(
            101, /*builtin string interp*/
            vec![
                Operand::ConstantString("Hello, world {}".into()),
                Operand::Move(Place::Local(call_result_tmp)),
            ],
        )),
    ));

    bb2.push_stmt(StatementKind::DebugPrint(Rvalue::Use(Operand::Move(
        Place::Local(interp_result_tmp),
    ))));

    // let mut m = Function::new("main".into(), builtin_type::VOID);
    // m.body.push(Command::ConstInt(11));
    // m.body.push(Command::ConstInt(8));
    // m.body.push(Command::Call(bob_def_id));
    // m.body.push(Command::ConstString("Hello, world {}".into()));
    // m.body.push(Command::Call(101)); //built-in string interpolation
    // m.body.push(Command::DebugPrint);

    /*
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
    */

    let mut rust = RustFile::new();

    codegen(&mut rust, &c);
    println!("{}", rust.render());

    let mut eval = Eval::new();
    eval.eval(&c);
}
