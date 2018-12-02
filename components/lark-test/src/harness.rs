use lark_parser::ParserDatabaseExt;
use lark_query_system::ls_ops::Cancelled;
use lark_query_system::ls_ops::LsDatabase;
use lark_query_system::LarkDatabase;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

mod options;
use options::TestMode;
use options::TestOptions;

pub struct TestPath {
    pub path: PathBuf,
    pub is_dir: bool,
}

pub fn search_files(root_path: &Path) -> Vec<TestPath> {
    let mut iterator = WalkDir::new(root_path).into_iter();

    // Search for all files (or directories!) named `.lark`.  Those
    // are the tests. Skip over other directories and ignore files
    // with other extensions.
    let mut test_paths = vec![];

    while let Some(entry) = iterator.next() {
        let entry = match entry {
            Err(_) => continue,
            Ok(entry) => entry,
        };

        let entry_path = entry.path();

        let extension = match entry_path.extension() {
            None => continue,
            Some(extension) => extension,
        };

        if extension == "lark" {
            let is_dir = entry.file_type().is_dir();

            if is_dir {
                iterator.skip_current_dir();
            }

            test_paths.push(TestPath {
                path: entry.into_path(),
                is_dir,
            });
        }
    }

    test_paths
}

pub fn run_test_harness(path: impl AsRef<Path>, is_dir: bool) {
    if is_dir {
        // this is meant to be a manifest tes
        unimplemented!("test directories");
    }

    let test_path = &TestPath {
        path: path.as_ref().to_owned(),
        is_dir,
    };

    eprintln!("Test file: `{}`", test_path.path.display());

    let test_name = test_path
        .path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let file_contents = fs::read_to_string(&test_path.path)
        .unwrap_or_else(|err| panic!("error reading `{}`: {}", test_path.path.display(), err));

    let options = TestOptions::from_source_text(&test_path.path, &file_contents);

    eprintln!("Options: {:?}", options);

    if let Some(explanation) = &options.skip_test {
        eprintln!("Skipped: {}", explanation);
        return;
    }

    let mut db = LarkDatabase::default();
    db.add_file(&test_name, &file_contents);

    TestContext {
        test_name,
        test_path,
        db,
        options,
    }
    .execute();
}

struct TestContext<'me> {
    test_name: String,
    test_path: &'me TestPath,
    db: LarkDatabase,
    options: TestOptions,
}

impl TestContext<'_> {
    fn execute(self) {
        match self.options.mode {
            TestMode::Compilation { error } => self.run_compilation_test(error),
            TestMode::Execute => self.run_execute_test(),
        }
    }

    fn run_compilation_test(&self, expect_error: bool) {
        if expect_error && self.options.expected_errors.is_empty() {
            panic!("no errors specified -- consider using `mode:compile_pass`")
        }

        let errors = match self.db.errors_for_project() {
            Ok(errors) => errors,
            Err(Cancelled) => panic!("encountered cancellation in unit test"),
        };

        // Compute the errors to the ones we expect to see.
        let mut expected_errors: Vec<_> = self.options.expected_errors.iter().collect();
        let mut unexpected_errors = vec![];
        for (file_name, errors) in errors {
            if file_name != self.test_name {
                unexpected_errors.extend(errors);
                continue;
            }

            for error in errors {
                let matching_expected_error = expected_errors.iter().position(|ee| {
                    ee.line_num == error.range.start.line && ee.message.is_match(&error.label)
                });

                if let Some(i) = matching_expected_error {
                    expected_errors.remove(i);
                } else {
                    unexpected_errors.push(error);
                }
            }
        }

        if !unexpected_errors.is_empty() {
            eprintln!("# Unexpected errors");
            for error in &unexpected_errors {
                eprintln!(
                    "{}:{}:{}:{}:{}: {}",
                    self.test_path.path.display(),
                    error.range.start.line + 1,
                    error.range.start.character + 1,
                    error.range.end.line + 1,
                    error.range.end.character + 1,
                    error.label,
                );
            }
        }

        if !expected_errors.is_empty() {
            eprintln!("# Expected errors not found");
            for error in &expected_errors {
                eprintln!(
                    "{}:{}: something matching `{}`",
                    self.test_path.path.display(),
                    error.line_num + 1,
                    error.message,
                );
            }
        }

        assert!(unexpected_errors.is_empty() && expected_errors.is_empty());
    }

    fn run_execute_test(&self) {
        unimplemented!()
    }
}
