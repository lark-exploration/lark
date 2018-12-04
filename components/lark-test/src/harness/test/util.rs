use crate::harness::test::TestContext;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

impl TestContext<'_> {
    /// Creates a path referring to the test file but with
    /// a different extension. For example, if we are testing
    /// `foo.lark`, and we invoke this with `stderr`, would
    /// create `foo.stderr`.
    crate fn reference_path(&self, extension: &str) -> PathBuf {
        self.test_path.with_extension(extension)
    }

    crate fn executable_path(&self) -> PathBuf {
        if cfg!(windows) {
            self.output_path("exe")
        } else {
            self.output_path("")
        }
    }

    /// Generates a path for an output file with the given extension.
    crate fn output_path(&self, extension: &str) -> PathBuf {
        // For now, use `/tmp`, but it would be better I think to use
        // the cargo target directory perhaps?

        let dir = std::env::temp_dir();
        let out_path = std::path::Path::new(self.relative_test_path).with_extension(extension);
        dir.join(out_path)
    }

    /// Load the contents of the reference file with the given extension.
    crate fn file_contents(&self, path: impl AsRef<Path>) -> Option<String> {
        let path: &Path = path.as_ref();

        if !path.exists() {
            return None;
        }

        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read `{}`: {}", path.display(), err));

        Some(contents)
    }

    crate fn compare_reference_contents(
        &self,
        extension: &str,
        actual_bytes: &[u8],
        normalize_paths: bool,
    ) {
        let mut actual_str = match std::str::from_utf8(actual_bytes) {
            Ok(s) => s.to_string(),
            Err(utf8_error) => panic!("actual output not utf8: `{}`", utf8_error),
        };

        if normalize_paths {
            actual_str = actual_str.replace("\\", "/");
        }
        actual_str = actual_str.replace("\r\n", "\n");

        let reference_path = self.reference_path(extension);

        if self.bless_mode {
            match fs::write(&reference_path, actual_bytes) {
                Ok(()) => {}
                Err(err) => panic!("failed to write `{}`: {}", reference_path.display(), err,),
            }
        }

        let reference_contents = self.file_contents(&reference_path);

        let mut reference_str = reference_contents.unwrap_or(String::new());
        reference_str = reference_str.replace("\r\n", "\n");

        if actual_str != reference_str {
            let mut first_diff = None;
            for (diff, i) in diff::lines(&reference_str, &actual_str).iter().zip(1..) {
                match diff {
                    diff::Result::Left(l) => eprintln!("-{}", l),
                    diff::Result::Both(l, _) => eprintln!(" {}", l),
                    diff::Result::Right(r) => eprintln!("+{}", r),
                }

                if first_diff.is_none() {
                    match diff {
                        diff::Result::Left(_) | diff::Result::Right(_) => {
                            first_diff = Some(i);
                        }
                        diff::Result::Both(..) => {}
                    }
                }
            }

            eprintln!(
                "{}:{}: file is not as expected",
                reference_path.display(),
                first_diff.unwrap(),
            );

            panic!(
                "contents of `{}` are not as expected",
                reference_path.display()
            );
        }
    }
}
