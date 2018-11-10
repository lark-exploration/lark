use crate::span::CurrentFile;
use crate::span::Span;
use lark_entity::Entity;
use std::sync::Arc;

crate struct ParsedEntity {
    /// The `Entity` identifier by which we are known.
    entity: Entity,

    /// The span of the entire entity.
    full_span: Span<CurrentFile>,

    /// A (sometimes) shorter span that can be used to highlight this
    /// entity in error messages. For example, for a method, it might
    /// be the method name -- this helps to avoid multi-line error
    /// messages, which are kind of a pain.
    characteristic_span: Span<CurrentFile>,

    /// The "thunk" contains methods that will recursively parse the
    /// contents of this entity in response to salsa queries (or, if
    /// the contents are already parsed, return pre-parsed bits and
    /// pieces). These routines are meant to be "purely functional",
    /// but the salsa runtime will memoize and ensure they are not
    /// reinvoked.
    thunk: Arc<dyn LazyParsedEntity>,
}

impl ParsedEntity {
    crate fn new<T: 'static + LazyParsedEntity>(
        entity: Entity,
        full_span: Span<CurrentFile>,
        characteristic_span: Span<CurrentFile>,
        thunk: Arc<T>,
    ) -> Self {
        Self {
            entity,
            full_span,
            characteristic_span,
            thunk,
        }
    }
}

crate trait LazyParsedEntity {
    fn parse_children(&self) -> Vec<ParsedEntity>;
}

crate struct ErrorParsedEntity;

impl LazyParsedEntity for ErrorParsedEntity {
    fn parse_children(&self) -> Vec<ParsedEntity> {
        vec![]
    }
}
