use crate::harness::test::TestContext;
use lark_cli::build::LarkDatabaseExt;
use lark_query_system::ls_ops::Cancelled;
use std::process::Command;

impl TestContext<'_> {
    crate fn run_executable(&self) {
        let exe_path = self.executable_path();
        self.db
            .build(exe_path.to_str().unwrap())
            .unwrap_or_else(|Cancelled| panic!("cancelled"));

        let cmd = Command::new(exe_path)
            .output()
            .expect("Failed to run compile test");
        let test_output = String::from_utf8(cmd.stdout).unwrap();

        self.compare_reference_contents("exe", test_output.as_bytes());
    }

    crate fn run_eval(&self) {
        let mut handler = lark_eval::IOHandler::new(true);
        lark_eval::eval(&self.db, &mut handler);
        let lark_eval::IOHandler { redirect: output } = handler;
        let output = output.unwrap();
        self.compare_reference_contents("eval", output.as_bytes());
    }
}
