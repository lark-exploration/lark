use crate::parser::Parser;
use crate::syntax::sigil::Comma;
use crate::syntax::Syntax;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_seq::Seq;

#[derive(DebugWith)]
pub struct CommaList<T>(pub T);

impl<T> CommaList<T> {
    fn element(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> Syntax<'parse> for CommaList<T>
where
    T: Syntax<'parse>,
{
    type Data = Seq<T::Data>;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        SeparatedList(self.element(), Comma).test(parser)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Seq<T::Data>, ErrorReported> {
        SeparatedList(self.element(), Comma).expect(parser)
    }
}

/// Parses a "list" of things. In general, lists in Lark can be
/// separated either by some given sigil (the `S`) *or* by a newline
/// (or both). Expects to be called immediately after the "opening
/// sigil" of the list (typically a `(` or `{`). Invokes
/// `parse_element_fn` to parse each element of the list.
///
/// Example of something we might parse:
///
/// ```ignore
/// Foo {
///      ^ cursor is here when we are called, on the newline (say)
///   a: int, b: uint
///   c: uint
///   d: uint,
///   e: uint
/// }
/// ^ cursor will be here when we return
/// ```
#[derive(DebugWith)]
pub struct SeparatedList<T, S>(pub T, pub S);

impl<T, S> SeparatedList<T, S> {
    fn element(&self) -> &T {
        &self.0
    }

    fn separator(&self) -> &S {
        &self.1
    }
}

impl<T, S> Syntax<'parse> for SeparatedList<T, S>
where
    T: Syntax<'parse>,
    S: Syntax<'parse>,
{
    type Data = Seq<T::Data>;

    fn test(&mut self, _parser: &Parser<'parse>) -> bool {
        true // we never produce an error
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Seq<T::Data>, ErrorReported> {
        let SeparatedList(element, delimiter) = self;

        let mut result = vec![];
        parser.skip_newlines();
        loop {
            if let Some(element) = parser.parse_if_present(&mut *element) {
                result.push(element?);

                if let Some(_) = parser.parse_if_present(&mut *delimiter) {
                    parser.skip_newlines();
                    continue;
                } else if parser.skip_newlines() {
                    continue;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(Seq::from(result))
    }
}
