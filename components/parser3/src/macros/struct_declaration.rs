use crate::macros::EntityMacroDefinition;
use crate::parsed_entity::LazyParsedEntity;
use crate::parsed_entity::ParsedEntity;
use crate::parser::Parser;
use crate::span::CurrentFile;
use crate::span::Span;
use crate::span::Spanned;
use crate::syntax::field::Field;
use crate::syntax::list::CommaList;
use intern::Intern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::ItemKind;
use lark_string::global::GlobalIdentifier;
use std::sync::Arc;

/// ```ignore
/// struct <id> {
///   <id>: <ty> // separated by `,` or newline
/// }
/// ```
#[derive(Default)]
pub struct StructDeclarationMacro;

impl EntityMacroDefinition for StructDeclarationMacro {
    fn parse(
        &self,
        parser: &mut Parser<'_>,
        base: Entity,
        macro_name: Spanned<GlobalIdentifier>,
    ) -> ParsedEntity {
        let struct_name = or_error_entity!(
            parser.eat_global_identifier(),
            parser,
            "expected struct name"
        );

        // We always produce a field list, but sometimes we also set
        // the error flag to `Some`, indicating that the field list is
        // wonky (i.e., there may be additional fields we do not know
        // about). Is this trying too hard to recover? Maybe we should
        // just use a `Result<Vec<>, ErrorReported>`?
        let mut error = None;
        let mut fields = vec![];

        if let Some(_) = parser.eat_sigil("{") {
            fields = parser.eat_infallible_syntax::<CommaList<Field>>();

            parser.eat_newlines();

            if let None = parser.eat_sigil("}") {
                parser.report_error("expected `}`", parser.peek_span());
                error = Some(parser.peek_span());
            }
        } else {
            parser.report_error("expected `}`", parser.peek_span());
            error = Some(parser.peek_span());
        }

        let entity = EntityData::ItemName {
            base,
            kind: ItemKind::Struct,
            id: struct_name.value,
        }
        .intern(parser);

        let full_span = macro_name.span.extended_until_end_of(parser.last_span());
        let characteristic_span = struct_name.span;
        let fields = Arc::new(fields);

        ParsedEntity::new(
            entity,
            full_span,
            characteristic_span,
            Arc::new(StructDeclaration { fields, error }),
        )
    }
}

struct StructDeclaration {
    fields: Arc<Vec<Field>>,
    error: Option<Span<CurrentFile>>,
}

impl LazyParsedEntity for StructDeclaration {
    fn parse_children(&self) -> Vec<ParsedEntity> {
        unimplemented!()
    }
}
