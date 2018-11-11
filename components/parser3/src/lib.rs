#![feature(const_fn)]
#![feature(const_let)]
#![feature(crate_visibility_modifier)]
#![feature(macro_at_most_once_rep)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]
#![feature(try_blocks)]
#![allow(dead_code)]

use crate::span::CurrentFile;
use crate::span::Span;
use lark_debug_derive::DebugWith;
use lark_error::Diagnostic;
use lark_string::global::GlobalIdentifier;
use lark_string::global::GlobalIdentifierTables;
use lark_string::text::Text;
use std::sync::Arc;

pub mod current_file;
mod lexer;
mod macros;
mod parser;
pub mod span;
pub mod syntax;

salsa::query_group! {
    pub trait ParserDatabase: AsRef<GlobalIdentifierTables> + salsa::Database {
        fn file_names() -> Arc<Vec<FileName>> {
            type FileNamesQuery;
            storage input;
        }

        fn file_text(id: FileName) -> Text {
            type FileTextQuery;
            storage input;
        }
    }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct FileName {
    pub id: GlobalIdentifier,
}

fn diagnostic(message: String, span: Span<CurrentFile>) -> Diagnostic {
    drop(span); // FIXME -- Diagostic uses the old codemap spans
    Diagnostic::new(message, ::parser::pos::Span::Synthetic)
}
