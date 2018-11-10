use crate::prelude::*;

use crate::parser2::allow::ALLOW_NEWLINE;
use crate::parser2::entity_tree::EntityKind;
use crate::parser2::lite_parse::LiteParser;
use crate::parser2::lite_parse::ScopeId;
use crate::parser2::macros::{MacroRead, Term};
use crate::parser2::reader::{self, Reader};
use crate::parser2::token_tree::Handle;
use crate::LexToken;

pub struct StructDef;

impl MacroRead for StructDef {
    fn extent(&self, reader: &mut Reader<'_>) -> Result<(), ParseError> {
        let name = reader.expect_id(ALLOW_NEWLINE)?;
        reader.start_entity(&name, EntityKind::Struct);

        reader.expect_sigil("{", ALLOW_NEWLINE)?;

        let mut fields: Vec<Field> = vec![];

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
                    fields.push(Field { name, ty });
                    reader.expect_sigil(",", ALLOW_NEWLINE)?;
                }
            }
        }

        reader.end_entity(box StructDefTerm { name, fields });

        Ok(())
    }

    #[allow(unused)]
    fn read(
        &self,
        scope: ScopeId,
        reader: &mut LiteParser<'_>,
    ) -> Result<Box<dyn Term>, ParseError> {
        unimplemented!()
    }
}

#[derive(Debug)]
struct Field {
    name: Spanned<LexToken>,
    ty: Handle,
}

#[derive(Debug)]
struct StructDefTerm {
    name: Spanned<GlobalIdentifier>,
    fields: Vec<Field>,
}

impl Term for StructDefTerm {}
