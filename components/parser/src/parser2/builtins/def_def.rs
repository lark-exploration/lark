use crate::prelude::*;

use crate::parser::{ParseError, Spanned};
use crate::parser2::allow::ALLOW_NEWLINE;
use crate::parser2::lite_parse::{BindingId, ScopeId};
use crate::parser2::lite_parse::{
    ExpectedId, LiteParser, MaybeTerminator, RelativePosition, Token,
};
use crate::parser2::macros::{MacroRead, Term};
use crate::parser2::reader::Reader;
use crate::parser2::token_tree::Handle;

use log::trace;

#[derive(Debug)]
struct Param {
    name: Spanned<Token>,
    ty: Handle,
}

pub struct DefDef;

impl MacroRead for DefDef {
    fn extent(&self, reader: &mut Reader<'_>) -> Result<(), ParseError> {
        unimplemented!()
    }

    fn read(
        &self,
        scope: ScopeId,
        reader: &mut LiteParser<'_>,
    ) -> Result<Box<dyn Term>, ParseError> {
        let binding = reader.export_name(scope, RelativePosition::Hoist, false)?;
        let name = reader.get_binding_name(&scope, binding.node());
        reader.start_entity(name);

        reader.expect_sigil("(", ALLOW_NEWLINE)?;

        let body_scope = reader.child_scope(&scope);

        let mut params: Vec<Param> = vec![];

        loop {
            let field = reader.expect_id_until(
                ALLOW_NEWLINE,
                ExpectedId::AnyIdentifier,
                |name| Token::Binding {
                    scope: body_scope,
                    name,
                },
                reader.sigil(")"),
            )?;

            match field {
                MaybeTerminator::Terminator(_) => break,
                MaybeTerminator::Token(name) => {
                    reader.expect_sigil(":", ALLOW_NEWLINE)?;
                    let ty = reader.expect_type(ALLOW_NEWLINE, scope)?;
                    params.push(Param { name, ty });

                    match reader.maybe_sigil(",", ALLOW_NEWLINE)? {
                        (true, _) => {}
                        (false, _) => {
                            reader.expect_sigil(")", ALLOW_NEWLINE)?;
                            break;
                        }
                    }
                }
            }
        }

        let ty = match reader.maybe_sigil("->", ALLOW_NEWLINE)? {
            (true, _) => Some(reader.expect_type(ALLOW_NEWLINE, scope)?),
            (false, _) => None,
        };

        reader.expect_sigil("{", ALLOW_NEWLINE)?;

        reader.expect_expr(&body_scope)?;

        reader.end_entity();

        trace!("DefDef {{ name: {:?}, params: {:?} }}", name, params);

        Ok(Box::new(DefDefTerm {
            name: binding,
            params,
            ret: ty,
        }))
    }
}

struct DefDefTerm {
    name: Spanned<BindingId>,
    params: Vec<Param>,
    ret: Option<Handle>,
}

impl Term for DefDefTerm {}
