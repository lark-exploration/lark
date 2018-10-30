use crate::prelude::*;

use crate::parser2::allow::ALLOW_NEWLINE;
use crate::parser2::entity_tree::EntityKind;
use crate::parser2::lite_parse::LiteParser;
use crate::parser2::lite_parse::ScopeId;
use crate::parser2::macros::{MacroRead, Term};
use crate::parser2::reader::{self, PairedDelimiter, Reader};
use crate::parser2::token_tree::Handle;
use crate::LexToken;

use log::trace;

impl MacroRead for DefDef {
    fn extent(&self, reader: &mut Reader<'_>) -> Result<(), ParseError> {
        let name = reader.expect_id(ALLOW_NEWLINE)?;
        reader.start_entity(&name, EntityKind::Def);

        reader.expect_sigil("(", ALLOW_NEWLINE)?;

        let mut params: Vec<Param> = vec![];

        loop {
            let field = reader.expect_id_until(
                ALLOW_NEWLINE,
                reader::ExpectedId::AnyIdentifier,
                reader.sigil(")"),
            )?;

            match field {
                reader::MaybeTerminator::Terminator(_) => break,
                reader::MaybeTerminator::Token(name) => {
                    reader.expect_sigil(":", ALLOW_NEWLINE)?;
                    let ty = reader.expect_type(ALLOW_NEWLINE)?;
                    params.push(Param { name, ty });

                    match reader.maybe_sigil(",", ALLOW_NEWLINE)? {
                        Ok(_) => {}
                        Err(_) => {
                            reader.expect_sigil(")", ALLOW_NEWLINE)?;
                            break;
                        }
                    }
                }
            }
        }

        let ret = match reader.maybe_sigil("->", ALLOW_NEWLINE)? {
            Ok(_) => Some(reader.expect_type(ALLOW_NEWLINE)?),
            Err(_) => None,
        };

        let sigil = reader.expect_sigil("{", ALLOW_NEWLINE)?;
        reader.expect_paired_delimiters(sigil.copy(PairedDelimiter::Curly))?;

        trace!(
            "DefDefTerm {{ name: {:?}, params: {:?}, ret: {:?} }}",
            name,
            params,
            ret
        );

        reader.end_entity(box DefDefTerm { name, params, ret });

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
struct Param {
    name: Spanned<LexToken>,
    ty: Handle,
}

pub struct DefDef;

#[derive(Debug)]
struct DefDefTerm {
    name: Spanned<StringId>,
    params: Vec<Param>,
    ret: Option<Handle>,
}

impl Term for DefDefTerm {}
