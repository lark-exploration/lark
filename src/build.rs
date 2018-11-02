use ast::HasParserState;
use codespan::{CodeMap, ColumnIndex, FileMap, FileName, LineIndex};
use flexi_logger::{opt_format, Logger};
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
        .insert(filename.to_string(), file_map);
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
            for (filename, ranges) in errors {
                for range in ranges {
                    eprintln!(
                        "{}:{}:{}:{}:{}: error",
                        filename,
                        range.start.line,
                        range.start.character,
                        range.end.line,
                        range.end.character,
                    );
                }
            }
        }

        Err(Cancelled) => unreachable!("cancellation?"),
    }
}
