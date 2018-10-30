use crate::prelude::*;

use codespan::CodeMap;
use language_reporting::{emit, Diagnostic, Label, Severity};
use termcolor::{ColorChoice, StandardStream};

pub fn print_parse_error(e: ParseError, codemap: &CodeMap) -> ! {
    let error = Diagnostic::new(Severity::Error, e.description);

    let error = match e.span {
        Span::Real(codespan) => error.with_label(Label::new_primary(codespan)),
        Span::EOF => error,
        Span::Synthetic => error,
    };

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
