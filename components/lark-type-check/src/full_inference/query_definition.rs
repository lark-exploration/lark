use crate::full_inference::datafrog;
use crate::full_inference::type_checker::FullInferenceStorage;
use crate::full_inference::{FullInference, FullInferenceTables};
use crate::TypeCheckDatabase;
use crate::TypeCheckResults;
use crate::TypeChecker;
use crate::UniverseBinder;
use generational_arena::Arena;
use lark_collections::FxIndexMap;
use lark_entity::Entity;
use lark_error::WithError;
use lark_indices::IndexVec;
use lark_ty::full_inferred::FullInferred;
use lark_unify::UnificationTable;
use std::sync::Arc;

crate fn full_type_check(
    db: &impl TypeCheckDatabase,
    fn_entity: Entity,
) -> WithError<Arc<TypeCheckResults<FullInferred>>> {
    let fn_body = db.fn_body(fn_entity).into_value();
    let interners = FullInferenceTables::default();
    let mut full_type_checker: TypeChecker<'_, _, FullInference, _> = TypeChecker {
        db,
        fn_entity,
        f_tables: interners.clone(),
        hir: fn_body.clone(),
        ops_arena: Arena::new(),
        ops_blocked: FxIndexMap::default(),
        unify: UnificationTable::new(interners.clone()),
        storage: FullInferenceStorage::default(),
        universe_binders: IndexVec::from(vec![UniverseBinder::Root]),
        errors: vec![],
    };

    full_type_checker.check_fn_body();

    let results = datafrog::inference(&full_type_checker, &full_type_checker.storage.constraints);

    unimplemented!()
}
