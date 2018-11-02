use ast::HasParserState;
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
    let interned_filename = db.intern_string(filename);
    let interned_contents = db.intern_string(&contents[..]);
    db.query_mut(ast::InputFilesQuery)
        .set((), Arc::new(vec![interned_filename]));
    let file_map = db.code_map().write().add_filemap(
        FileName::Virtual(Cow::Owned(filename.to_string())),
        contents.to_string(),
    );
    let file_span = file_map.span();
    let start_offset = file_map.span().start().to_usize() as u32;
    db.file_maps()
        .write()
        .insert(filename.to_string(), file_map.clone());
    db.query_mut(ast::InputTextQuery).set(
        interned_filename,
        Some(ast::InputText {
            text: interned_contents,
            start_offset,
            span: Span::from(file_span),
        }),
    );
    match db.errors_for_project() {
        Ok(errors) => {
            let mut first = true;
            for (_filename, labeled_ranges) in errors {
                for labeled_range in labeled_ranges {
                    if !std::mem::replace(&mut first, false) {
                        eprintln!("");
                    }

                    let error = Diagnostic::new(Severity::Error, labeled_range.label);

                    let span = codespan::Span::new(
                        file_map.byte_index_for_position(labeled_range.range.start),
                        file_map.byte_index_for_position(labeled_range.range.end),
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
