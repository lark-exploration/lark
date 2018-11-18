use ast::AstDatabase;
use codespan::{ByteIndex, CodeMap, ColumnIndex, FileMap, FileName, LineIndex};
use flexi_logger::{opt_format, Logger};
use intern::{Intern, Untern};
use language_reporting::{emit, Diagnostic, Label, Severity};
use languageserver_types::Position;
use lark_entity::{EntityData, ItemKind, MemberKind};
use lark_language_server::{lsp_serve, LspResponder};
use lark_mir::MirDatabase;
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

pub(crate) fn build(filename: &str, output_filename: Option<&str>) {
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
    let file = db.add_file(filename, contents.to_string());

    match db.errors_for_project() {
        Ok(errors) => {
            let mut first = true;
            let mut error_count = 0;

            for (_filename, ranged_diagnostics) in errors {
                for ranged_diagnostic in ranged_diagnostics {
                    error_count += 1;
                    if !std::mem::replace(&mut first, false) {
                        eprintln!("");
                    }

                    let range = ranged_diagnostic.range;
                    let error = Diagnostic::new(Severity::Error, ranged_diagnostic.label);

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

            if error_count == 0 {
                let out_filename = if let Some(path) = output_filename {
                    path.to_string()
                } else {
                    let file_path = if cfg!(windows) {
                        std::path::Path::new(filename).with_extension("exe")
                    } else {
                        std::path::Path::new(filename).with_extension("")
                    };

                    file_path.file_name().unwrap().to_str().unwrap().to_string()
                };

                let source_file = lark_build::codegen(&mut db, lark_build::CodegenType::Rust);

                lark_build::build(
                    &out_filename,
                    &source_file.value,
                    lark_build::CodegenType::Rust,
                )
                .unwrap();
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
