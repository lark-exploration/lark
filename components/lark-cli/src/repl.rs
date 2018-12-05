use crate::build::LarkDatabaseExt;
use lark_entity::{EntityData, ItemKind};
use lark_eval::Value;
use lark_intern::{Intern, Untern};
use lark_mir::{FnBytecode, MirDatabase, StatementKind, Variable};
use lark_parser::{ParserDatabase, ParserDatabaseExt};
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::LarkDatabase;
use std::collections::HashMap;
use std::io::{stdin, stdout, Write};
use termcolor::{ColorChoice, StandardStream, WriteColor};

const REPL_FILENAME: &str = "__REPL__.lark";

pub fn get_bytecode(
    db: &LarkDatabase,
) -> lark_error::WithError<std::sync::Arc<lark_mir::FnBytecode>> {
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
                    let bytecode = db.fn_bytecode(entity);
                    return bytecode;
                }
            }
            _ => {}
        }
    }

    panic!("Internal error: Lost track of function bytecode")
}

pub fn repl() {
    let mut fn_body: Vec<String> = vec![];
    let mut io_handler = lark_eval::IOHandler::new(false);
    let mut db = LarkDatabase::default();
    let mut variables: HashMap<Variable, Vec<Value>> = HashMap::new();
    let mut num_to_skip = 0;

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
                format!("def main() {{\n{}\n}}", fn_body.join("\n"))
            );
            continue;
        }
        if input == ":v" {
            println!("{:#?}", variables);
            continue;
        }
        if input == ":?" {
            println!("Commands available:");
            println!("  :q - quit");
            println!("  :p - view currently accepted source lines");
            println!("  :v - view variables");
            continue;
        }

        fn_body.push(input);

        let _ = db.add_file(
            REPL_FILENAME,
            format!("def main() {{\n{}\n}}", fn_body.join("\n")),
        );

        let writer = StandardStream::stderr(ColorChoice::Auto);
        let error_count = db
            .display_errors(&mut writer.lock())
            .unwrap_or_else(|_| panic!("cancelled"));

        if error_count > 0 {
            // The last command was bad. Let's remove it from our function body
            fn_body.pop();
        } else {
            // No errors, so let's run the last line of our function body
            let fn_bytecode = get_bytecode(&mut db).value;

            //println!("{:#?}", fn_bytecode);

            let mut num_to_skip_remaining = num_to_skip;
            let mut num_to_skip_next = 0;
            let mut output = Value::Void;

            for basic_block in fn_bytecode.basic_blocks.iter(&fn_bytecode) {
                let basic_block_data = &fn_bytecode.tables[basic_block];

                for statement in basic_block_data.statements.iter(&fn_bytecode) {
                    if num_to_skip_remaining > 0 {
                        num_to_skip_remaining -= 1;
                    } else {
                        let statement_data = &fn_bytecode.tables[statement];

                        match &statement_data.kind {
                            StatementKind::StorageLive(variable) => {
                                num_to_skip_next += 1;
                                // Because we manage our own variables, we have to do a bit of bookkeeping
                                lark_eval::create_variable(&mut variables, *variable);
                            }
                            StatementKind::StorageDead(_) => {
                                // In the repl, we don't currently delete old variables
                            }
                            _ => {
                                num_to_skip_next += 1;
                                output = lark_eval::eval_statement(
                                    &mut db,
                                    &fn_bytecode,
                                    statement,
                                    &mut variables,
                                    &mut io_handler,
                                );
                            }
                        }
                    }
                }
            }

            num_to_skip += num_to_skip_next;
            match output {
                Value::Void => {}
                x => println!("{}", x),
            }
        }
    }
}
