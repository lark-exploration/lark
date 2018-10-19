use crate::prelude::*;

use crate::parser::{ParseError, Spanned};
use crate::parser2::lite_parse::{BindingId, ScopeId};
use crate::parser2::lite_parse::{
    ExpectedId, LiteParser, MaybeTerminator, RelativePosition, Token, ALLOW_NEWLINE,
};
use crate::parser2::macros::Term;
use crate::parser2::token_tree::Handle;

use log::trace;

#[derive(Debug)]
struct Field {
    name: Spanned<Token>,
    ty: Handle,
}

pub fn struct_def(
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

    Ok(Box::new(StructDef {
        name: binding,
        fields,
    }))
}

struct StructDef {
    name: Spanned<BindingId>,
    fields: Vec<Field>,
}

impl Term for StructDef {}
