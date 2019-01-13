use crate::parser::Parser;
use crate::syntax::entity::InvalidParsedEntity;
use crate::syntax::entity::LazyParsedEntity;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::entity::ParsedEntity;
use crate::syntax::fn_signature::FunctionSignature;
use crate::syntax::fn_signature::ParsedFunctionSignature;
use crate::syntax::guard::Guard;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::sigil::Colon;
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::type_reference::{ParsedTypeReference, TypeReference};
use crate::syntax::Syntax;
use lark_collections::Seq;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_error::ResultExt;
use lark_error::WithError;
use lark_hir as hir;
use lark_intern::Intern;
use lark_intern::Untern;
use lark_span::FileName;
use lark_span::Spanned;
use lark_string::GlobalIdentifier;
use lark_ty as ty;
use lark_ty::declaration::Declaration;
use std::sync::Arc;

#[derive(DebugWith)]
pub struct Member;

impl Syntax<'parse> for Member {
    type Data = Spanned<ParsedMember, FileName>;

    fn test(&mut self, parser: &Parser<'_>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(&mut self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        let name = parser.expect(SpannedGlobalIdentifier)?;

        if let Some(ty) =
            parser.parse_if_present(SkipNewline(Guard(Colon, SkipNewline(TypeReference))))
        {
            let span = name.span.extended_until_end_of(parser.last_span());
            let ty = ty.unwrap_or_error_sentinel(&*parser);

            return Ok(Spanned {
                value: ParsedMember::ParsedField(ParsedField { name, ty }),
                span,
            });
        }

        let signature = parser.expect(FunctionSignature)?;
        let span = name.span.extended_until_end_of(parser.last_span());

        return Ok(Spanned {
            value: ParsedMember::ParsedMethod(ParsedMethod { name, signature }),
            span,
        });
    }
}

#[derive(DebugWith)]
pub struct Field;

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

pub enum ParsedMember {
    ParsedMethod(ParsedMethod),
    ParsedField(ParsedField),
}

/// Represents a parse of something like `foo: Type`
#[derive(Clone, DebugWith)]
pub struct ParsedMethod {
    pub name: Spanned<GlobalIdentifier, FileName>,
    pub signature: ParsedFunctionSignature,
}

impl LazyParsedEntity for ParsedMethod {
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
    ) -> WithError<Result<Arc<ty::GenericDeclarations>, ErrorReported>> {
        WithError::ok(Ok(ty::GenericDeclarations::empty(None)))
    }

    fn parse_type(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<ty::Ty<Declaration>> {
        // For each method `foo`, create a unique type `foo` as in
        // Rust.
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

    fn parse_signature(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<ty::Signature<Declaration>, ErrorReported>> {
        let parent_entity = entity.untern(&db).parent().unwrap();
        let parent_ty = db.ty(parent_entity).into_value();
        self.signature.parse_signature(entity, db, Some(parent_ty))
    }

    fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody> {
        let self_argument: GlobalIdentifier = "self".intern(&db);
        let spanned_self_argument = Spanned {
            value: self_argument,
            span: self.name.span,
        };
        self.signature
            .parse_fn_body(entity, db, Some(spanned_self_argument))
    }
}

/// Represents a parse of something like `foo: Type`
#[derive(Copy, Clone, DebugWith)]
pub struct ParsedField {
    pub name: Spanned<GlobalIdentifier, FileName>,
    pub ty: ParsedTypeReference,
}

impl LazyParsedEntity for ParsedField {
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
    ) -> WithError<Result<Arc<ty::GenericDeclarations>, ErrorReported>> {
        WithError::ok(Ok(ty::GenericDeclarations::empty(None)))
    }

    fn parse_type(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<ty::Ty<Declaration>> {
        self.ty.parse_type(entity, db)
    }

    fn parse_signature(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<ty::Signature<Declaration>, ErrorReported>> {
        InvalidParsedEntity.parse_signature(entity, db)
    }

    fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody> {
        InvalidParsedEntity.parse_fn_body(entity, db)
    }
}
