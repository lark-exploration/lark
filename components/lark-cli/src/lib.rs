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
#![feature(trace_macros)]
#![allow(dead_code)]
#![allow(unused_imports)]

use flexi_logger::{opt_format, Logger};
use std::{env, io};

mod build;
mod ide;
mod repl;
mod run;

pub fn main() {
    Logger::with_env_or_str("error,lark_query_system=info")
        .log_to_file()
        .directory("log_files")
        .format(opt_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    let mut args = std::env::args();

    match (args.next(), args.next(), args.next(), args.next()) {
        (_, Some(ref cmd), Some(ref x), Some(ref out)) if cmd == "build" => {
            build::build(x, Some(out))
        }
        (_, Some(ref cmd), Some(ref x), None) if cmd == "build" => build::build(x, None),
        (_, Some(ref cmd), Some(ref x), None) if cmd == "run" => run::run(x),
        (_, Some(ref cmd), None, None) if cmd == "repl" => repl::repl(),
        (_, Some(ref cmd), None, None) if cmd == "ide" => ide::ide(),
        _ => {
            println!("Usage:");
            println!("  lark build <file> [<output>] - compiles the given file");
            println!("  lark run <file>              - runs the given file");
            println!("  lark repl                    - REPL/interactive mode");
            println!("  lark ide                     - run the Lark languge server/IDE support");
        }
    }
}
