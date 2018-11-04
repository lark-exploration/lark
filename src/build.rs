use codespan::{ByteIndex, CodeMap, ColumnIndex, FileMap, FileName, LineIndex};
use flexi_logger::{opt_format, Logger};
use language_reporting::{emit, Diagnostic, Label, Severity};
use languageserver_types::Position;
use lark_language_server::{lsp_serve, LspResponder};
use lark_query_system::ls_ops::Cancelled;
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::LarkDatabase;
use lark_query_system::QuerySystem;
use lark_task_manager::Actor;
use parser::pos::Span;
use parser::{HasParserState, ReaderDatabase};
use salsa::Database;
use std::borrow::Cow;
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

    let file = parser::add_file(&mut db, filename, contents.to_string());

    match db.errors_for_project() {
        Ok(errors) => {
            let mut first = true;
            for (_filename, ranges) in errors {
                for range in ranges {
                    if !std::mem::replace(&mut first, false) {
                        eprintln!("");
                    }

                    let error = Diagnostic::new(Severity::Error, "something is wrong here =)");

                    let span = codespan::Span::new(
                        file.byte_index(range.start.line, range.start.character)
                            .unwrap(),
                        file.byte_index(range.end.line, range.end.character)
                            .unwrap(),
                    );

                    let error = error.with_label(Label::new_primary(span));

                    let writer = StandardStream::stderr(ColorChoice::Auto);

                    emit(
                        &mut writer.lock(),
                        &db.code_map().read(),
                        &error,
                        &language_reporting::DefaultConfig,
                    )
                    .unwrap();
                }
            }
        }

        Err(Cancelled) => unreachable!("cancellation?"),
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
