use lark_query_system::LarkDatabase;
use parser::{HasParserState, HasReaderState, ReaderDatabase};
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

    lark_eval::eval(&mut db, &mut lark_eval::IOHandler::new(false));
}
