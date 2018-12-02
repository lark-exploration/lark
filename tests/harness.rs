use lark_test;

#[test]
fn bad_identifier() {
    lark_test::run_test_harness("tests/test_files/type_checker/bad_identifier.lark", false);
}

#[test]
fn bad_callee() {
    lark_test::run_test_harness("tests/test_files/type_checker/bad_callee.lark", false);
}
