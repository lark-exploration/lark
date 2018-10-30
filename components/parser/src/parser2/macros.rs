use crate::prelude::*;

use super::lite_parse::ScopeId;

use crate::intern::ModuleTable;
use crate::parser2::builtins;
use crate::parser2::lite_parse::LiteParser;
use crate::parser2::reader::Reader;

use map::FxIndexMap;
use std::fmt::{self, Debug};
use std::sync::Arc;

#[derive(Default)]
pub struct MacroMap {
    macros: FxIndexMap<StringId, Arc<MacroRead>>,
}

impl MacroMap {
    pub fn add(mut self, name: StringId, macro_def: impl MacroRead + 'static) -> MacroMap {
        self.macros.insert(name, Arc::new(macro_def));
        self
    }

    pub fn get(&self, name: StringId) -> Option<Arc<dyn MacroRead>> {
        self.macros.get(&name).cloned()
    }

    pub fn has(&self, name: &StringId) -> bool {
        self.macros.contains_key(name)
    }
}

impl Debug for MacroMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.macros.keys()).finish()
    }
}

#[derive(Default)]
pub struct Macros {
    named: MacroMap,

    #[allow(unused)]
    operator: MacroMap,
    #[allow(unused)]
    prefix: MacroMap,
}

pub trait MacroRead {
    fn extent(&self, reader: &mut Reader<'_>) -> Result<(), ParseError>;

    fn read(
        &self,
        scope: ScopeId,
        reader: &mut LiteParser<'_>,
    ) -> Result<Box<dyn Term>, ParseError>;
}

impl Debug for Macros {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.named.fmt(f)
    }
}

impl Macros {
    pub fn add(mut self, name: StringId, macro_def: impl MacroRead + 'static) -> Macros {
        self.named = self.named.add(name, macro_def);
        self
    }

    pub fn get(&self, name: StringId) -> Option<Arc<dyn MacroRead>> {
        self.named.get(name)
    }

    pub fn has(&self, name: &StringId) -> bool {
        self.named.has(name)
    }
}

pub fn macros(table: &mut ModuleTable) -> Macros {
    Macros::default()
        .add(table.intern(&"struct"), builtins::StructDef)
        .add(table.intern(&"def"), builtins::DefDef)
}

pub trait Term: Debug {}
