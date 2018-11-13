use crate::prelude::*;
use crate::{LookupStringId, ModuleTable};
use intern::Intern;
use lark_string::global::GlobalIdentifierTables;
use std::borrow::Cow;

/// Trait encapsulating the String interner. This should be
/// synchronized with the `intern` crate eventually.
pub trait HasParserState: LookupStringId {
    fn parser_state(&self) -> &ParserState;

    fn untern_string(&self, string_id: GlobalIdentifier) -> Text {
        self.parser_state().untern_string(string_id)
    }

    fn intern_string<I>(&self, hashable: I) -> I::Key
    where
        I: Intern<GlobalIdentifierTables>,
    {
        self.parser_state().intern_string(hashable)
    }
}

#[derive(Default)]
pub struct ParserState {
    module_table: ModuleTable,
}

impl ParserState {
    pub fn parse(&self, input_text: &str) -> Result<crate::ast::Module, ParseError> {
        crate::parse(Cow::Borrowed(input_text), &self.module_table, 1)
    }

    pub fn untern_string(&self, string_id: GlobalIdentifier) -> Text {
        self.module_table.lookup(&string_id)
    }

    fn intern_string<I>(&self, hashable: I) -> I::Key
    where
        I: Intern<GlobalIdentifierTables>,
    {
        self.module_table.intern(hashable)
    }
}

impl AsRef<GlobalIdentifierTables> for ParserState {
    fn as_ref(&self) -> &GlobalIdentifierTables {
        self.module_table.as_ref()
    }
}
