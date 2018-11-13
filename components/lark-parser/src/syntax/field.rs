use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::entity::ParsedEntity;
use crate::syntax::entity::LazyParsedEntity;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::sigil::Colon;
use crate::syntax::type_reference::ParsedTypeReference;
use crate::syntax::type_reference::TypeReference;
use crate::syntax::Syntax;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::ErrorReported;
use lark_error::ResultExt;
use lark_error::WithError;
use lark_string::global::GlobalIdentifier;

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

    fn parse(&self, parser: &mut Parser<'_>) -> Result<Spanned<ParsedField>, ErrorReported> {
        let name = parser.expect(SpannedGlobalIdentifier)?;

        let ty_result: Result<_, ErrorReported> = try {
            parser.expect(Colon)?;
            parser.expect(TypeReference)?
        };

        let ty = ty_result.unwrap_or_error_sentinel(&*parser);

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
