use flexi_logger::{opt_format, Logger};
use language_reporting::{emit, Diagnostic, Label, Severity};
use languageserver_types::Position;
use lark_entity::{EntityData, ItemKind, MemberKind};
use lark_intern::{Intern, Untern};
use lark_language_server::{lsp_serve, LspResponder};
use lark_parser::{ParserDatabase, ParserDatabaseExt};
use lark_query_system::ls_ops::Cancelled;
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::LarkDatabase;
use lark_query_system::QuerySystem;
use lark_span::{ByteIndex, FileName, IntoFileName, Span};
use lark_task_manager::Actor;
use salsa::Database;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::{env, io};
use termcolor::{ColorChoice, StandardStream, WriteColor};

pub fn build(file_name: &str, output_file_name: Option<&str>) {
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

    let writer = StandardStream::stderr(ColorChoice::Auto);
    let error_count = db
        .display_errors(&mut writer.lock())
        .unwrap_or_else(|Cancelled| panic!("cancelled"));

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

        db.build(&out_file_name)
            .unwrap_or_else(|Cancelled| panic!("cancelled"));
    }
}

pub trait LarkDatabaseExt {
    fn display_errors(&self, out: impl WriteColor) -> Result<usize, Cancelled>;

    /// Build an executable into `output_file_name`.
    fn build(&self, output_file_name: &str) -> Result<(), Cancelled>;
}

impl LarkDatabaseExt for LarkDatabase {
    fn build(&self, output_file_name: &str) -> Result<(), Cancelled> {
        let source_file = lark_build::codegen(self, lark_build::CodegenType::Rust);

        lark_build::build(
            &output_file_name,
            &source_file.value,
            lark_build::CodegenType::Rust,
        )
        .unwrap();

        Ok(())
    }

    /// Displays all errors for the project on stderr. Returns `Ok(n)` where
    /// n is the number of errors (or `Cancelled` if execution is cancelled).
    fn display_errors(&self, mut out: impl WriteColor) -> Result<usize, Cancelled> {
        let db = self;

        let errors = db.errors_for_project()?;
        let mut first = true;
        let mut error_count = 0;

        for (file_name, ranged_diagnostics) in errors {
            let file_id: FileName = file_name.into_file_name(&db);

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

                emit(&mut out, &db, &error, &language_reporting::DefaultConfig).unwrap();
            }
        }

        Ok(error_count)
    }
}
