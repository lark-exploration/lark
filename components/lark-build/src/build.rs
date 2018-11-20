use crate::CodegenType;

/// Build a source file using the default tools on the given platform
pub fn build(
    target_filename: &str,
    src: &String,
    codegen_type: CodegenType,
) -> std::io::Result<()> {
    match codegen_type {
        CodegenType::Rust => build_rust(target_filename, src),
    }
}

/// Create a temporary file we can write the source into for compilation
fn create_src_file(codegen_type: CodegenType) -> tempfile::NamedTempFile {
    let temp_file = match codegen_type {
        CodegenType::Rust => tempfile::NamedTempFileOptions::new()
            .prefix("lark")
            .suffix(".rs")
            .rand_bytes(6)
            .create()
            .unwrap(),
        /*
        CodegenType::C => tempfile::NamedTempFileOptions::new()
            .prefix("lark")
            .suffix(".c")
            .rand_bytes(6)
            .create()
            .unwrap(),
        */
    };

    temp_file
}

/// Invoke the Rust compiler to build the source file
fn build_rust(target_filename: &str, src: &String) -> std::io::Result<()> {
    use std::io::Write;
    use std::process::Command;

    let mut src_file = create_src_file(CodegenType::Rust);
    src_file.write_all(src.as_bytes()).unwrap();
    let src_file_name = src_file.path().to_string_lossy().to_string();

    let output = Command::new(r"rustc")
        .arg(src_file_name)
        .arg("-o")
        .arg(target_filename)
        .output()
        .expect("Failed to run Rust compiler");

    if output.status.success() {
        Ok(())
    } else {
        use std::io::{Error, ErrorKind};

        let compile_stdout = String::from_utf8(output.stdout).unwrap();
        let compile_stderr = String::from_utf8(output.stderr).unwrap();

        let combined_compile_msg = compile_stdout + &compile_stderr;

        Err(Error::new(ErrorKind::Other, combined_compile_msg))
    }
}
