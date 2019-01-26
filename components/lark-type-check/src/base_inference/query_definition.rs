use crate::base_inference::resolve_to_base_inferred::ResolveToBaseInferred;
use crate::base_inference::{BaseInference, BaseInferenceTables};
use crate::results::TypeCheckResults;
use crate::TypeCheckDatabase;
use crate::TypeChecker;
use crate::UniverseBinder;
use generational_arena::Arena;
use lark_collections::{FxIndexMap, IndexVec};
use lark_entity::Entity;
use lark_error::Diagnostic;
use lark_error::WithError;
use lark_ty::base_inferred::BaseInferred;
use lark_ty::map_family::Map;
use lark_unify::UnificationTable;
use std::sync::Arc;

crate fn base_type_check(
    db: &impl TypeCheckDatabase,
    fn_entity: Entity,
) -> WithError<Arc<TypeCheckResults<BaseInferred>>> {
    let fn_body = db.fn_body(fn_entity).into_value();
    let interners = BaseInferenceTables::default();
    let mut base_type_checker: TypeChecker<'_, BaseInference, _> = TypeChecker {
        db,
        fn_entity,
        f_tables: interners.clone(),
        hir: fn_body.clone(),
        ops_arena: Arena::new(),
        ops_blocked: FxIndexMap::default(),
        unify: UnificationTable::new(interners.clone()),
        storage: TypeCheckResults::default(),
        universe_binders: IndexVec::from(vec![UniverseBinder::Root]),
        errors: vec![],
    };

    let mut unresolved_variables = base_type_checker.check_fn_body();

    // Record the final results. If any unresolved type variables are
    // encountered, report an error.
    let inferred_results = base_type_checker
        .storage
        .map(&mut ResolveToBaseInferred::new(
            &mut base_type_checker.unify,
            db.as_ref(),
            &mut unresolved_variables,
        ));

    let mut errors = base_type_checker.errors;
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
