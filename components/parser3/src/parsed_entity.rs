use crate::span::CurrentFile;
use crate::span::Span;
use lark_entity::Entity;
use std::sync::Arc;

crate struct ParsedEntity {
    entity: Entity,
    span: Span<CurrentFile>,
    thunk: Arc<dyn LazyParsedEntity>,
}

impl ParsedEntity {
    crate fn new<T: 'static + LazyParsedEntity>(
        entity: Entity,
        span: Span<CurrentFile>,
        thunk: Arc<T>,
    ) -> Self {
        Self {
            entity,
            span,
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
