use lark_test::*;

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

#[test]
fn struct_field_int_vs_uint() {
    run_test(
        "struct Foo { f: int } def get(a: Foo) -> uint { a.f }",
        "                                                ~~~",
    );
}

#[test]
fn struct_field_uint_vs_uint() {
    run_test(
        "struct Foo { f: uint } def get(a: Foo) -> uint { a.f }",
        NoErrors,
    );
}

#[test]
fn struct_ctor_correct_arg_type() {
    run_test(
        "struct Foo { b: bool } def make() -> Foo { Foo(b: true) }",
        NoErrors,
    );
}

#[test]
fn struct_ctor_wrong_arg_type() {
    run_test(
        "struct Foo { b: bool } def make() -> Foo { Foo(b: 22) }",
        "                                                  ~~",
    );
}

#[test]
fn binary_operator_plus_uint_uint_uint() {
    run_test("def add(a: uint, b: uint) -> uint { a + b }", NoErrors);
}

#[test]
fn binary_operator_plus_int_int_int() {
    run_test("def add(a: int, b: int) -> int { a + b }", NoErrors);
}

#[test]
fn binary_operator_plus_int_int_uint() {
    run_test(
        "def add(a: int, b: int) -> uint { a + b }",
        "                                  ~~~~~",
    );
}

#[test]
fn binary_operator_int_eq_int_int() {
    run_test(
        "def add(a: int, b: int) -> int { a == b }",
        "                                 ~~~~~~",
    );
}
