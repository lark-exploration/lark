use crate::syntax::entity::InvalidParsedEntity;
use crate::syntax::entity::LazyParsedEntity;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::entity::ParsedEntity;

use derive_new::new;
use lark_collections::Seq;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::ErrorReported;
use lark_error::WithError;
use lark_hir as hir;
use lark_span::{FileName, Span};
use lark_ty as ty;
use lark_ty::declaration::Declaration;
use std::sync::Arc;

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, new)]
pub struct ParsedFile {
    pub file_name: FileName,
    pub entities: Seq<ParsedEntity>,
    pub span: Span<FileName>,
}

impl ParsedFile {
    pub fn entities(&self) -> &Seq<ParsedEntity> {
        &self.entities
    }
}

impl LazyParsedEntity for ParsedFile {
    fn parse_children(
        &self,
        _entity: Entity,
        _db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Seq<ParsedEntity>> {
        WithError::ok(self.entities.clone())
    }

    fn parse_generic_declarations(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<Arc<ty::GenericDeclarations>, ErrorReported>> {
        InvalidParsedEntity.parse_generic_declarations(entity, db)
    }

    fn parse_type(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<ty::Ty<Declaration>> {
        InvalidParsedEntity.parse_type(entity, db)
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
