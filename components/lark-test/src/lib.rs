#![feature(crate_visibility_modifier)]
#![feature(specialization)]

use lark_collections::seq;
use lark_intern::Intern;
use lark_parser::{ParserDatabase, ParserDatabaseExt};
use lark_query_system::ls_ops::{Cancelled, LsDatabase, RangedDiagnostic};
use lark_query_system::LarkDatabase;
use lark_span::FileName;
use lark_string::Text;
use salsa::Database;
use std::fmt::Debug;

mod harness;
pub use harness::run_test_harness;
pub use harness::search_files;
pub use harness::TestPath;

pub use lark_debug_with::DebugWith;
pub use lark_span::IntoFileName;

pub trait ErrorSpec {
    fn check_errors(&self, errors: &[RangedDiagnostic]);
}

pub struct NoErrors;

impl ErrorSpec for NoErrors {
    fn check_errors(&self, errors: &[RangedDiagnostic]) {
        if errors.is_empty() {
            return;
        }

        for error in errors {
            eprintln!("{:?}", error);
        }

        assert_eq!(0, errors.len());
    }
}

impl ErrorSpec for &str {
    fn check_errors(&self, errors: &[RangedDiagnostic]) {
        assert_eq!(
            errors.len(),
            1,
            "expected exactly one error, got {:#?}",
            errors
        );

        for error in errors {
            let range = error.range;

            let expected = format!("0:{}..0:{}", self.find('~').unwrap(), self.len());
            let actual = format!(
                "{}:{}..{}:{}",
                range.start.line, range.start.character, range.end.line, range.end.character
            );

            if expected != actual {
                eprintln!("expected error on {}", expected);
                eprintln!("found error on {}", actual);
                eprintln!("error = {:#?}", error);
            }

            assert_eq!(expected, actual);
        }
    }
}

pub fn db_with_test(file_name: impl IntoFileName, text: &str) -> LarkDatabase {
    let mut db = LarkDatabase::default();
    db.add_file(file_name, text);
    db
}

pub fn run_test(text: &str, error_spec: impl ErrorSpec) {
    let file_name_str = "input.lark";
    let db = db_with_test(file_name_str, text);
    let parsed = db.parsed_file(file_name_str.into_file_name(&db));
    assert!(parsed.value.entities().len() >= 1, "input with no items");

    match db.errors_for_project() {
        Ok(errors) => {
            let flat_errors: Vec<_> = errors
                .into_iter()
                .flat_map(|(file_name, errors)| {
                    assert_eq!(file_name, file_name_str);
                    errors
                })
                .collect();
            error_spec.check_errors(&flat_errors);
        }

        Err(Cancelled) => {
            panic!("cancelled?!");
        }
    }
}

/// Creates a lark database with a single file containing the given
/// test. Intended for tests targeting the `lark_parser` crate, which
/// is not yet fully wired into everything else.
pub fn lark_parser_db(text: impl AsRef<str>) -> (FileName, LarkDatabase) {
    let text: &str = text.as_ref();
    let mut db = LarkDatabase::default();

    // Setup the input:
    let path1 = FileName {
        id: "path1".intern(&db),
    };
    let text = Text::from(text);
    db.query_mut(lark_parser::FileNamesQuery)
        .set((), seq![path1]);
    db.query_mut(lark_parser::FileTextQuery).set(path1, text);

    (path1, db)
}

/// Test that two values are equal, with a better error than `assert_eq`
pub fn assert_equal<Cx, A>(cx: &Cx, expected_value: &A, actual_value: &A)
where
    A: ?Sized + Debug + DebugWith + Eq,
{
    // First check that they have the same debug text. This produces a better error.
    let expected_text = format!("{:#?}", expected_value.debug_with(cx));
    assert_expected_debug(cx, &expected_text, actual_value);

    // Then check that they are `eq` too, for good measure.
    assert_eq!(expected_value, actual_value);
}

/// Test that the debug output of `actual_value` is as expected. Gives
/// a nice diff if things fail.
pub fn assert_expected_debug<Cx, A>(cx: &Cx, expected_text: &str, actual_value: &A)
where
    A: ?Sized + DebugWith,
{
    let actual_text = format!("{:#?}", actual_value.debug_with(cx));

    if expected_text == actual_text {
        return;
    }

    println!("# expected_text");
    println!("{}", expected_text);

    println!("# actual_text");
    println!("{}", actual_text);

    println!("# diff");
    for diff in diff::lines(&expected_text, &actual_text) {
        match diff {
            diff::Result::Left(l) => println!("-{}", l),
            diff::Result::Both(l, _) => println!(" {}", l),
            diff::Result::Right(r) => println!("+{}", r),
        }
    }

    panic!("debug comparison failed");
}
