use parking_lot::RwLock;
use parser::ast;
use parser::ParseError;
use parser::StringId;
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Default)]
pub struct ParserState {
    module_table: RwLock<parser::program::ModuleTable>,
}

impl ParserState {
    crate fn parse(
        &self,
        _path: StringId,
        input_text: StringId,
    ) -> Result<ast::Module, ParseError> {
        let mut module_table = self.module_table.write();
        let input_text = module_table.lookup(input_text).clone();
        parser::parse(Cow::Borrowed(&**input_text), &mut module_table, 0)
    }

    crate fn untern_string(&self, string_id: StringId) -> Arc<String> {
        self.module_table.read().lookup(string_id).clone()
    }

    crate fn intern_string(&self, hashable: impl parser::program::Seahash) -> StringId {
        {
            let module_table = self.module_table.read();
            if let Some(id) = module_table.get(&hashable) {
                return id;
            }
        }

        let mut module_table = self.module_table.write();
        module_table.intern(hashable)
    }
}
