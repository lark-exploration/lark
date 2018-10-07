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
mod indices;

mod codegen;
mod debug;
mod eval;
mod hir;
mod ide;
mod intern;
mod ir;
mod map;
mod parser;
mod parser2;
mod task_manager;
mod tests;
mod ty;
mod type_check;
mod unify;

use std::{env, io};

use crate::ide::lsp_serve;

fn build(_filename: &str) {}

fn run(_filename: &str) {}

fn repl() {}

fn ide() {
    let mut task_manager = task_manager::TaskManager::new();

    task_manager.start_type_checker();
    task_manager.start_lsp_server();

    let send_to_manager_channel = task_manager.send_to_manager.clone();
    let join_handle = task_manager.start();

    lsp_serve(send_to_manager_channel);
    let _ = join_handle.join();
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
