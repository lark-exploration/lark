//! Built-in syntax that can be used by macros.

use crate::parser::Parser;

pub mod field;
pub mod list;
pub mod sigil;
pub mod type_reference;

pub trait Syntax {
    /// The value that is produced (often, but not always, `Self`) by the
    /// parsing routine.
    type Data;

    /// Routine to do the parsing.
    fn parse(&self, parser: &mut Parser<'_>) -> Option<Self::Data>;

    /// For error messages, human readable name describing what is to
    /// be parsed in the singular, e.g., "type" or "expression".
    fn singular_name(&self) -> String;

    /// For error messages, human readable name describing what is to
    /// be parsed in the plural, e.g., "types" or "expressions".
    fn plural_name(&self) -> String;
}

impl<T> Syntax for &T
where
    T: Syntax,
{
    type Data = T::Data;

    fn parse(&self, parser: &mut Parser<'_>) -> Option<Self::Data> {
        T::parse(self, parser)
    }

    fn singular_name(&self) -> String {
        T::singular_name(self)
    }

    fn plural_name(&self) -> String {
        T::plural_name(self)
    }
}
