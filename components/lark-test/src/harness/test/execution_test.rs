use crate::harness::test::TestContext;
use lark_cli::build::LarkDatabaseExt;
use lark_query_system::ls_ops::Cancelled;
use std::process::Command;

impl TestContext<'_> {
    crate fn build_and_run_executable(&self) {
        let exe_path = self.executable_path();
        self.db
            .build(exe_path.to_str().unwrap())
            .unwrap_or_else(|Cancelled| panic!("cancelled"));

        let cmd = Command::new(exe_path)
            .output()
            .expect("Failed to run compile test");
        let test_output = String::from_utf8(cmd.stdout).unwrap();

        self.compare_reference_contents("output", test_output.as_bytes(), false);
    }

    crate fn run_eval(&self) {
        let mut handler = lark_eval_hir::IOHandler::new(true);
        lark_eval_hir::eval(&self.db, &mut handler);
        let lark_eval_hir::IOHandler { redirect: output } = handler;
        let output = output.unwrap();
        self.compare_reference_contents("output", output.as_bytes(), false);
    }
}
