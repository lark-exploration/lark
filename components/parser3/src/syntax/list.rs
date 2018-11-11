use crate::parser::Parser;
use crate::syntax::InfallibleSyntax;
use crate::syntax::Syntax;

pub struct CommaList<T> {
    data: Vec<T>,
}

impl<T> InfallibleSyntax for CommaList<T>
where
    T: Syntax,
{
    type Data = Vec<T::Data>;

    fn singular_name() -> String {
        T::plural_name()
    }

    fn plural_name() -> String {
        T::plural_name()
    }

    fn parse_infallible(parser: &mut Parser<'_>) -> Vec<T::Data> {
        parse_list::<T>(parser, ",")
    }
}

/// Parses a "list" of things. In general, lists in Lark can be
/// separated either by some given sigil *or* by a newline (or
/// both). Expects to be called immediately after the "opening
/// sigil" of the list (typically a `(` or `{`). Invokes
/// `parse_element_fn` to parse each element of the list.
///
/// Example of something we might parse:
///
/// ```
/// Foo {
///      ^ cursor is here when we are called, on the newline (say)
///   a: int, b: uint
///   c: uint
///   d: uint,
///   e: uint
/// }
/// ^ cursor will be here when we return
/// ```
fn parse_list<T>(parser: &mut Parser<'_>, separator_sigil: &str) -> Vec<T::Data>
where
    T: Syntax,
{
    let mut result = vec![];
    parser.eat_newlines();
    loop {
        if let Some(element) = parser.eat_syntax::<T>() {
            result.push(element);

            if let Some(_) = parser.eat_sigil(separator_sigil) {
                parser.eat_newlines();
                continue;
            } else if parser.eat_newlines() {
                continue;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
}
