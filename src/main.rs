#![deny(rust_2018_idioms)]
#![feature(in_band_lifetimes)]
#![feature(box_patterns)]
#![feature(box_syntax)]
#![feature(crate_visibility_modifier)]
#![feature(existential_type)]
#![feature(self_in_typedefs)]
#![feature(never_type)]
#![feature(nll)]
#![feature(min_const_fn)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(try_from)]
#![feature(macro_at_most_once_rep)]
#![feature(trace_macros)]
#![allow(dead_code)]
#![allow(unused_imports)]

#[macro_use]
mod lexer;

#[macro_use]
mod intern;

mod debug;
mod hir;
mod parser;
mod parser2;
mod tests;
mod ty;
mod type_check;
mod unify;

use std::{env, io};

use ide::{lsp_serve, LspResponder};
use task_manager::{Actor, FakeTypeChecker};

fn build(_filename: &str) {}

fn run(_filename: &str) {}

fn repl() {}

fn ide() {
    let type_checker = FakeTypeChecker::new();
    let lsp_responder = LspResponder;

    let task_manager = task_manager::TaskManager::spawn(type_checker, lsp_responder);

    lsp_serve(task_manager.channel);
    let _ = task_manager.join_handle.join();
}

fn main() {
    let mut args = std::env::args();

    match (args.next(), args.next(), args.next()) {
        (_, Some(ref cmd), Some(ref x)) if cmd == "build" => build(x),
        (_, Some(ref cmd), Some(ref x)) if cmd == "run" => run(x),
        (_, Some(ref cmd), None) if cmd == "repl" => repl(),
        (_, Some(ref cmd), None) if cmd == "ide" => ide(),
        _ => {
            println!("Usage:");
            println!("  lark build <file> - compiles the given file");
            println!("  lark run <file>   - runs the given file");
            println!("  lark repl         - REPL/interactive mode");
            println!("  lark ide          - run the Lark languge server/IDE support");
        }
    }
}
