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
    Rvalue, StatementKind, Struct, TerminatorKind,
};

use crate::ty::intern::TyInterners;

fn main() {
    let mut c = Context::new();

    let i32_ty = c.simple_type_for_def_id(builtin_type::I32);
    let void_ty = c.simple_type_for_def_id(builtin_type::VOID);
    let string_ty = c.simple_type_for_def_id(builtin_type::STRING);

    let mut bob = Function::new(
        i32_ty,
        vec![
            LocalDecl::new(i32_ty, Some("x".into())),
            LocalDecl::new(i32_ty, Some("y".into())),
        ],
        "bob".into(),
    );

    let bob_tmp = bob.new_temp(i32_ty);

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

    let person = Struct::new("Person".into())
        .field("height".into(), i32_ty)
        .field("id".into(), i32_ty);

    let person_def_id = c.add_definition(Definition::Struct(person));
    let person_ty = c.simple_type_for_def_id(person_def_id);

    let mut m = Function::new(void_ty, vec![], "main".into());
    let call_result_tmp = m.new_temp(i32_ty);
    let interp_result_tmp = m.new_temp(string_ty);
    let person_result_tmp = m.new_temp(person_ty);

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

    bb2.push_stmt(StatementKind::Assign(
        Place::Local(person_result_tmp),
        Rvalue::Call(
            person_def_id,
            vec![Operand::ConstantInt(17), Operand::ConstantInt(18)],
        ),
    ));

    bb2.push_stmt(StatementKind::DebugPrint(Place::Field(
        person_result_tmp,
        "id".into(),
    )));

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
