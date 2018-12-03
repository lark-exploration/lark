extern crate proc_macro;
use lark_test::TestPath;
use std::fmt::Write;
use std::path::PathBuf;

use proc_macro::TokenStream;

#[proc_macro]
pub fn generate_tests(_item: TokenStream) -> TokenStream {
    let mut result = String::new();

    let base_path = PathBuf::from("tests/test_files");

    for TestPath {
        relative_test_path,
        test_path,
        is_dir,
    } in lark_test::search_files(&base_path)
    {
        // TODO -- make this `foo::bar` -- maybe move the `search_files` logic
        // into here?

        // "foo/bar_baz.lark" becomes `foo__bar_baz`
        let test_name = relative_test_path
            .with_extension("")
            .display()
            .to_string()
            .replace("/", "__");

        write!(
            result,
            "{}",
            unindent::unindent(&format!(
                r#"
                    #[test]
                    fn r#{test_name}() {{
                        lark_test::run_test_harness(
                            {relative_test_path:?},
                            {test_path:?},
                            {is_dir:?},
                            std::env::var("LARK_BLESS").is_ok(),
                        );
                    }}
                "#,
                test_name = test_name,
                relative_test_path = relative_test_path.to_str().unwrap(),
                test_path = test_path.to_str().unwrap(),
                is_dir = is_dir,
            )),
        )
        .unwrap();
    }

    result.parse().unwrap()
}
