#![cfg(test)]

use crate::harness::run_test_harness;

#[test]
#[should_panic(expected = "no expected errors found, but no `execute` comment")]
fn no_error_no_mode() {
    run_test_harness(
        "no_error_no_mode.lark",
        "self_tests/no_error_no_mode.lark",
        false,
        false,
    );
}
