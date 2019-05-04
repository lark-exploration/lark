use lark_parser::ParserDatabase;
use lark_span::ByteIndex;
use lark_test::*;

#[test]
fn location() {
    let file_name = "foo.lark";
    let db = db_with_test(file_name, "abc\ndef\n\ng");
    //                                0123 4567 8 9

    // Check the start of a line:
    let file_name = file_name.into_file_name(&db);
    let loc_4 = db.location(file_name, ByteIndex::from(4));
    assert_expected_debug(
        &(),
        &unindent::unindent(
            "Location {
                 line: 1
                 column: 0
                 byte: ByteIndex(
                     4
                 )
             }",
        ),
        &loc_4,
    );

    // Check the start of a line:
    let file_name = file_name.into_file_name(&db);
    let loc_4 = db.location(file_name, ByteIndex::from(5));
    assert_expected_debug(
        &(),
        &unindent::unindent(
            "Location {
                 line: 1
                 column: 1
                 byte: ByteIndex(
                     5
                 ),
             }",
        ),
        &loc_4,
    );
}
