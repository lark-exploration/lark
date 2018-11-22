use crate::macros::EntityMacroDefinition;
use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::entity::ErrorParsedEntity;
use crate::syntax::entity::LazyParsedEntity;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::entity::ParsedEntity;
use crate::syntax::entity::ParsedEntityThunk;
use crate::syntax::field::Field;
use crate::syntax::field::ParsedField;
use crate::syntax::fn_body;
use crate::syntax::guard::Guard;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::list::CommaList;
use crate::syntax::matched::{Matched, ParsedMatch};
use crate::syntax::sigil::{Curlies, Parentheses, RightArrow};
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::type_reference::ParsedTypeReference;
use crate::syntax::type_reference::TypeReference;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::ItemKind;
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_error::ResultExt;
use lark_error::WithError;
use lark_hir as hir;
use lark_seq::Seq;
use lark_span::FileName;
use lark_span::Spanned;
use lark_string::GlobalIdentifier;
use lark_ty as ty;
use lark_ty::declaration::Declaration;
use lark_ty::GenericDeclarations;
use std::sync::Arc;

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
    ) -> WithError<Seq<ParsedEntity>> {
        WithError::ok(Seq::default())
    }

    fn parse_generic_declarations(
        &self,
        _entity: Entity,
        _db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<Arc<GenericDeclarations>, ErrorReported>> {
        // FIXME -- no support for generics yet
        WithError::ok(Ok(GenericDeclarations::empty(None)))
    }

    fn parse_type(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<ty::Ty<Declaration>> {
        // For each function `foo`, create a unique type `foo` as in
        // Rust.
        match db.generic_declarations(entity).into_value() {
            Ok(generic_declarations) => {
                assert!(generic_declarations.is_empty());
                let ty = crate::type_conversion::declaration_ty_named(
                    &db,
                    entity,
                    ty::ReprKind::Direct,
                    ty::Generics::empty(),
                );
                WithError::ok(ty)
            }
            Err(err) => WithError::error_sentinel(&db, err),
        }
    }

    fn parse_signature(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<ty::Signature<Declaration>, ErrorReported>> {
        let mut errors = vec![];

        let inputs: Seq<_> = self
            .parameters
            .iter()
            .map(|p| {
                p.ty.parse_type(entity, db)
                    .accumulate_errors_into(&mut errors)
            })
            .collect();

        let output = self
            .return_type
            .parse_type(entity, db)
            .accumulate_errors_into(&mut errors);

        WithError {
            value: Ok(ty::Signature { inputs, output }),
            errors,
        }
    }

    fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody> {
        match self.body {
            Err(err) => ErrorParsedEntity { err }.parse_fn_body(entity, db),

            Ok(Spanned {
                span: _,
                value:
                    ParsedMatch {
                        start_token,
                        end_token,
                    },
            }) => {
                let file_name = entity.untern(&db).file_name(&db).unwrap();
                let input = db.file_text(file_name);
                let tokens = db
                    .file_tokens(file_name)
                    .into_value()
                    .extract(start_token..end_token);
                let entity_macro_definitions = crate::macro_definitions(&db, entity);
                let arguments: Seq<_> = self.parameters.iter().map(|f| f.value.name).collect();
                fn_body::parse_fn_body(
                    entity,
                    db,
                    &entity_macro_definitions,
                    &input,
                    &tokens,
                    arguments,
                )
            }
        }
    }
}
