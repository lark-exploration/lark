use crate::parser::Parser;
use crate::syntax::sigil::Comma;
use crate::syntax::Syntax;

pub struct CommaList<T>(pub T);

impl<T> CommaList<T> {
    fn element(&self) -> &T {
        &self.0
    }
}

impl<T> Syntax for CommaList<T>
where
    T: Syntax,
{
    type Data = Vec<T::Data>;

    fn singular_name(&self) -> String {
        SeparatedList(self.element(), Comma).singular_name()
    }

    fn plural_name(&self) -> String {
        SeparatedList(self.element(), Comma).plural_name()
    }

    fn parse(&self, parser: &mut Parser<'_>) -> Option<Vec<T::Data>> {
        SeparatedList(self.element(), Comma).parse(parser)
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
pub struct SeparatedList<T, S>(pub T, pub S);

impl<T, S> SeparatedList<T, S> {
    fn element(&self) -> &T {
        &self.0
    }

    fn separator(&self) -> &S {
        &self.1
    }
}

impl<T, S> Syntax for SeparatedList<T, S>
where
    T: Syntax,
    S: Syntax,
{
    type Data = Vec<T::Data>;

    fn parse(&self, parser: &mut Parser<'_>) -> Option<Vec<T::Data>> {
        let SeparatedList(element, delimiter) = self;

        let mut result = vec![];
        parser.eat_newlines();
        loop {
            if let Some(element) = parser.eat(element) {
                result.push(element);

                if let Some(_) = parser.eat(delimiter) {
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

        Some(result)
    }

    fn singular_name(&self) -> String {
        self.element().plural_name()
    }

    fn plural_name(&self) -> String {
        self.element().plural_name()
    }
}
