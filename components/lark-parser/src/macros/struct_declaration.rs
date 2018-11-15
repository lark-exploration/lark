use crate::macros::EntityMacroDefinition;
use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::entity::{
    LazyParsedEntity, LazyParsedEntityDatabase, ParsedEntity, ParsedEntityThunk,
};
use crate::syntax::field::{Field, ParsedField};
use crate::syntax::list::CommaList;
use crate::syntax::sigil::Curlies;
use crate::syntax::skip_newline::SkipNewline;

use debug::DebugWith;
use intern::Intern;
use lark_entity::{Entity, EntityData, ItemKind, MemberKind};
use lark_error::{ErrorReported, WithError};
use lark_seq::Seq;
use lark_span::{FileName, Spanned, SpannedGlobalIdentifier};
use lark_string::GlobalIdentifier;

/// ```ignore
/// struct <id> {
///   <id>: <ty> // separated by `,` or newline
/// }
/// ```
#[derive(Default)]
pub struct StructDeclaration;

impl EntityMacroDefinition for StructDeclaration {
    fn expect(
        &self,
        parser: &mut Parser<'_>,
        base: Entity,
        macro_name: Spanned<GlobalIdentifier, FileName>,
    ) -> Result<ParsedEntity, ErrorReported> {
        log::trace!(
            "StructDeclaration::parse(base={}, macro_name={})",
            base.debug_with(parser),
            macro_name.debug_with(parser)
        );

        log::trace!("StructDeclaration::parse: parsing name");
        let struct_name = parser.expect(SkipNewline(SpannedGlobalIdentifier))?;

        log::trace!("StructDeclaration::parse: parsing fields");
        let fields = parser
            .expect(SkipNewline(Delimited(Curlies, CommaList(Field))))
            .unwrap_or_else(|ErrorReported(_)| Seq::default());

        log::trace!("StructDeclaration::parse: done");
        let entity = EntityData::ItemName {
            base,
            kind: ItemKind::Struct,
            id: struct_name.value,
        }
        .intern(parser);

        let full_span = macro_name.span.extended_until_end_of(parser.last_span());
        let characteristic_span = struct_name.span;

        Ok(ParsedEntity::new(
            entity,
            full_span,
            characteristic_span,
            ParsedEntityThunk::new(ParsedStructDeclaration { fields }),
        ))
    }
}

struct ParsedStructDeclaration {
    fields: Seq<Spanned<ParsedField, FileName>>,
}

impl LazyParsedEntity for ParsedStructDeclaration {
    fn parse_children(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Vec<ParsedEntity>> {
        WithError::ok(
            self.fields
                .iter()
                .map(|Spanned { value: field, span }| {
                    let field_entity = EntityData::MemberName {
                        base: entity,
                        kind: MemberKind::Field,
                        id: field.name.value,
                    }
                    .intern(db);

                    ParsedEntity::new(
                        field_entity,
                        *span,
                        field.name.span,
                        ParsedEntityThunk::new(field.clone()),
                    )
                })
                .collect(),
        )
    }
}
