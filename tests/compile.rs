mod common;

#[cfg(test)]
mod tests {
    use lark_codegen::{build, codegen, CodegenType};
    use std::process::Command;

    #[test]
    fn build_big_test_in_rust() {
        let (context, _) = crate::common::generate_big_test();
        let src = codegen(&context, CodegenType::Rust);

        let mut out_path = std::env::temp_dir();
        out_path.push("codegen_simple");

        let result = build(out_path.to_str().unwrap(), &src, CodegenType::Rust);
        result.unwrap();
        let output = Command::new(out_path.to_str().unwrap())
            .output()
            .expect("Failed to run test binary");

        let output_stdout = String::from_utf8_lossy(&output.stdout);

        assert_eq!(output_stdout, "\"Hello, world 3\"\n18\n");
    }
}
