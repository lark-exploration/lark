use crate::ParserDatabase;

use lark_entity::Entity;
use lark_error::WithError;
use lark_hir::FnBody;
use std::sync::Arc;

crate fn fn_body(_db: &impl ParserDatabase, _key: Entity) -> WithError<Arc<FnBody>> {
    unimplemented!()
}
