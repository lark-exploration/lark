use crate::harness::options::ExecutionMode;
use crate::harness::options::TestOptions;
use lark_query_system::ls_ops::Cancelled;
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::LarkDatabase;
use std::path::Path;

mod compilation_test;
mod execution_test;
mod util;

crate struct TestContext<'me> {
    /// If true, generate `stderr` files etc from the actual content (and do not
    /// error if old contents were wrong or missing).
    crate bless_mode: bool,

    /// Full path to the lark file (e.g., `tests/test_files/foo/bar.lark`)
    /// (might be a directory)
    crate test_path: &'me Path,

    /// Path to the lark file relative to the test file set (e.g. `foo/bar.lark`)
    crate relative_test_path: &'me Path,

    /// Test name -- effectively relative test path without extension (e.g., `foo/bar`)
    crate test_name: String,

    /// Lark database used during the test
    crate db: LarkDatabase,

    /// Options parsed from the comments in the file
    crate options: TestOptions,
}

impl TestContext<'_> {
    crate fn execute(self) {
        let errors = match self.db.errors_for_project() {
            Ok(errors) => errors,
            Err(Cancelled) => panic!("encountered cancellation in unit test"),
        };

        self.compare_errors_against_expected(errors);
        self.compare_stderr_against_expected();

        match self.options.execution_mode {
            None => {
                if self.options.expected_errors.is_empty() {
                    panic!("no expected errors found, but no `//~ mode` comment")
                }
            }
            Some(ExecutionMode::Compile) => {}
            Some(ExecutionMode::Executable) => {
                self.run_executable();
            }
            Some(ExecutionMode::Eval) => {
                self.run_eval();
            }
            Some(ExecutionMode::All) => {
                self.run_executable();
                self.run_eval();
            }
        }
    }
}
