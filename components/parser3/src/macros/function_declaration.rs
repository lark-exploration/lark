use crate::macros::type_reference::ParsedTypeReference;
use crate::macros::EntityMacroDefinition;
use crate::parsed_entity::LazyParsedEntity;
use crate::parsed_entity::ParsedEntity;
use crate::parser::Parser;
use crate::span::CurrentFile;
use crate::span::Location;
use crate::span::Spanned;
use intern::Intern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::ItemKind;
use lark_string::global::GlobalIdentifier;
use std::sync::Arc;

/// ```ignore
/// `def` <id> `(` <id> `:` <ty> `)` [ `->` <ty> ] <block>
/// ```
#[derive(Default)]
pub struct FunctionDeclaration;

impl EntityMacroDefinition for FunctionDeclaration {
    fn parse(
        &self,
        parser: &mut Parser<'_>,
        base: Entity,
        start: Location<CurrentFile>,
    ) -> ParsedEntity {
        let function_name = or_error_entity!(
            parser.eat_global_identifier(),
            parser,
            "expected function name"
        );

        or_error_entity!(parser.eat_sigil("("), parser, "expected `(`");

        // Consume the parameters
        parser.eat_newlines();

        let mut fields = vec![];
        loop {
            if let Some(name) = parser.eat_global_identifier() {
                if let Some(ty) = parser.parse_type() {
                    fields.push(ParsedField { name, ty });

                    // If there is a `,` or a newline, then there may
                    // be more fields, so go back around the loop.
                    if let Some(_) = parser.eat_sigil(",") {
                        parser.eat_newlines();
                        continue;
                    } else if parser.eat_newlines() {
                        continue;
                    }
                }
            }

            break;
        }

        if let None = parser.eat_sigil("}") {
            parser.report_error("expected `}`", parser.peek_span());
        }

        let entity = EntityData::ItemName {
            base,
            kind: ItemKind::Struct,
            id: struct_name.value,
        }
        .intern(parser);

        let span = start.until_end_of(parser.last_span());

        ParsedEntity::new(entity, span, Arc::new(ParsedStructDeclaration { fields }))
    }
}

struct ParsedStructDeclaration {
    fields: Vec<ParsedField>,
}

struct ParsedField {
    name: Spanned<GlobalIdentifier>,
    ty: ParsedTypeReference,
}

impl LazyParsedEntity for ParsedStructDeclaration {
    fn parse_children(&self) -> Vec<ParsedEntity> {
        unimplemented!()
    }
}
