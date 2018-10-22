use crate::prelude::*;

use crate::parser::{ParseError, Spanned};
use crate::parser2::allow::ALLOW_NEWLINE;
use crate::parser2::lite_parse::{BindingId, ScopeId};
use crate::parser2::lite_parse::{
    ExpectedId, LiteParser, MaybeTerminator, RelativePosition, Token,
};
use crate::parser2::macros::{MacroRead, Term};
use crate::parser2::quicklex::Token as LexToken;
use crate::parser2::reader::{self, Reader};
use crate::parser2::token_tree::Handle;

use log::trace;

pub struct StructDef;

impl MacroRead for StructDef {
    fn extent(&self, reader: &mut Reader<'_>) -> Result<(), ParseError> {
        let name = reader.expect_id(ALLOW_NEWLINE)?;
        reader.start_entity(&name);

        reader.expect_sigil("{", ALLOW_NEWLINE)?;

        let mut fields: Vec<ExtentField> = vec![];

        loop {
            let field = reader.expect_id_until(
                ALLOW_NEWLINE,
                // TODO: Extract
                reader::ExpectedId::AnyIdentifier,
                reader.sigil("}"),
            )?;

            match field {
                reader::MaybeTerminator::Terminator(_) => break,
                reader::MaybeTerminator::Token(name) => {
                    reader.expect_sigil(":", ALLOW_NEWLINE)?;
                    let ty = reader.expect_type(ALLOW_NEWLINE)?;
                    fields.push(ExtentField { name, ty });
                    reader.expect_sigil(",", ALLOW_NEWLINE)?;
                }
            }
        }

        Ok(())
    }

    fn read(
        &self,
        scope: ScopeId,
        reader: &mut LiteParser<'_>,
    ) -> Result<Box<dyn Term>, ParseError> {
        let binding = reader.export_name(scope, RelativePosition::Hoist, false)?;
        let name = reader.get_binding_name(&scope, binding.node());
        reader.start_entity(name);
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

        reader.end_entity();

        trace!("StructDecl {{ name: {:?}, fields: {:?} }}", name, fields);

        Ok(Box::new(StructDefTerm {
            name: binding,
            fields,
        }))
    }
}

#[derive(Debug)]
struct ExtentField {
    name: Spanned<LexToken>,
    ty: Handle,
}

#[derive(Debug)]
struct StructExtentTerm {
    name: Spanned<LexToken>,
}

#[derive(Debug)]
struct Field {
    name: Spanned<Token>,
    ty: Handle,
}

#[derive(Debug)]
struct StructDefTerm {
    name: Spanned<BindingId>,
    fields: Vec<Field>,
}

impl Term for StructDefTerm {}
