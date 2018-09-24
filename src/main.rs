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
use crate::eval::eval_context;
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
        "bob".into(),
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

    let mut m = Function::new(builtin_type::VOID, vec![], "main".into());
    let call_result_tmp = m.new_temp(builtin_type::I32);
    let interp_result_tmp = m.new_temp(builtin_type::STRING);

    let mut bb2 = BasicBlock::new();

    bb2.push_stmt(StatementKind::Assign(
        Place::Local(call_result_tmp),
        Rvalue::Call(
            bob_def_id,
            vec![Operand::ConstantInt(11), Operand::ConstantInt(8)],
        ),
    ));

    bb2.push_stmt(StatementKind::Assign(
        Place::Local(interp_result_tmp),
        Rvalue::Call(
            101, /*builtin string interp*/
            vec![
                Operand::ConstantString("Hello, world {}".into()),
                Operand::Move(Place::Local(call_result_tmp)),
            ],
        ),
    ));

    bb2.push_stmt(StatementKind::DebugPrint(Place::Local(interp_result_tmp)));

    bb2.terminate(TerminatorKind::Return);
    m.push_block(bb2);
    let main_def_id = c.add_definition(Definition::Fn(m));

    let mut rust = RustFile::new();

    codegen(&mut rust, &c);
    println!("{}", rust.render());

    //let mut eval = Eval::new();
    //eval.eval(&c);

    eval_context(&c, main_def_id);
}
