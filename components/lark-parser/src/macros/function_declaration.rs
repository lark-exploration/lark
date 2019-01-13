use crate::macros::EntityMacroDefinition;
use crate::parser::Parser;
use crate::syntax::entity::LazyParsedEntity;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::entity::ParsedEntity;
use crate::syntax::entity::ParsedEntityThunk;
use crate::syntax::fn_signature::FunctionSignature;
use crate::syntax::fn_signature::ParsedFunctionSignature;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::skip_newline::SkipNewline;
use lark_collections::Seq;
use lark_debug_derive::DebugWith;
use lark_debug_with::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::ItemKind;
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_error::WithError;
use lark_hir as hir;
use lark_intern::Intern;
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

        let signature = parser.expect(FunctionSignature)?;

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
            ParsedEntityThunk::new(ParsedFunctionDeclaration { signature }),
        ))
    }
}

#[derive(Clone, DebugWith)]
pub struct ParsedFunctionDeclaration {
    pub signature: ParsedFunctionSignature,
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
        self.signature.parse_signature(entity, db, None)
    }

    fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody> {
        self.signature.parse_fn_body(entity, db, None)
    }
}
