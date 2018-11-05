use crate::resolve_to_base_inferred::ResolveToBaseInferred;
use crate::TypeCheckDatabase;
use crate::TypeCheckResults;
use crate::TypeChecker;
use crate::UniverseBinder;
use generational_arena::Arena;
use indices::IndexVec;
use lark_entity::Entity;
use lark_error::{Diagnostic, WithError};
use lark_ty::base_inference::{BaseOnly, BaseOnlyTables};
use lark_ty::base_inferred::BaseInferred;
use lark_ty::map_family::Map;
use lark_unify::InferVar;
use lark_unify::UnificationTable;
use map::FxIndexMap;
use std::sync::Arc;

crate fn base_type_check(
    db: &impl TypeCheckDatabase,
    fn_entity: Entity,
) -> WithError<Arc<TypeCheckResults<BaseInferred>>> {
    let fn_body = db.fn_body(fn_entity).into_value();
    let interners = BaseOnlyTables::default();
    let mut base_type_checker: TypeChecker<'_, _, BaseOnly> = TypeChecker {
        db,
        fn_entity,
        f_tables: interners.clone(),
        hir: fn_body.clone(),
        ops_arena: Arena::new(),
        ops_blocked: FxIndexMap::default(),
        unify: UnificationTable::new(interners.clone()),
        results: TypeCheckResults::default(),
        universe_binders: IndexVec::from(vec![UniverseBinder::Root]),
        errors: vec![],
    };

    // Run the base type-check.
    base_type_checker.check_fn_body();

    // Complete all deferred type operations; run to steady state.
    loop {
        let vars: Vec<InferVar> = base_type_checker.unify.drain_events().collect();
        if vars.is_empty() {
            break;
        }
        for var in vars {
            base_type_checker.trigger_ops(var);
        }
    }

    let mut unresolved_variables = vec![];

    // Look for any deferred operations that never executed. Those
    // variables that they are blocked on must not be resolved; record
    // as an error.
    base_type_checker.untriggered_ops(&mut unresolved_variables);

    // Record the final results. If any unresolved type variables are
    // encountered, report an error.
    let inferred_results = base_type_checker
        .results
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
