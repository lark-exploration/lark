#![deny(rust_2018_idioms)]
#![feature(in_band_lifetimes)]
#![feature(box_patterns)]
#![feature(box_syntax)]
#![feature(crate_visibility_modifier)]
#![feature(existential_type)]
#![feature(self_in_typedefs)]
#![feature(never_type)]
#![feature(nll)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(try_from)]
#![feature(macro_at_most_once_rep)]
#![feature(trace_macros)]
#![allow(dead_code)]
#![allow(unused_imports)]

use flexi_logger::{opt_format, Logger};
use lark_language_server::{lsp_serve, LspResponder};
use lark_query_system::QuerySystem;
use lark_task_manager::Actor;
use std::{env, io};

mod build;

fn run(_filename: &str) {}

fn repl() {}

fn ide() {
    let query_system = QuerySystem::new();
    let lsp_responder = LspResponder;

    let task_manager = lark_task_manager::TaskManager::spawn(query_system, lsp_responder);

    lsp_serve(task_manager.channel);
    let _ = task_manager.join_handle.join();
}

fn main() {
    Logger::with_env_or_str("error,lark_query_system=info")
        .log_to_file()
        .directory("log_files")
        .format(opt_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    let mut args = std::env::args();

    eprintln!("Lark: executing");
    log::error!("Lark: executing");

    match (args.next(), args.next(), args.next()) {
        (_, Some(ref cmd), Some(ref x)) if cmd == "build" => build::build(x),
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
