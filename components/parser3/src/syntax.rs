//! Built-in syntax that can be used by macros.

use crate::parser::Parser;

pub mod field;
pub mod list;
pub mod type_reference;

pub trait Syntax {
    /// The value that is produced (often, but not always, `Self`) by the
    /// parsing routine.
    type Data;

    /// Routine to do the parsing.
    fn parse(parser: &mut Parser<'_>) -> Option<Self::Data>;

    /// For error messages, human readable name describing what is to
    /// be parsed in the singular, e.g., "type" or "expression".
    fn singular_name() -> String;

    /// For error messages, human readable name describing what is to
    /// be parsed in the plural, e.g., "types" or "expressions".
    fn plural_name() -> String;
}

pub trait InfallibleSyntax {
    type Data;

    fn parse_infallible(parser: &mut Parser<'_>) -> Self::Data;

    fn singular_name() -> String;

    fn plural_name() -> String;
}

impl<T: InfallibleSyntax> Syntax for T {
    type Data = <T as InfallibleSyntax>::Data;

    fn parse(parser: &mut Parser<'_>) -> Option<Self::Data> {
        Some(<T as InfallibleSyntax>::parse_infallible(parser))
    }

    fn singular_name() -> String {
        <T as InfallibleSyntax>::singular_name()
    }

    fn plural_name() -> String {
        <T as InfallibleSyntax>::plural_name()
    }
}
