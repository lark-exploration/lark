use lark_parser::ParserDatabaseExt;
use lark_query_system::LarkDatabase;
use crate::build::LarkDatabaseExt;
use lark_query_system::ls_ops::Cancelled;
use termcolor::{ColorChoice, StandardStream, WriteColor};
use std::fs::File;
use std::io::Read;

pub fn run(filename: &str) {
    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(err) => {
            eprintln!("failed to open `{}`: {}", filename, err);
            return;
        }
    };

    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
        Ok(_bytes_read) => {}
        Err(err) => {
            eprintln!("failed to read `{}`: {}", filename, err);
            return;
        }
    }

    let mut db = LarkDatabase::default();
    let _ = db.add_file(filename, contents.to_string());

    // Check for errors, and only run if there aren't any
    let writer = StandardStream::stderr(ColorChoice::Auto);
    let error_count = db
        .display_errors(&mut writer.lock())
        .unwrap_or_else(|Cancelled| panic!("cancelled"));

    if error_count == 0 {
        lark_eval::eval(&mut db, &mut lark_eval::IOHandler::new(false));
    }
}
