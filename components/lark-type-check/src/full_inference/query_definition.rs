use crate::full_inference::datafrog;
use crate::full_inference::resolve_to_full_inferred::ResolveToFullInferred;
use crate::full_inference::type_checker::FullInferenceStorage;
use crate::full_inference::FullInference;
use crate::full_inference::FullInferenceTables;
use crate::results::TypeCheckResults;
use crate::TypeCheckDatabase;
use crate::TypeChecker;
use crate::UniverseBinder;
use generational_arena::Arena;
use lark_collections::FxIndexMap;
use lark_entity::Entity;
use lark_error::Diagnostic;
use lark_error::WithError;
use lark_indices::IndexVec;
use lark_ty::full_inferred::FullInferred;
use lark_ty::map_family::Map;
use lark_unify::UnificationTable;
use std::sync::Arc;

crate fn full_type_check(
    db: &impl TypeCheckDatabase,
    fn_entity: Entity,
) -> WithError<Arc<TypeCheckResults<FullInferred>>> {
    let fn_body = db.fn_body(fn_entity).into_value();
    let interners = FullInferenceTables::default();
    let mut type_checker: TypeChecker<'_, _, FullInference, _> = TypeChecker {
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

    type_checker.check_fn_body();

    let perm_kinds = datafrog::inference(&type_checker, &type_checker.storage.constraints);

    let mut unresolved_variables = vec![];
    let inferred_results = type_checker
        .storage
        .results
        .map(&mut ResolveToFullInferred::new(
            &mut type_checker.unify,
            &interners,
            type_checker.db.as_ref(),
            &mut unresolved_variables,
            &perm_kinds,
        ));

    let mut errors = type_checker.errors;
    for _ in unresolved_variables {
        // FIXME: Decent diagnostics for unresolved inference
        // variables.
        errors.push(Diagnostic::new(
            "Unresolved variable".into(),
            fn_body.span(fn_body.root_expression),
        ));
    }

    WithError {
        value: Arc::new(inferred_results),
        errors,
    }
}
