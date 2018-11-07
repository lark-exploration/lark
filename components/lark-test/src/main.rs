use ast::AstDatabase;
use lark_hir::HirDatabase;
use lark_query_system::LarkDatabase;
use parser::HasParserState;
use parser::HasReaderState;

fn run_test(text: &str, span: &str) {
    let mut db = LarkDatabase::default();
    let path1_str = "path1";
    let path1_interned = db.intern_string("path1");

    db.add_file(path1_str, text);

    let items_in_file = db.items_in_file(path1_interned);
    assert_eq!(items_in_file.len(), 1, "input with more than one item");

    let entity = items_in_file[0];
    let hir_with_errors = db.fn_body(entity);

    assert_eq!(
        hir_with_errors.errors.len(),
        1,
        "input with more than one error"
    );

    // total hack: we know that the byte index will be relative to the
    // start of the string
    let expected_range = format!("{}..{}", span.find('~').unwrap() + 1, span.len() + 1);

    let error_span = hir_with_errors.errors[0].span;
    assert_eq!(expected_range, format!("{:?}", error_span));
}

#[test]
fn bad_identifier() {
    run_test(
        "def new(msg: bool,) -> bool { msg1 }",
        "                              ~~~~",
    );
}
