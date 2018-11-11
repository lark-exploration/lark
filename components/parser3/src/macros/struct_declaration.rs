use crate::macros::EntityMacroDefinition;
use crate::parsed_entity::LazyParsedEntity;
use crate::parsed_entity::ParsedEntity;
use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::field::Field;
use crate::syntax::field::ParsedField;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::list::CommaList;
use crate::syntax::sigil::CloseCurly;
use crate::syntax::sigil::OpenCurly;
use intern::Intern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::ItemKind;
use lark_error::ErrorReported;
use lark_string::global::GlobalIdentifier;
use std::sync::Arc;

/// ```ignore
/// struct <id> {
///   <id>: <ty> // separated by `,` or newline
/// }
/// ```
#[derive(Default)]
pub struct StructDeclaration;

impl EntityMacroDefinition for StructDeclaration {
    fn parse(
        &self,
        parser: &mut Parser<'_>,
        base: Entity,
        macro_name: Spanned<GlobalIdentifier>,
    ) -> Result<ParsedEntity, ErrorReported> {
        let struct_name = parser.expect(SpannedGlobalIdentifier)?;
        let _ = parser.expect(OpenCurly)?;
        let fields = parser.eat(CommaList(Field)).unwrap_or(vec![]);
        parser.eat_newlines();
        parser.expect(CloseCurly)?;

        let entity = EntityData::ItemName {
            base,
            kind: ItemKind::Struct,
            id: struct_name.value,
        }
        .intern(parser);

        let full_span = macro_name.span.extended_until_end_of(parser.last_span());
        let characteristic_span = struct_name.span;
        let fields = Arc::new(fields);

        Ok(ParsedEntity::new(
            entity,
            full_span,
            characteristic_span,
            Arc::new(ParsedStructDeclaration { fields }),
        ))
    }
}

struct ParsedStructDeclaration {
    fields: Arc<Vec<ParsedField>>,
}

impl LazyParsedEntity for ParsedStructDeclaration {
    fn parse_children(&self) -> Vec<ParsedEntity> {
        unimplemented!()
    }
}
