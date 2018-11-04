use crate::prelude::*;

use crate::{LookupStringId, ModuleTable, Seahash, StringId};

use lark_debug_derive::DebugWith;
use parking_lot::RwLock;
use std::borrow::Cow;
use std::sync::Arc;

/// Trait encapsulating the String interner. This should be
/// synchronized with the `intern` crate eventually.
pub trait HasParserState: LookupStringId {
    fn parser_state(&self) -> &ParserState;

    fn untern_string(&self, string_id: StringId) -> Arc<String> {
        self.parser_state().untern_string(string_id)
    }

    fn intern_string(&self, hashable: impl Seahash) -> StringId {
        self.parser_state().intern_string(hashable)
    }
}

#[derive(Default)]
pub struct ParserState {
    module_table: RwLock<ModuleTable>,
}

impl ParserState {
    pub fn parse(&self, input_text: &str) -> Result<crate::ast::Module, ParseError> {
        crate::parse(Cow::Borrowed(input_text), &mut self.module_table.write(), 1)
    }

    pub fn untern_string(&self, string_id: StringId) -> Arc<String> {
        Arc::new(self.module_table.read().lookup(&string_id).to_string())
    }

    pub fn intern_string(&self, hashable: impl crate::Seahash) -> StringId {
        {
            let module_table = self.module_table.read();
            if let Some(id) = module_table.get(&hashable) {
                return id;
            }
        }

        let mut module_table = self.module_table.write();
        module_table.intern(&hashable)
    }
}

impl LookupStringId for ParserState {
    fn lookup(&self, id: StringId) -> Arc<String> {
        self.untern_string(id)
    }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq)]
pub struct InputText {
    pub text: StringId,
    pub start_offset: u32,
    pub span: Span,
}
