use ast::AstDatabase;
use codespan::{ByteIndex, CodeMap, ColumnIndex, FileMap, FileName, LineIndex};
use flexi_logger::{opt_format, Logger};
use intern::{Intern, Untern};
use language_reporting::{emit, Diagnostic, Label, Severity};
use languageserver_types::Position;
use lark_entity::{EntityData, ItemKind, MemberKind};
use lark_language_server::{lsp_serve, LspResponder};
use lark_mir2::MirDatabase;
use lark_query_system::ls_ops::Cancelled;
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::LarkDatabase;
use lark_query_system::QuerySystem;
use lark_task_manager::Actor;
use parser::pos::Span;
use parser::{HasParserState, HasReaderState, ReaderDatabase};
use salsa::Database;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::{env, io};
use termcolor::{ColorChoice, StandardStream};

pub(crate) fn build(filename: &str) {
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

    let file_path = if cfg!(windows) {
        std::path::Path::new(filename).with_extension("exe")
    } else {
        std::path::Path::new(filename).with_extension("")
    };

    let source_file = lark_codegen2::codegen(&mut db, lark_codegen2::CodegenType::Rust);

    if source_file.errors.len() > 0 {
        for error in source_file.errors {
            /*
            let writer = StandardStream::stderr(ColorChoice::Auto);
            emit(
                &mut writer.lock(),
                &db.code_map().read(),
                &error,
                &language_reporting::DefaultConfig,
            )
            .unwrap();
            */
            println!("Error: {:#?}", error);
        }
    } else {
        let out_filename = file_path.file_name().unwrap().to_str().unwrap();

        println!("Output: {}", out_filename);
        println!("{}", source_file.value);
        lark_codegen2::build(
            out_filename,
            &source_file.value,
            lark_codegen2::CodegenType::Rust,
        )
        .unwrap();
    }
}

trait FileMapExt {
    fn byte_index_for_position(&self, position: Position) -> ByteIndex;
}

impl FileMapExt for FileMap {
    fn byte_index_for_position(&self, position: Position) -> ByteIndex {
        let line = LineIndex::from(position.line as u32);
        let column = ColumnIndex::from(position.character as u32);
        self.byte_index(line, column).unwrap()
    }
}
