//! Built-in syntax that can be used by macros.

use crate::parser::Parser;
use lark_debug_with::DebugWith;
use lark_error::ErrorReported;

pub mod delimited;
pub mod entity;
pub mod field;
pub mod fn_body;
pub mod guard;
pub mod identifier;
pub mod list;
pub mod matched;
pub mod sigil;
pub mod skip_newline;
pub mod type_reference;
pub mod expression;

pub trait Syntax<'parse>: DebugWith {
    /// The value that is produced (often, but not always, `Self`) by the
    /// parsing routine.
    type Data;

    /// Routine to check if this syntax applies. This often does a
    /// much more shallow check than `expect`, e.g., just checking an
    /// initial token or two.
    fn test(&mut self, parser: &Parser<'parse>) -> bool;

    /// Routine to do the parsing itself. This will produce a parse
    /// error if the syntax is not found at the current point.
    ///
    /// **Relationship to test:** If `test` returns false, errors are
    /// guaranteed. Even if `test` returns true, however, errors are
    /// still possible, since `test` does a more shallow check.
    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported>;
}

/// A Syntax whose `expect` method, when `test` returns true, always
/// consumes at least one token.
pub trait NonEmptySyntax<'parse>: Syntax<'parse> {}

pub trait Delimiter<'parse>: DebugWith {
    type Open: NonEmptySyntax<'parse>;
    type Close: NonEmptySyntax<'parse>;
    fn open_syntax(&self) -> Self::Open;
    fn close_syntax(&self) -> Self::Close;
}

impl<T> Syntax<'parse> for &mut T
where
    T: Syntax<'parse>,
{
    type Data = T::Data;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        T::test(self, parser)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        T::expect(self, parser)
    }
}
