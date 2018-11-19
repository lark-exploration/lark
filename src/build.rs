use flexi_logger::{opt_format, Logger};
use language_reporting::{emit, Diagnostic, Label, Severity};
use languageserver_types::Position;
use lark_language_server::{lsp_serve, LspResponder};
use lark_parser::{IntoFileName, ParserDatabase, ParserDatabaseExt};
use lark_query_system::ls_ops::{Cancelled, LsDatabase};
use lark_query_system::{LarkDatabase, QuerySystem};
use lark_span::{ByteIndex, FileName, Span};
use lark_task_manager::Actor;
use salsa::Database;
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::{env, io};
use termcolor::{ColorChoice, StandardStream};

pub(crate) fn build(file_name: &str) {
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
    let file_name: FileName = file_name.into_file_name(&db);
    db.add_file(file_name, contents);

    let file_id = file_name.into_file_name(&db);

    match db.errors_for_project() {
        Ok(errors) => {
            let mut first = true;
            for (_filename, ranged_diagnostics) in errors {
                for ranged_diagnostic in ranged_diagnostics {
                    if !std::mem::replace(&mut first, false) {
                        eprintln!("");
                    }

                    let range = ranged_diagnostic.range;
                    let error = Diagnostic::new(Severity::Error, ranged_diagnostic.label);

                    let span = Span::new(
                        file_name,
                        db.byte_index(file_id, range.start.line, range.start.character),
                        db.byte_index(file_id, range.end.line, range.end.character),
                    );

                    let error = error.with_label(Label::new_primary(span));

                    let writer = StandardStream::stderr(ColorChoice::Auto);

                    emit(
                        &mut writer.lock(),
                        &&db,
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
