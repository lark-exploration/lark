use lark_test::*;

#[test]
fn bad_identifier() {
    run_test(
        "def new(msg: bool,) -> bool { msg1 }",
        "                              ~~~~",
    );
}

#[test]
fn bad_callee() {
    run_test(
        "def foo(msg: bool,) -> bool { bar(msg) }",
        "                              ~~~",
    );
}

#[test]
fn correct_call() {
    run_test(
        "def foo(msg: bool,) { bar(msg) } def bar(arg:bool,) { }",
        NoErrors,
    );
}

#[test]
fn wrong_num_of_arguments() {
    run_test(
        "def foo(msg: bool,) -> bool { bar(msg) } def bar(arg:bool, arg2:bool) { }",
        "                              ~~~~~~~~",
    );
}

#[test]
fn wrong_return_type() {
    // `bar` returns unit, we expect `bool`
    run_test(
        "def foo(msg: bool,) -> bool { bar(msg) } def bar(arg:bool,) { }",
        "                              ~~~~~~~~",
    );
}

#[test]
fn wrong_type_of_arguments() {
    run_test(
        "def foo(msg: int,) { bar(msg) } def bar(arg:bool,) { }",
        "                         ~~~",
    );
}
