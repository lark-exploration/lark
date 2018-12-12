use crate::build::LarkDatabaseExt;
use lark_debug_with::DebugWith;
use lark_entity::{EntityData, ItemKind};
use lark_eval::Value;
use lark_hir as hir;
use lark_intern::{Intern, Untern};
use lark_parser::{ParserDatabase, ParserDatabaseExt};
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::LarkDatabase;
use std::collections::HashMap;
use std::io::{stdin, stdout, Write};
use termcolor::{ColorChoice, StandardStream, WriteColor};

const REPL_FILENAME: &str = "__REPL__.lark";

pub fn get_body(db: &LarkDatabase) -> lark_error::WithError<std::sync::Arc<lark_hir::FnBody>> {
    let main_name = "main".intern(&db);
    let repl_filename = REPL_FILENAME.intern(&db);
    let entities = db.top_level_entities_in_file(repl_filename);

    for &entity in &*entities {
        match entity.untern(&db) {
            EntityData::ItemName {
                kind: ItemKind::Function,
                id,
                ..
            } => {
                if id == main_name {
                    let body = db.fn_body(entity);
                    return body;
                }
            }
            _ => {}
        }
    }

    panic!("Internal error: Lost track of function bytecode")
}

pub fn repl() {
    let mut virtual_fn: Vec<String> = vec![];
    let mut io_handler = lark_eval::IOHandler::new(false);
    let mut db = LarkDatabase::default();

    let _ = db.add_file(
        REPL_FILENAME,
        format!("def main() {{\n{}\n}}", virtual_fn.join("\n")),
    );

    let mut eval_state = lark_eval::EvalState::new();
    eval_state.is_repl = true;

    println!("Lark repl (:? - command help)");
    loop {
        let mut input = String::new();

        print!("> ");
        let _ = stdout().flush();
        stdin().read_line(&mut input).expect("Could not read input");
        input = input.trim().to_string();

        if input == ":q" {
            break;
        }
        if input == ":p" {
            println!(
                "Will execute: {}",
                format!("def main() {{\n{}\n}}", virtual_fn.join("\n"))
            );
            continue;
        }
        if input == ":v" {
            println!("{:#?}", eval_state.variables);
            continue;
        }
        if input == ":?" {
            println!("Commands available:");
            println!("  :q - quit");
            println!("  :p - view currently accepted source lines");
            println!("  :v - view variables");
            continue;
        }

        virtual_fn.push(input);

        let _ = db.add_file(
            REPL_FILENAME,
            format!("def main() {{\n{}\n}}", virtual_fn.join("\n")),
        );

        let writer = StandardStream::stderr(ColorChoice::Auto);
        let error_count = db
            .display_errors(&mut writer.lock())
            .unwrap_or_else(|_| panic!("cancelled"));

        if error_count > 0 {
            // The last command was bad. Let's remove it from our function body
            virtual_fn.pop();
        } else {
            // No errors, so let's run the last line of our function body
            let fn_body = get_body(&mut db).value;

            let output = lark_eval::eval_expression(
                &db,
                &fn_body,
                fn_body.root_expression,
                &mut eval_state,
                &mut io_handler,
            );

            eval_state.skip_until = eval_state.current_expression;

            match output {
                Value::Void => {}
                x => println!("{}", x),
            }
        }
    }
}
