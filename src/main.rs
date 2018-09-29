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
mod hir;
mod intern;
mod ir;
mod parser;
mod ty;
mod typeck;

use std::io::prelude::Read;
use std::{env, io};

use crate::codegen::{codegen, RustFile};
use crate::eval::eval_context;
use crate::ir::{
    builtin_type, BasicBlock, BinOp, Context, Definition, Function, LocalDecl, Operand, Place,
    Rvalue, StatementKind, Struct, TerminatorKind,
};
use serde_derive::{Deserialize, Serialize};

use crate::ty::intern::TyInterners;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
#[allow(non_camel_case_types)]
enum LSCommand {
    initialize {
        id: usize,
        params: languageserver_types::InitializeParams,
    },
}

fn build(_filename: &str) {}

fn run(_filename: &str) {}

fn repl() {}

fn ide() {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            //eprintln!("{} bytes read", n);
            //eprintln!("{}", input);
            let content_length_items: Vec<&str> = input.split(' ').collect();
            if content_length_items[0] == "Content-Length:" {
                let num_bytes: usize = content_length_items[1].trim().parse().unwrap();
                let mut buffer = vec![0u8; num_bytes + 2];
                let read_result = io::stdin().read(&mut buffer);

                eprintln!("read in {:?}", read_result);

                let buffer_string = String::from_utf8(buffer).unwrap();
                eprintln!("command: {}", buffer_string);

                let command = serde_json::from_str::<LSCommand>(&buffer_string);

                eprintln!("Command: {:#?}", command);
            }
        }
        Err(error) => eprintln!("error: {}", error),
    }
}

fn internaltest() {
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

    eval_context(&c, main_def_id);
}

fn main() {
    let mut args = std::env::args();

    match (args.next(), args.next(), args.next()) {
        (_, Some(ref cmd), Some(ref x)) if cmd == "build" => build(x),
        (_, Some(ref cmd), Some(ref x)) if cmd == "run" => run(x),
        (_, Some(ref cmd), None) if cmd == "repl" => repl(),
        (_, Some(ref cmd), None) if cmd == "ide" => ide(),
        (_, Some(ref cmd), None) if cmd == "internaltest" => internaltest(),
        _ => {
            println!("Usage:");
            println!("  lark build <file> - compiles the given file");
            println!("  lark run <file>   - runs the given file");
            println!("  lark repl         - REPL/interactive mode");
            println!("  lark ide          - run the Lark languge server/IDE support");
            println!("  lark internaltest - run some internal tests");
        }
    }
}
