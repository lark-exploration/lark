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

pub mod build;
mod ide;
mod repl;
mod run;

pub fn codegen(file_name: &str) {
    use flexi_logger::{opt_format, Logger};
    use language_reporting::{emit, Diagnostic, Label, Severity};
    use languageserver_types::Position;
    use lark_entity::{EntityData, ItemKind, MemberKind};
    use lark_intern::{Intern, Untern};
    use lark_language_server::{lsp_serve, LspResponder};
    use lark_mir::MirDatabase;
    use lark_parser::{ParserDatabase, ParserDatabaseExt};
    use lark_query_system::ls_ops::Cancelled;
    use lark_query_system::ls_ops::LsDatabase;
    use lark_query_system::LarkDatabase;
    use lark_query_system::QuerySystem;
    use lark_span::{ByteIndex, FileName, IntoFileName, Span};
    use lark_task_manager::Actor;
    use salsa::Database;
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Read;
    use std::sync::Arc;
    use std::{env, io};
    use termcolor::{ColorChoice, StandardStream, WriteColor};

    let mut file = match File::open(file_name) {
        Ok(f) => f,
        Err(err) => {
            eprintln!("failed to open `{}`: {}", file_name, err);
            return;
        }
    };

    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
        Ok(_bytes_read) => {}
        Err(err) => {
            eprintln!("failed to read `{}`: {}", file_name, err);
            return;
        }
    }

    let mut db = LarkDatabase::default();

    let file_id: FileName = file_name.into_file_name(&db);
    db.add_file(file_id, contents);

    let rust_file = lark_build_hir::codegen(&db, lark_build_hir::CodegenType::Rust);

    println!("{}", rust_file.into_value());
}

pub fn main() {
    Logger::with_env_or_str("error,lark_query_system=info")
        .log_to_file()
        .directory("log_files")
        .format(opt_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    let mut args = std::env::args();

    match (args.next(), args.next(), args.next(), args.next()) {
        (_, Some(ref cmd), Some(ref x), None) if cmd == "codegen" => codegen(x),
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
