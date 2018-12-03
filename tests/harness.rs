use lark_test;

#[test]
fn bad_identifier() {
    lark_test::run_test_harness(
        "type_checker/bad_identifier.lark",
        "tests/test_files/type_checker/bad_identifier.lark",
        false,
        std::env::var("LARK_BLESS").is_ok(),
    );
}

#[test]
fn bad_callee() {
    lark_test::run_test_harness(
        "type_checker/bad_callee.lark",
        "tests/test_files/type_checker/bad_callee.lark",
        false,
        std::env::var("LARK_BLESS").is_ok(),
    );
}

#[test]
fn test_true() {
    lark_test::run_test_harness(
        "true.lark",
        "tests/test_files/true.lark",
        false,
        std::env::var("LARK_BLESS").is_ok(),
    );
}
