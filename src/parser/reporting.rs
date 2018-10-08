use codespan::ByteOffset;
use crate::parser::ast::DebugModuleTable;

use codespan::ByteIndex;
use codespan::CodeMap;
use crate::parser::ast::{Debuggable, DebuggableVec, Mode};
use crate::parser::lexer_helpers::ParseError;
use crate::parser::pos::{Span, Spanned};
use crate::parser::program::ModuleTable;
use crate::parser::program::StringId;
use crate::parser::{self, ast};

use derive_new::new;
use itertools::Itertools;
use language_reporting::{emit, Diagnostic, Label, Severity};
use log::{debug, trace, warn};
use std::collections::HashMap;
use termcolor::{ColorChoice, StandardStream};

pub fn print_parse_error(e: ParseError, codemap: &CodeMap) -> ! {
    let error = Diagnostic::new(Severity::Error, e.description)
        .with_label(Label::new_primary(e.span.to_codespan()));
    let writer = StandardStream::stderr(ColorChoice::Auto);
    emit(
        &mut writer.lock(),
        &codemap,
        &error,
        &language_reporting::DefaultConfig,
    )
    .unwrap();
    panic!("Parse Error");
}
