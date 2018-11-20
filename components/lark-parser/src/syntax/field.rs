use crate::parser::Parser;
use crate::syntax::entity::InvalidParsedEntity;
use crate::syntax::entity::LazyParsedEntity;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::entity::ParsedEntity;
use crate::syntax::guard::Guard;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::sigil::Colon;
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::type_reference::{ParsedTypeReference, TypeReference};
use crate::syntax::Syntax;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::ErrorReported;
use lark_error::ResultExt;
use lark_error::WithError;
use lark_hir as hir;
use lark_span::FileName;
use lark_span::Spanned;
use lark_string::global::GlobalIdentifier;

#[derive(DebugWith)]
pub struct Field;

/// Represents a parse of something like `foo: Type`
#[derive(Copy, Clone, DebugWith)]
pub struct ParsedField {
    pub name: Spanned<GlobalIdentifier, FileName>,
    pub ty: ParsedTypeReference,
}

impl Syntax<'parse> for Field {
    type Data = Spanned<ParsedField, FileName>;

    fn test(&mut self, parser: &Parser<'_>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(&mut self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
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

    fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody> {
        InvalidParsedEntity.parse_fn_body(entity, db)
    }
}
