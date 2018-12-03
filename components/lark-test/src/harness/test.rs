use crate::harness::options::TestMode;
use crate::harness::options::TestOptions;
use crate::harness::TestPath;
use lark_query_system::LarkDatabase;

mod compilation_test;
mod execution_test;
mod util;

crate struct TestContext<'me> {
    crate bless_mode: bool,
    crate test_name: String,
    crate test_path: &'me TestPath,
    crate db: LarkDatabase,
    crate options: TestOptions,
}

impl TestContext<'_> {
    crate fn execute(self) {
        match self.options.mode {
            TestMode::Compilation { error } => self.run_compilation_test(error),
            TestMode::Execute => self.run_execute_test(),
        }
    }
}
