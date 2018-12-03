use lark_parser::ParserDatabaseExt;
use lark_query_system::LarkDatabase;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

mod options;
use options::TestOptions;

mod test;
use test::TestContext;

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

/// Runs the test harness against a given test file:
///
/// - `base_path` -- the path where test files are found (`tests/test_files`)
/// - `test_path` -- path to an individual file (`tests/test_files/foo/bar.lark`)
/// - `is_dir` -- true if `test_path` is a directory
/// - `bless_mode` -- if true, generate reference files with content
pub fn run_test_harness(
    base_path: impl AsRef<Path>,
    test_path: impl AsRef<Path>,
    is_dir: bool,
    bless_mode: bool,
) {
    let base_path: &Path = base_path.as_ref();
    let test_path: &Path = test_path.as_ref();

    if is_dir {
        // this is meant to be a manifest tes
        unimplemented!("test directories");
    }

    eprintln!("Test file: `{}`", test_path.display());

    let relative_test_path = test_path.strip_prefix(base_path).unwrap_or_else(|err| {
        panic!(
            "failed to strip prefix `{}` from `{}`: {}",
            base_path.display(),
            test_path.display(),
            err
        )
    });

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
