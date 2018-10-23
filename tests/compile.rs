mod common;

#[cfg(test)]
mod tests {
    use lark_codegen::{build, codegen, CodegenType};
    use lark_mir::{Context, DefId};
    use std::process::Command;

    fn run_compile_test(
        filename: &str,
        codegen_type: CodegenType,
        full_context: (Context, DefId),
        expected: &str,
    ) {
        let src = codegen(&full_context.0, codegen_type);

        let mut out_path = std::env::temp_dir();
        out_path.push(filename);

        let result = build(out_path.to_str().unwrap(), &src, codegen_type);
        result.unwrap();
        let output = Command::new(out_path.to_str().unwrap())
            .output()
            .expect("Failed to run test binary");

        let output_stdout = String::from_utf8_lossy(&output.stdout);

        assert_eq!(output_stdout, expected);
    }

    #[test]
    fn build_big_test_in_rust() {
        run_compile_test(
            "big_test_in_rust",
            CodegenType::Rust,
            crate::common::generate_big_test(),
            "3\n18\n",
        );
    }

    #[test]
    fn build_simple_add_test_in_rust() {
        run_compile_test(
            "simple_add_test_in_rust",
            CodegenType::Rust,
            crate::common::generate_simple_add_test(),
            "18\n",
        );
    }

    #[cfg(windows)]
    #[test]
    fn build_simple_add_test_in_c() {
        run_compile_test(
            "simple_add_test_in_c",
            CodegenType::C,
            crate::common::generate_simple_add_test(),
            "18\r\n",
        );
    }

    #[cfg(unix)]
    #[test]
    fn build_simple_add_test_in_c() {
        run_compile_test(
            "simple_add_test_in_c",
            CodegenType::C,
            crate::common::generate_simple_add_test(),
            "18\n",
        );
    }

    #[cfg(windows)]
    #[test]
    fn build_big_test_in_c() {
        run_compile_test(
            "big_test_in_c",
            CodegenType::C,
            crate::common::generate_big_test(),
            "3\r\n18\r\n",
        );
    }

    #[cfg(unix)]
    #[test]
    fn build_big_test_in_c() {
        run_compile_test(
            "big_test_in_c",
            CodegenType::C,
            crate::common::generate_big_test(),
            "3\n18\n",
        );
    }
}
