use crate::harness::test::TestContext;
use lark_cli::build::LarkDatabaseExt;
use lark_query_system::ls_ops::Cancelled;
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::ls_ops::RangedDiagnostic;
use std::collections::HashMap;
use termcolor::NoColor;

impl TestContext<'_> {
    crate fn run_compilation_test(&self, expect_error: bool) {
        if expect_error && self.options.expected_errors.is_empty() {
            panic!("no errors specified -- consider using `mode:compile_pass`")
        }

        let errors = match self.db.errors_for_project() {
            Ok(errors) => errors,
            Err(Cancelled) => panic!("encountered cancellation in unit test"),
        };

        self.compare_errors_against_expected(errors);
        self.compare_stderr_against_expected();
    }

    fn compare_errors_against_expected(&self, errors: HashMap<String, Vec<RangedDiagnostic>>) {
        let mut expected_errors: Vec<_> = self.options.expected_errors.iter().collect();
        let mut unexpected_errors = vec![];
        for (file_name, errors) in errors {
            if file_name != self.test_name {
                unexpected_errors.extend(errors);
                continue;
            }

            for error in errors {
                let matching_expected_error = expected_errors.iter().position(|ee| {
                    ee.line_num == error.range.start.line && ee.message.is_match(&error.label)
                });

                if let Some(i) = matching_expected_error {
                    expected_errors.remove(i);
                } else {
                    unexpected_errors.push(error);
                }
            }
        }

        if !unexpected_errors.is_empty() {
            eprintln!("# Unexpected errors");
            for error in &unexpected_errors {
                eprintln!(
                    "{}:{}:{}:{}:{}: {}",
                    self.test_path.path.display(),
                    error.range.start.line + 1,
                    error.range.start.character + 1,
                    error.range.end.line + 1,
                    error.range.end.character + 1,
                    error.label,
                );
            }
        }

        if !expected_errors.is_empty() {
            eprintln!("# Expected errors not found");
            for error in &expected_errors {
                eprintln!(
                    "{}:{}: something matching `{}`",
                    self.test_path.path.display(),
                    error.line_num + 1,
                    error.message,
                );
            }
        }

        assert!(unexpected_errors.is_empty() && expected_errors.is_empty());
    }

    fn compare_stderr_against_expected(&self) {
        let mut buffer = Vec::new();
        self.db
            .display_errors(NoColor::new(&mut buffer))
            .unwrap_or_else(|Cancelled| panic!("cancelled?"));

        self.compare_reference_contents("stderr", &buffer);
    }
}
