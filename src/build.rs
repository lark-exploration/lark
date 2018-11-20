use flexi_logger::{opt_format, Logger};
use intern::{Intern, Untern};
use language_reporting::{emit, Diagnostic, Label, Severity};
use languageserver_types::Position;
use lark_entity::{EntityData, ItemKind, MemberKind};
use lark_language_server::{lsp_serve, LspResponder};
use lark_mir::MirDatabase;
use lark_parser::{IntoFileName, ParserDatabase, ParserDatabaseExt};
use lark_query_system::ls_ops::Cancelled;
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::LarkDatabase;
use lark_query_system::QuerySystem;
use lark_span::{ByteIndex, FileName, Span};
use lark_task_manager::Actor;
use salsa::Database;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::{env, io};
use termcolor::{ColorChoice, StandardStream};

pub(crate) fn build(file_name: &str, output_file_name: Option<&str>) {
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

    let file_id: FileName = file_name.into_file_name(&db);
    db.add_file(file_id, contents);

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

                    let span = Span::new(
                        file_id,
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

            if error_count == 0 {
                let out_file_name = if let Some(path) = output_file_name {
                    path.to_string()
                } else {
                    let file_path = if cfg!(windows) {
                        std::path::Path::new(file_name).with_extension("exe")
                    } else {
                        std::path::Path::new(file_name).with_extension("")
                    };

                    file_path.file_name().unwrap().to_str().unwrap().to_string()
                };

                let source_file = lark_build::codegen(&mut db, lark_build::CodegenType::Rust);

                lark_build::build(
                    &out_file_name,
                    &source_file.value,
                    lark_build::CodegenType::Rust,
                )
                .unwrap();
            }
        }

        Err(Cancelled) => unreachable!("cancellation?"),
    }
}
