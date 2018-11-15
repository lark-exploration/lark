use crate::macros::EntityMacroDefinition;
use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::entity::{
    LazyParsedEntity, LazyParsedEntityDatabase, ParsedEntity, ParsedEntityThunk,
};
use crate::syntax::field::{Field, ParsedField};
use crate::syntax::guard::Guard;
use crate::syntax::list::CommaList;
use crate::syntax::matched::{Matched, ParsedMatch};
use crate::syntax::sigil::{Curlies, Parentheses, RightArrow};
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::type_reference::{ParsedTypeReference, TypeReference};

use debug::DebugWith;
use intern::Intern;
use lark_entity::{Entity, EntityData, ItemKind};
use lark_error::{ErrorReported, ResultExt, WithError};
use lark_seq::Seq;
use lark_span::{FileName, Spanned, SpannedGlobalIdentifier};
use lark_string::global::GlobalIdentifier;

/// ```ignore
/// `def` <id> `(` <id> `:` <ty> `)` [ `->` <ty> ] <block>
/// ```
#[derive(Default)]
pub struct FunctionDeclaration;

impl EntityMacroDefinition for FunctionDeclaration {
    fn expect(
        &self,
        parser: &mut Parser<'_>,
        base: Entity,
        macro_name: Spanned<GlobalIdentifier, FileName>,
    ) -> Result<ParsedEntity, ErrorReported> {
        log::trace!(
            "FunctionDeclaration::parse(base={}, macro_name={})",
            base.debug_with(parser),
            macro_name.debug_with(parser)
        );

        let function_name = parser.expect(SkipNewline(SpannedGlobalIdentifier))?;

        let parameters = parser
            .expect(SkipNewline(Delimited(Parentheses, CommaList(Field))))
            .unwrap_or_else(|ErrorReported(_)| Seq::default());

        let return_type = match parser
            .parse_if_present(SkipNewline(Guard(RightArrow, SkipNewline(TypeReference))))
        {
            Some(ty) => ty.unwrap_or_error_sentinel(&*parser),
            None => ParsedTypeReference::Elided(parser.elided_span()),
        };

        let body = parser.expect(SkipNewline(Matched(Curlies)));

        let entity = EntityData::ItemName {
            base,
            kind: ItemKind::Function,
            id: function_name.value,
        }
        .intern(parser);

        let full_span = macro_name.span.extended_until_end_of(parser.last_span());
        let characteristic_span = function_name.span;

        Ok(ParsedEntity::new(
            entity,
            full_span,
            characteristic_span,
            ParsedEntityThunk::new(ParsedFunctionDeclaration {
                parameters,
                return_type,
                body,
            }),
        ))
    }
}

struct ParsedFunctionDeclaration {
    parameters: Seq<Spanned<ParsedField, FileName>>,
    return_type: ParsedTypeReference,
    body: Result<Spanned<ParsedMatch, FileName>, ErrorReported>,
}

impl LazyParsedEntity for ParsedFunctionDeclaration {
    fn parse_children(
        &self,
        _entity: Entity,
        _db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Vec<ParsedEntity>> {
        WithError::ok(vec![])
    }
}
