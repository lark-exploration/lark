use crate::ir::DefId;
use crate::query::Query;
use std::sync::Arc;

crate trait TypeckQueryContext: crate::query::BaseQueryContext {
    query_prototype!(
        /// Find the fields of a struct.
        fn fields() for Fields
    );

    query_prototype!(
        /// Find the type of something.
        fn ty() for Ty
    );
}

query_definition! {
    /// Test documentation.
    crate Fields(_: &impl TypeckQueryContext, _: DefId) -> Arc<Vec<DefId>> {
        Arc::new(vec![])
    }
}

#[derive(Default, Debug)]
crate struct Ty;

impl<QC> Query<QC> for Ty
where
    QC: TypeckQueryContext,
{
    type Key = DefId;
    type Value = Arc<Vec<DefId>>;
    type Storage = crate::query::storage::MemoizedStorage<QC, Self>;

    fn execute(query: &QC, key: DefId) -> Arc<Vec<DefId>> {
        query.ty().of(key)
    }
}
