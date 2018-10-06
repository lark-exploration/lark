use crate::hir;
use crate::ty;
use crate::ty::declaration::Declaration;
use crate::ty::interners::TyInternTables;
use crate::ty::map_family::Map;
use crate::ty::BaseData;
use crate::ty::Ty;
use crate::ty::TypeFamily;
use crate::type_check::TypeCheckDatabase;
use crate::type_check::TypeCheckFamily;
use crate::type_check::TypeChecker;
use crate::unify::InferVar;
use crate::unify::Inferable;
use generational_arena::Arena;

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

impl<DB, F> TypeChecker<'q, DB, F>
where
    DB: crate::type_check::TypeCheckDatabase,
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

    pub(super) fn substitute<M>(
        &mut self,
        location: hir::MetaIndex,
        owner_perm: F::Perm,
        owner_base_data: &BaseData<F>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, F>,
    {
        F::substitute(self, location, owner_perm, owner_base_data, value)
    }

    /// If `base` can be mapped to a concrete `BaseData`,
    /// invokes `op` and returns the resulting type.
    /// Otherwise, creates a type variable and returns that;
    /// once `base` can be mapped, the closure `op` will be
    /// invoked and the type variable will be unified.
    pub(super) fn with_base_data(
        &mut self,
        cause: hir::MetaIndex,
        base: F::TcBase,
        op: impl FnOnce(&mut Self, BaseData<F>) -> Ty<F> + 'static,
    ) -> Ty<F> {
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
