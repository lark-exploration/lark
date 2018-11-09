#![cfg(test)]

use crate::lexer::definition::LexerState;
use crate::lexer::tools::Tokenizer;
use crate::span::CurrentFile;
use crate::span::Span;
use lark_error::Diagnostic;

use unindent::unindent;

/// The "specification" consists of alternating source lines
/// and annotation lines. An annotation line has this format:
///
/// ```ignore
/// <Spans> Token0 Token1 ... TokenN
/// ```
///
/// The `TokenN` things are the names of token kinds. The spans are digits
/// like `0011222`, which will indicate the spans of each token. So e.g.
/// `Token0` would be the first two character (the ones covered by `00`).
fn process(specification: &str) -> Result<(), Diagnostic> {
    let mut lines = specification.lines();

    while let Some(source_line) = lines.next() {
        println!("source_line = {:?}", source_line);
        let specification_line = lines.next().unwrap_or_else(|| panic!("missing spec line"));
        println!("specification_line = {:?}", specification_line);

        let mut tokens: Tokenizer<LexerState> = Tokenizer::new(source_line);

        let mut spec_words = specification_line.split(char::is_whitespace);
        if let Some(span_word) = spec_words.next() {
            for (index, token_kind) in spec_words.enumerate() {
                let ch = std::char::from_digit(index as u32, 10).unwrap();
                let start_index = span_word.find(ch).unwrap_or_else(|| {
                    panic!(
                        "bad specification {:?}: no start for {}",
                        specification_line, index
                    )
                });
                let end_index = span_word.rfind(ch).unwrap() + 1;
                let span = Span::new(CurrentFile, start_index, end_index);

                let token = tokens.next().unwrap()?;
                assert_eq!(
                    token_kind,
                    format!("{:?}", token.value),
                    "token {} has wrong kind",
                    index
                );
                assert_eq!(span, token.span, "token {} has wrong span", index);
            }
        }
    }

    Ok(())
}

#[test]
fn test_quicklex() -> Result<(), Diagnostic> {
    let source = unindent(
        r##"
            struct Diagnostic {
            0000001222222222234 Identifier Whitespace Identifier Whitespace Sigil
              msg: String,
            00111234444445 Whitespace Identifier Sigil Whitespace Identifier Sigil
              level: String,
            0011111234444445 Whitespace Identifier Sigil Whitespace Identifier Sigil
            }
            0 Sigil
            "##,
    );

    process(&source)?;

    Ok(())
}
