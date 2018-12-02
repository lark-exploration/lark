use regex::Regex;
use std::path::Path;

#[derive(Debug, Default)]
crate struct TestOptions {
    crate mode: TestMode,
    crate skip_test: Option<String>,
    crate expected_errors: Vec<ExpectedError>,
}

#[derive(Debug)]
crate enum TestMode {
    Compilation { error: bool },
    Execute,
}

impl Default for TestMode {
    fn default() -> Self {
        TestMode::Compilation { error: true }
    }
}

#[derive(Clone, Debug)]
crate struct ExpectedError {
    crate line_num: u64,
    crate message: Regex,
}

lazy_static::lazy_static! {
    static ref WITH_OPTION: Regex = Regex::new(r"\s*//~ ([a-zA-Z_]+):(.*)").unwrap();
    static ref NO_OPTION: Regex = Regex::new(r"\s*//~ ([a-zA-Z_]+)\s*$").unwrap();
}

impl TestOptions {
    /// Parse the `//~` options from the source text `text`.
    ///
    /// Panic if something is wrong.
    crate fn from_source_text(path: &Path, text: &str) -> Self {
        let mut result = TestOptions::default();
        let mut last_non_comment_line = None;

        for (line, line_num) in text.lines().zip(0..) {
            let error = if let Some(cap) = WITH_OPTION.captures(line) {
                result.apply_comment(&cap[1], cap[2].trim(), last_non_comment_line)
            } else if let Some(cap) = NO_OPTION.captures(line) {
                result.apply_comment(&cap[1], "", last_non_comment_line)
            } else if line.contains("//~") {
                Err("`//~` comments must appear alone".to_string())
            } else {
                let line_trim = line.trim();
                if !line_trim.is_empty() && !line_trim.starts_with("//") {
                    last_non_comment_line = Some(line_num);
                }
                Ok(())
            };

            match error {
                Ok(()) => {}

                Err(err) => {
                    eprintln!("{}:{}: {}", path.display(), line_num + 1, err);
                    panic!("illegal test file `{}`", path.display());
                }
            }
        }

        result
    }

    // Applies a comment `// key: value` found in a lark test file.
    //
    // Returns false if the comment was not recognized.
    crate fn apply_comment(
        &mut self,
        key: &str,
        value: &str,
        last_non_comment_line: Option<u64>,
    ) -> Result<(), String> {
        match key {
            "skip_test" => {
                if value.trim().is_empty() {
                    Err("skip_test requires an explanation of why the test is skipped".to_string())
                } else {
                    self.skip_test = Some(value.trim().to_string());
                    Ok(())
                }
            }

            "mode" => {
                self.mode = match value.trim() {
                    "compile_fail" => TestMode::Compilation { error: true },
                    "compile_pass" => TestMode::Compilation { error: false },
                    "run_pass" => TestMode::Execute,
                    _ => return Err(format!("unexpected compilation mode: `{}`", value.trim())),
                };
                Ok(())
            }

            "ERROR" => match last_non_comment_line {
                None => Err("cannot find line that error applies to".to_string()),
                Some(line_num) => match Regex::new(value.trim()) {
                    Ok(message) => {
                        self.expected_errors
                            .push(ExpectedError { line_num, message });
                        Ok(())
                    }
                    Err(error) => Err(format!("illegal regular expression `{}`", error)),
                },
            },

            _ => Err(format!("unknown option `{}`", key)),
        }
    }
}
