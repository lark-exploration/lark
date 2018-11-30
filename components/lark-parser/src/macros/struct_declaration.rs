use crate::macros::EntityMacroDefinition;
use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::entity::{
    InvalidParsedEntity, LazyParsedEntity, LazyParsedEntityDatabase, ParsedEntity,
    ParsedEntityThunk,
};
use crate::syntax::field::{Field, ParsedField};
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::list::CommaList;
use crate::syntax::sigil::Curlies;
use crate::syntax::skip_newline::SkipNewline;
use lark_debug_with::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::ItemKind;
use lark_entity::MemberKind;
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_error::WithError;
use lark_hir as hir;
use lark_intern::Intern;
use lark_seq::Seq;
use lark_span::FileName;
use lark_span::Spanned;
use lark_string::GlobalIdentifier;
use lark_ty as ty;
use lark_ty::declaration::Declaration;
use std::sync::Arc;

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
    ) -> WithError<Seq<ParsedEntity>> {
        WithError::ok(
            self.fields
                .iter()
                .map(|Spanned { value: field, span }| {
                    let field_entity = EntityData::MemberName {
                        base: entity,
                        kind: MemberKind::Field,
                        id: field.name.value,
                    }
                    .intern(&db);

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

    fn parse_generic_declarations(
        &self,
        _entity: Entity,
        _db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<Arc<ty::GenericDeclarations>, ErrorReported>> {
        // FIXME -- no support for generics yet
        WithError::ok(Ok(ty::GenericDeclarations::empty(None)))
    }

    fn parse_signature(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<ty::Signature<Declaration>, ErrorReported>> {
        InvalidParsedEntity.parse_signature(entity, db)
    }

    fn parse_type(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<ty::Ty<Declaration>> {
        // For each struct `Foo`, the "type" is just `own Foo`
        match db.generic_declarations(entity).into_value() {
            Ok(generic_declarations) => {
                assert!(generic_declarations.is_empty());
                let ty = crate::type_conversion::declaration_ty_named(
                    &db,
                    entity,
                    ty::declaration::DeclaredPermKind::Own,
                    ty::ReprKind::Direct,
                    ty::Generics::empty(),
                );
                WithError::ok(ty)
            }
            Err(err) => WithError::error_sentinel(&db, err),
        }
    }

    fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody> {
        panic!(
            "cannot parse fn body of a struct: {:?}",
            entity.debug_with(db)
        )
    }
}
