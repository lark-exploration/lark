//! Built-in syntax that can be used by macros.

use crate::parser::Parser;
use debug::DebugWith;
use lark_error::ErrorReported;

pub mod delimited;
pub mod entity;
pub mod field;
pub mod identifier;
pub mod list;
pub mod sigil;
pub mod type_reference;

pub trait Syntax: DebugWith {
    /// The value that is produced (often, but not always, `Self`) by the
    /// parsing routine.
    type Data;

    /// Routine to check if this syntax applies.
    fn test(&self, parser: &Parser<'_>) -> bool;

    /// Routine to do the parsing itself. If `test` is not true, this
    /// will produce a parse error.
    fn parse(&self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported>;
}

pub trait Delimiter: DebugWith {
    type Open: Syntax;
    type Close: Syntax;
    fn open_syntax(&self) -> Self::Open;
    fn close_syntax(&self) -> Self::Close;
}

impl<T> Syntax for &T
where
    T: Syntax,
{
    type Data = T::Data;

    fn test(&self, parser: &Parser<'_>) -> bool {
        T::test(self, parser)
    }

    fn parse(&self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        T::parse(self, parser)
    }
}
