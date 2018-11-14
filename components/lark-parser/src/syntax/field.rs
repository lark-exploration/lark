use crate::parser::Parser;
use crate::syntax::entity::{LazyParsedEntity, LazyParsedEntityDatabase, ParsedEntity};
use crate::syntax::guard::Guard;
use crate::syntax::sigil::Colon;
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::type_reference::{ParsedTypeReference, TypeReference};
use crate::syntax::Syntax;

use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::{ErrorReported, ResultExt, WithError};
use lark_span::{Spanned, SpannedGlobalIdentifier};
use lark_string::GlobalIdentifier;

#[derive(DebugWith)]
pub struct Field;

/// Represents a parse of something like `foo: Type`
#[derive(Copy, Clone, DebugWith)]
pub struct ParsedField {
    pub name: Spanned<GlobalIdentifier>,
    pub ty: ParsedTypeReference,
}

impl Syntax for Field {
    type Data = Spanned<ParsedField>;

    fn test(&self, parser: &Parser<'_>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(&self, parser: &mut Parser<'_>) -> Result<Spanned<ParsedField>, ErrorReported> {
        let name = parser.expect(SpannedGlobalIdentifier)?;

        let ty = parser
            .expect(SkipNewline(Guard(Colon, SkipNewline(TypeReference))))
            .unwrap_or_error_sentinel(&*parser);

        let span = name.span.extended_until_end_of(parser.last_span());

        return Ok(Spanned {
            value: ParsedField { name, ty },
            span,
        });
    }
}

impl LazyParsedEntity for ParsedField {
    fn parse_children(
        &self,
        _entity: Entity,
        _db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Vec<ParsedEntity>> {
        WithError::ok(vec![])
    }
}
