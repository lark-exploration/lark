use super::lite_parse::ScopeId;

use crate::parser::ast::Debuggable;
use crate::parser::program::ModuleTable;
use crate::parser::program::StringId;
use crate::parser::{ParseError, Spanned};
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

#[derive(Debug)]
struct Field {
    name: Spanned<Token>,
    ty: Handle,
}

pub fn struct_decl(
    scope: ScopeId,
    reader: &mut LiteParser<'_>,
) -> Result<Box<dyn Term>, ParseError> {
    let name = reader.export_name(scope, RelativePosition::Hoist, false)?;
    reader.expect_sigil("{", ALLOW_NEWLINE)?;

    let mut fields: Vec<Field> = vec![];

    loop {
        let field = reader.expect_id_until(
            ALLOW_NEWLINE,
            ExpectedId::AnyIdentifier,
            Token::Label,
            reader.sigil("}"),
        )?;

        match field {
            MaybeTerminator::Terminator(_) => break,
            MaybeTerminator::Token(name) => {
                reader.expect_sigil(":", ALLOW_NEWLINE)?;
                let ty = reader.expect_type(ALLOW_NEWLINE, scope)?;
                fields.push(Field { name, ty });
                reader.expect_sigil(",", ALLOW_NEWLINE)?;
            }
        }
    }

    trace!("StructDecl {{ name: {:?}, fields: {:?} }}", name, fields);

    Ok(Box::new(StructDecl { name, fields }))
}

pub fn macros(table: &mut ModuleTable) -> Macros {
    Macros::default().add(table.intern(&"struct"), MacroReadFn::new(struct_decl))
}

pub trait Term {}

struct StructDecl {
    name: Spanned<BindingId>,
    fields: Vec<Field>,
}

impl Term for StructDecl {}
