use ast::item_id::ItemId;
use crate::TypeCheckFamily;
use crate::TypeChecker;
use crate::UniverseBinder;
use hir;
use ty::declaration::Declaration;
use ty::interners::TyInternTables;
use ty::map_family::Map;
use ty::BaseData;
use ty::GenericDeclarations;
use ty::GenericKind;
use ty::Generics;
use ty::Placeholder;
use ty::Ty;
use ty::Universe;
use unify::InferVar;
use unify::Inferable;

#[derive(Copy, Clone, Debug)]
pub(super) struct OpIndex {
    index: generational_arena::Index,
}

pub(super) trait BoxedTypeCheckerOp<TypeCheck> {
    fn execute(self: Box<Self>, typeck: &mut TypeCheck);
}

struct ClosureTypeCheckerOp<C> {
    closure: C,
}

impl<C, TypeCheck> BoxedTypeCheckerOp<TypeCheck> for ClosureTypeCheckerOp<C>
where
    C: FnOnce(&mut TypeCheck),
{
    fn execute(self: Box<Self>, typeck: &mut TypeCheck) {
        (self.closure)(typeck)
    }
}

impl<DB, F> TypeChecker<'_, DB, F>
where
    DB: crate::TypeCheckDatabase,
    F: TypeCheckFamily,
{
    pub(super) fn new_infer_ty(&mut self) -> Ty<F> {
        F::new_infer_ty(self)
    }

    pub(super) fn equate_types(&mut self, cause: hir::MetaIndex, ty1: Ty<F>, ty2: Ty<F>) {
        F::equate_types(self, cause, ty1, ty2)
    }

    pub(super) fn boolean_type(&self) -> Ty<F> {
        F::boolean_type(self)
    }

    pub(super) fn error_type(&self) -> Ty<F> {
        F::error_type(self)
    }

    pub(super) fn substitute<M>(
        &mut self,
        location: impl Into<hir::MetaIndex>,
        generics: &Generics<F>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, F>,
    {
        F::substitute(self, location.into(), generics, value)
    }

    pub(super) fn apply_owner_perm<M>(
        &mut self,
        location: impl Into<hir::MetaIndex>,
        owner_perm: F::Perm,
        value: M,
    ) -> M::Output
    where
        M: Map<F, F>,
    {
        F::apply_owner_perm(self, location, owner_perm, value)
    }

    pub(super) fn require_assignable(
        &mut self,
        expression: hir::Expression,
        value_ty: Ty<F>,
        place_ty: Ty<F>,
    ) {
        F::require_assignable(self, expression, value_ty, place_ty)
    }

    pub(super) fn apply_user_perm(&mut self, perm: hir::Perm, place_ty: Ty<F>) -> Ty<F> {
        F::apply_user_perm(self, perm, place_ty)
    }

    pub(super) fn own_perm(&mut self) -> F::Perm {
        F::own_perm(self)
    }

    pub(super) fn least_upper_bound(
        &mut self,
        if_expression: hir::Expression,
        true_ty: Ty<F>,
        false_ty: Ty<F>,
    ) -> Ty<F> {
        F::least_upper_bound(self, if_expression, true_ty, false_ty)
    }

    pub(super) fn placeholders_for(&mut self, def_id: ItemId) -> Generics<F> {
        let GenericDeclarations {
            parent_item,
            declarations,
        } = &*self.db.generic_declarations(def_id);

        let mut generics = match parent_item {
            Some(def_id) => self.placeholders_for(*def_id),
            None => Generics::empty(),
        };

        if !declarations.is_empty() {
            let universe = self.fresh_universe(UniverseBinder::FromItem(def_id));
            generics.extend(
                declarations
                    .indices()
                    .map(|bound_var| Placeholder {
                        universe,
                        bound_var,
                    })
                    .map(|p| {
                        GenericKind::Ty(Ty {
                            perm: self.own_perm(),
                            base: F::intern_base_data(self, BaseData::from_placeholder(p)),
                        })
                    }),
            );
        }

        generics
    }

    /// Create a fresh universe (one that did not exist before) with
    /// the given binder. This universe will be able to see names
    /// from all previously existing universes.
    fn fresh_universe(&mut self, binder: UniverseBinder) -> Universe {
        self.universe_binders.push(binder)
    }

    /// If `base` can be mapped to a concrete `BaseData`,
    /// invokes `op` and returns the resulting type.
    /// Otherwise, creates a type variable and returns that;
    /// once `base` can be mapped, the closure `op` will be
    /// invoked and the type variable will be unified.
    pub(super) fn with_base_data(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        base: F::TcBase,
        op: impl FnOnce(&mut Self, BaseData<F>) -> Ty<F> + 'static,
    ) -> Ty<F> {
        let cause = cause.into();
        match self.unify.shallow_resolve_data(base) {
            Ok(data) => op(self, data),

            Err(_) => {
                let var: Ty<F> = self.new_infer_ty();
                self.with_base_data_unify_with(cause, base, var, op);
                var
            }
        }
    }

    pub(super) fn with_base_data_unify_with(
        &mut self,
        cause: hir::MetaIndex,
        base: F::TcBase,
        output_ty: Ty<F>,
        op: impl FnOnce(&mut Self, BaseData<F>) -> Ty<F> + 'static,
    ) {
        match self.unify.shallow_resolve_data(base) {
            Ok(data) => {
                let ty1 = op(self, data);
                self.equate_types(cause, output_ty, ty1);
            }

            Err(_) => self.enqueue_op(Some(base), move |this| {
                this.with_base_data_unify_with(cause, base, output_ty, op)
            }),
        }
    }

    /// Enqueues a closure to execute when any of the
    /// variables in `values` are unified.
    pub(super) fn enqueue_op(
        &mut self,
        values: impl IntoIterator<Item = impl Inferable<TyInternTables>>,
        closure: impl FnOnce(&mut Self) + 'static,
    ) {
        let op: Box<dyn BoxedTypeCheckerOp<Self>> = Box::new(ClosureTypeCheckerOp { closure });
        let op_index = OpIndex {
            index: self.ops_arena.insert(op),
        };
        let mut inserted = false;
        for infer_value in values {
            // Check if `infer_value` represents an unbound inference variable.
            if let Err(var) = self.unify.shallow_resolve_data(infer_value) {
                // As yet unbound. Enqueue this op to be notified when
                // it does get bound.
                self.ops_blocked.entry(var).or_insert(vec![]).push(op_index);
                inserted = true;
            }
        }
        assert!(
            inserted,
            "enqueued an op with no unknown inference variables"
        );
    }

    /// Executes any closures that are blocked on `var`.
    pub(super) fn trigger_ops(&mut self, var: InferVar) {
        let blocked_ops = self.ops_blocked.remove(&var).unwrap_or(vec![]);
        for OpIndex { index } in blocked_ops {
            match self.ops_arena.remove(index) {
                None => {
                    // The op may already have been removed. This occurs
                    // when -- for example -- the same op is blocked on multiple variables.
                    // In that case, just ignore it.
                }

                Some(op) => {
                    op.execute(self);
                }
            }
        }
    }
}
