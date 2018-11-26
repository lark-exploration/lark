use std::env;
use std::process::Command;

fn eval_test(fname: &str, expected_value: &str) {
    let cmd = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("run")
        .arg(fname)
        .output()
        .expect("Failed to spawn child process");
    let test_output = String::from_utf8(cmd.stdout).unwrap();

    assert_eq!(expected_value, test_output.trim());
}

fn build_test(fname: &str, expected_value: &str) {
    let mut dir = env::temp_dir();

    let out_file = if cfg!(windows) {
        std::path::Path::new(fname).with_extension("exe")
    } else {
        std::path::Path::new(fname).with_extension("")
    };

    dir.push(out_file.file_name().unwrap());

    let out_fname = dir.to_str().unwrap();

    let cmd = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("build")
        .arg(fname)
        .arg(out_fname)
        .output()
        .expect("Failed to spawn child process");
    assert!(cmd.status.success());

    let cmd = Command::new(out_fname)
        .output()
        .expect("Failed to run compile test");

    let test_output = String::from_utf8(cmd.stdout).unwrap();

    assert_eq!(expected_value, test_output.trim());
}

fn run_eval_and_build_test(fname: &str, expected_value: &str) {
    eval_test(fname, expected_value);
    build_test(fname, expected_value);
}

#[test]
fn test_true() {
    run_eval_and_build_test("tests/test_files/true.lark", "true");
}

#[test]
fn test_assign_variable() {
    run_eval_and_build_test("tests/test_files/assign_variable.lark", "true");
}

#[test]
fn test_call_in_call() {
    run_eval_and_build_test("tests/test_files/call_in_call.lark", "false");
}

#[test]
fn test_call() {
    run_eval_and_build_test("tests/test_files/call.lark", "false");
}

#[test]
fn test_multi_statement() {
    run_eval_and_build_test("tests/test_files/multi_statement.lark", "false\ntrue");
}

#[test]
fn test_struct_init() {
    run_eval_and_build_test("tests/test_files/struct_init.lark", "true");
}
