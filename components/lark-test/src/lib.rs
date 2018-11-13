use ast::AstDatabase;
use debug::DebugWith;
use intern::Intern;
use lark_parser::FileName;
use lark_query_system::ls_ops::Cancelled;
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::ls_ops::RangedDiagnostic;
use lark_query_system::LarkDatabase;
use lark_seq::seq;
use lark_string::text::Text;
use parser::HasParserState;
use parser::HasReaderState;
use salsa::Database;

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

pub fn run_test(text: &str, error_spec: impl ErrorSpec) {
    let mut db = LarkDatabase::default();
    let path1_str = "path1";
    let path1_interned = db.intern_string("path1");

    db.add_file(path1_str, text);

    let items_in_file = db.items_in_file(path1_interned);
    assert!(items_in_file.len() >= 1, "input with no items");

    match db.errors_for_project() {
        Ok(errors) => {
            let flat_errors: Vec<_> = errors
                .into_iter()
                .flat_map(|(file_name, errors)| {
                    assert_eq!(file_name, path1_str);
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

pub fn compare_debug<Cx, A>(cx: &Cx, expected_text: &str, actual_value: &A)
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
