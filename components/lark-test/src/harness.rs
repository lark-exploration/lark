use lark_parser::ParserDatabaseExt;
use lark_query_system::LarkDatabase;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

mod options;
use options::TestOptions;

mod self_test;

mod test;
use test::TestContext;

pub struct TestPath {
    pub relative_test_path: PathBuf,
    pub test_path: PathBuf,
    pub is_dir: bool,
}

pub fn search_files(root_path: impl AsRef<Path>) -> Vec<TestPath> {
    let root_path: &Path = root_path.as_ref();

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

            let test_path = entry.into_path();

            let relative_test_path = test_path
                .strip_prefix(root_path)
                .unwrap_or_else(|err| {
                    panic!(
                        "failed to strip prefix `{}` from `{}`: {}",
                        root_path.display(),
                        test_path.display(),
                        err
                    )
                })
                .to_owned();

            test_paths.push(TestPath {
                test_path,
                relative_test_path,
                is_dir,
            });
        }
    }

    test_paths
}

/// Runs the test harness against a given test file. The first few arguments
/// are the fields from `TestPath`.
pub fn run_test_harness(
    relative_test_path: impl AsRef<Path>,
    test_path: impl AsRef<Path>,
    is_dir: bool,
    bless_mode: bool,
) {
    let relative_test_path: &Path = relative_test_path.as_ref();
    let test_path: &Path = test_path.as_ref();

    if is_dir {
        // this is meant to be a manifest tes
        unimplemented!("test directories");
    }

    eprintln!("Test file: `{}`", test_path.display());

    let test_name = relative_test_path.with_extension("").display().to_string();

    let file_contents = fs::read_to_string(&test_path)
        .unwrap_or_else(|err| panic!("error reading `{}`: {}", test_path.display(), err));

    let options = TestOptions::from_source_text(&test_path, &file_contents);

    eprintln!("Options: {:?}", options);

    if let Some(explanation) = &options.skip_test {
        eprintln!("Skipped: {}", explanation);
        return;
    }

    let mut db = LarkDatabase::default();
    db.add_file(&test_name, &file_contents);

    TestContext {
        bless_mode,
        test_name,
        test_path,
        relative_test_path: &relative_test_path,
        db,
        options,
    }
    .execute();
}
