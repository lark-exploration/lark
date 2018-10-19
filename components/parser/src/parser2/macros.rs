use crate::prelude::*;

use super::lite_parse::ScopeId;

use crate::parser::ast::Debuggable;
use crate::parser::program::ModuleTable;
use crate::parser::program::StringId;
use crate::parser::{ParseError, Spanned};
use crate::parser2::builtins;
use crate::parser2::lite_parse::{
    AllowPolicy, BindingId, Expected, ExpectedId, LiteParser, MaybeTerminator, RelativePosition,
    Token, ALLOW_EOF, ALLOW_NEWLINE, ALLOW_NONE,
};
use crate::parser2::quicklex::Token as LexToken;
use crate::parser2::token_tree::Handle;

use derive_new::new;
use log::trace;
use map::FxIndexMap;
use std::fmt::{self, Debug};
use std::sync::Arc;

#[derive(Default)]
pub struct Macros {
    named: FxIndexMap<StringId, Arc<MacroRead>>,
}

pub trait MacroRead {
    fn read(
        &self,
        scope: ScopeId,
        reader: &mut LiteParser<'_>,
    ) -> Result<Box<dyn Term>, ParseError>;
}

impl Debug for Macros {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.named.keys()).finish()
    }
}

impl Macros {
    pub fn add(mut self, name: StringId, macro_def: impl MacroRead + 'static) -> Macros {
        self.named.insert(name, Arc::new(macro_def));
        self
    }

    pub fn get(&self, name: StringId) -> Option<Arc<dyn MacroRead>> {
        self.named.get(&name).cloned()
    }
}

#[derive(new)]
pub struct MacroReadFn<F>
where
    F: Fn(ScopeId, &mut LiteParser<'_>) -> Result<Box<dyn Term>, ParseError>,
{
    func: F,
}

impl<F> MacroRead for MacroReadFn<F>
where
    F: Fn(ScopeId, &mut LiteParser<'_>) -> Result<Box<dyn Term>, ParseError>,
{
    fn read(
        &self,
        scope: ScopeId,
        reader: &mut LiteParser<'_>,
    ) -> Result<Box<dyn Term>, ParseError> {
        (self.func)(scope, reader)
    }
}

pub fn macros(table: &mut ModuleTable) -> Macros {
    Macros::default().add(
        table.intern(&"struct"),
        MacroReadFn::new(builtins::struct_def),
    )
}

pub trait Term {}
