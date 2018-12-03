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

pub fn run_test_harness(path: impl AsRef<Path>, is_dir: bool, bless_mode: bool) {
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
        bless_mode,
        test_name,
        test_path,
        db,
        options,
    }
    .execute();
}
