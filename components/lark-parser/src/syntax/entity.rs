use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::NonEmptySyntax;
use crate::syntax::Syntax;
use crate::ParserDatabase;
use debug::DebugWith;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityTables;
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_error::WithError;
use lark_hir as hir;
use lark_seq::Seq;
use lark_span::FileName;
use lark_span::Span;
use lark_span::Spanned;
use lark_string::text::Text;
use lark_string::GlobalIdentifier;
use lark_string::GlobalIdentifierTables;
use lark_ty::declaration::Declaration;
use lark_ty::declaration::DeclarationTables;
use lark_ty::GenericDeclarations;
use lark_ty::Ty;
use std::sync::Arc;

#[derive(DebugWith)]
pub struct EntitySyntax {
    parent_entity: Entity,
}

impl EntitySyntax {
    pub fn new(parent_entity: Entity) -> Self {
        EntitySyntax { parent_entity }
    }
}

impl Syntax<'parse> for EntitySyntax {
    type Data = ParsedEntity;

    fn test(&mut self, parser: &Parser<'_>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(&mut self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        // Parse the macro keyword, which we must find first. So something like
        //
        // ```
        // struct Foo { ... }
        // ^^^^^^ ----------- parsed by the macro itself
        // |
        // parsed by us
        // ```

        let macro_name = parser.expect(SpannedGlobalIdentifier)?;

        log::debug!(
            "EntitySyntax::parse(macro_name = {:?})",
            macro_name.debug_with(parser),
        );

        let macro_definition = match parser.entity_macro_definitions().get(&macro_name.value) {
            Some(m) => m.clone(),
            None => Err(parser.report_error("no macro with this name", macro_name.span))?,
        };

        Ok(macro_definition.expect(parser, self.parent_entity, macro_name)?)
    }
}

impl NonEmptySyntax<'parse> for EntitySyntax {}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq)]
pub struct ParsedEntity {
    /// The `Entity` identifier by which we are known.
    pub entity: Entity,

    /// The span of the entire entity.
    pub full_span: Span<FileName>,

    /// A (sometimes) shorter span that can be used to highlight this
    /// entity in error messages. For example, for a method, it might
    /// be the method name -- this helps to avoid multi-line error
    /// messages, which are kind of a pain.
    pub characteristic_span: Span<FileName>,

    /// Thunk to extract contents
    pub thunk: ParsedEntityThunk,
}

impl ParsedEntity {
    crate fn new(
        entity: Entity,
        full_span: Span<FileName>,
        characteristic_span: Span<FileName>,
        thunk: ParsedEntityThunk,
    ) -> Self {
        Self {
            entity,
            full_span,
            characteristic_span,
            thunk,
        }
    }
}

/// The "parsed entity thunk" contains methods that will recursively
/// parse the contents of this entity in response to salsa queries
/// (or, if the contents are already parsed, return pre-parsed bits
/// and pieces). These routines are meant to be "purely functional",
/// but the salsa runtime will memoize and ensure they are not
/// reinvoked.
#[derive(Clone)]
pub struct ParsedEntityThunk {
    object: Arc<dyn LazyParsedEntity + Send + Sync>,
}

impl ParsedEntityThunk {
    pub fn new<T: 'static + LazyParsedEntity + Send + Sync>(object: T) -> Self {
        Self {
            object: Arc::new(object),
        }
    }

    /// See [`LazyParsedEntity::parse_children`]
    crate fn parse_children(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Vec<ParsedEntity>> {
        self.object.parse_children(entity, db)
    }

    /// See [`LazyParsedEntity::parse_generic_declarations`]
    crate fn parse_generic_declarations(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<Arc<GenericDeclarations>, ErrorReported>> {
        self.object.parse_generic_declarations(entity, db)
    }

    /// See [`LazyParsedEntity::parse_type`]
    crate fn parse_type(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Ty<Declaration>> {
        self.object.parse_type(entity, db)
    }

    /// See [`LazyParsedEntity::parse_fn_body`]
    crate fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody> {
        self.object.parse_fn_body(entity, db)
    }
}

impl std::fmt::Debug for ParsedEntityThunk {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("LazyParsedEntity").finish()
    }
}

impl std::cmp::PartialEq for ParsedEntityThunk {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.object, &other.object)
    }
}

impl std::cmp::Eq for ParsedEntityThunk {}

debug::debug_fallback_impl!(ParsedEntityThunk);

pub trait LazyParsedEntity {
    /// Parse the children of this entity.
    ///
    /// # Parameters
    ///
    /// - `entity`: the entity id of self
    /// - `db`: the necessary bits/pieces of the parser database
    fn parse_children(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Vec<ParsedEntity>>;

    /// What "generic declarations" are on this entity? e.g., for
    /// `struct Foo<T> { .. }`, this would return the declaration for
    /// `T`.
    fn parse_generic_declarations(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<Arc<GenericDeclarations>, ErrorReported>>;

    /// What is the "declared type" of this entity? If the entity has
    /// generic types, this type will include bound types referring to
    /// those generics. e.g., for `struct Foo<T> { .. }`, the result
    /// would be effectively `Foo<T>`.
    fn parse_type(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Ty<Declaration>>;

    /// Parses the fn body associated with this entity,
    /// panicking if there is none.
    ///
    /// # Parameters
    ///
    /// - `entity`: the entity id of self
    /// - `db`: the necessary bits/pieces of the parser database
    fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody>;
}

crate struct ErrorParsedEntity {
    crate err: ErrorReported,
}

impl LazyParsedEntity for ErrorParsedEntity {
    fn parse_children(
        &self,
        _entity: Entity,
        _db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Vec<ParsedEntity>> {
        WithError::ok(vec![])
    }

    fn parse_generic_declarations(
        &self,
        _entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<Arc<GenericDeclarations>, ErrorReported>> {
        WithError::ok(ErrorSentinel::error_sentinel(db, self.err))
    }

    fn parse_type(
        &self,
        _entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Ty<Declaration>> {
        WithError::ok(ErrorSentinel::error_sentinel(&db, self.err))
    }

    fn parse_fn_body(
        &self,
        _entity: Entity,
        _db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody> {
        unimplemented!()
    }
}

/// Convenience type: implemnts `LazyParsedEntityDatabase` but just
/// panics.  Use as the impl for methods you don't support on a
/// certain kind of entity.
crate struct InvalidParsedEntity;

impl LazyParsedEntity for InvalidParsedEntity {
    fn parse_children(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Vec<ParsedEntity>> {
        panic!(
            "cannot invoke `parse_children` on {:?}",
            entity.debug_with(db)
        )
    }

    fn parse_generic_declarations(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Result<Arc<GenericDeclarations>, ErrorReported>> {
        panic!(
            "cannot invoke `parse_generic_declarations` on {:?}",
            entity.debug_with(db)
        )
    }

    fn parse_type(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<Ty<Declaration>> {
        panic!("cannot invoke `parse_type` on {:?}", entity.debug_with(db))
    }

    fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<hir::FnBody> {
        panic!(
            "cannot invoke `parse_fn_body` on {:?}",
            entity.debug_with(db)
        )
    }
}

/// The trait given to the [`LazyParsedEntity`] methods. It is a "dyn
/// capable" variant of `ParserDatabase`.
pub trait LazyParsedEntityDatabase:
    AsRef<GlobalIdentifierTables> + AsRef<EntityTables> + AsRef<DeclarationTables>
{
    /// Looks up a name `name` to see if it matches any entities in the scope of `item_entity`.
    fn resolve_name(&self, item_entity: Entity, name: GlobalIdentifier) -> Option<Entity>;

    /// The `file_text` query
    fn file_text(&self, id: FileName) -> Text;

    /// The `file_tokens` query
    fn file_tokens(&self, id: FileName) -> WithError<Seq<Spanned<LexToken, FileName>>>;

    /// The `generic_declarations` query
    fn generic_declarations(
        &self,
        entity: Entity,
    ) -> WithError<Result<Arc<GenericDeclarations>, ErrorReported>>;
}

impl<T: ParserDatabase> LazyParsedEntityDatabase for T {
    fn file_text(&self, id: FileName) -> Text {
        ParserDatabase::file_text(self, id)
    }

    fn resolve_name(&self, item_entity: Entity, name: GlobalIdentifier) -> Option<Entity> {
        ParserDatabase::resolve_name(self, item_entity, name)
    }

    fn file_tokens(&self, id: FileName) -> WithError<Seq<Spanned<LexToken, FileName>>> {
        ParserDatabase::file_tokens(self, id)
    }

    fn generic_declarations(
        &self,
        entity: Entity,
    ) -> WithError<Result<Arc<GenericDeclarations>, ErrorReported>> {
        ParserDatabase::generic_declarations(self, entity)
    }
}
