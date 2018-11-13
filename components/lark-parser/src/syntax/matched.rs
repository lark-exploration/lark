use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::Delimiter;
use crate::syntax::NonEmptySyntax;
use crate::syntax::Syntax;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;

/// Some sequence of tokens that begins with an open delimiter and
/// ends with a (matched) close delimiter. The tokens in between are
/// not (yet) parsed.
#[derive(DebugWith)]
pub struct Matched<D>(pub D);

impl<D> Matched<D> {
    fn delimiters(&self) -> &D {
        &self.0
    }
}

impl<D> Syntax for Matched<D>
where
    D: Delimiter,
{
    type Data = Spanned<()>;

    fn test(&self, parser: &Parser<'_>) -> bool {
        parser.test(self.delimiters().open_syntax())
    }

    fn expect(&self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        let open_syntax = self.delimiters().open_syntax();
        let close_syntax = self.delimiters().close_syntax();

        let start_span = parser.peek_span();
        parser.expect(&open_syntax)?;

        let mut counter = 1;
        loop {
            if let Some(_) = parser.parse_if_present(&open_syntax) {
                counter += 1;
            } else if let Some(_) = parser.parse_if_present(&close_syntax) {
                counter -= 1;
                if counter == 0 {
                    break;
                }
            } else {
                parser.shift();
            }
        }

        let full_span = start_span.extended_until_end_of(parser.last_span());
        Ok(Spanned::new((), full_span))
    }
}

impl<D> NonEmptySyntax for Matched<D> where D: Delimiter {}
