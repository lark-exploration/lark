use crate::TypeChecker;
use crate::TypeCheckerFamily;
use crate::TypeCheckerFamilyDependentExt;
use crate::UniverseBinder;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::LangItem;
use lark_error::{Diagnostic, ErrorReported};
use lark_hir as hir;
use lark_intern::Intern;
use lark_ty::BaseData;
use lark_ty::BaseKind;
use lark_ty::GenericDeclarations;
use lark_ty::GenericKind;
use lark_ty::Generics;
use lark_ty::Placeholder;
use lark_ty::Ty;
use lark_ty::Universe;
use lark_unify::InferVar;
use lark_unify::Inferable;
use std::sync::Arc;

#[derive(Copy, Clone, Debug)]
crate struct OpIndex {
    index: generational_arena::Index,
}

crate trait BoxedTypeCheckerOp<TypeCheck> {
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

impl<DB, F, S> TypeChecker<'_, DB, F, S>
where
    DB: crate::TypeCheckDatabase,
    F: TypeCheckerFamily,
    Self: TypeCheckerFamilyDependentExt<F>,
    F::Base: Inferable<F::InternTables, KnownData = BaseData<F>>,
{
    crate fn boolean_type(&self) -> Ty<F> {
        self.primitive_type(LangItem::Boolean)
    }

    crate fn int_type(&self) -> Ty<F> {
        self.primitive_type(LangItem::Int)
    }

    crate fn uint_type(&self) -> Ty<F> {
        self.primitive_type(LangItem::Uint)
    }

    crate fn string_type(&self) -> Ty<F> {
        self.primitive_type(LangItem::String)
    }

    crate fn unit_type(&self) -> Ty<F> {
        self.primitive_type(LangItem::Tuple(0))
    }

    crate fn error_type(&self) -> Ty<F> {
        F::error_type(self)
    }

    fn primitive_type(&self, item: LangItem) -> Ty<F> {
        let entity = EntityData::LangItem(item).intern(self);
        Ty {
            repr: F::direct_repr(self),
            perm: F::own_perm(self),
            base: F::intern_base_data(
                self,
                BaseData {
                    kind: BaseKind::Named(entity),
                    generics: Generics::empty(),
                },
            ),
        }
    }

    /// Record that an error occurred at the given location.
    crate fn record_error(
        &mut self,
        label: impl Into<String>,
        location: impl Into<hir::MetaIndex>,
    ) {
        let span = self.hir.span(location.into());
        self.errors.push(Diagnostic::new(label.into(), span));
    }

    crate fn own_perm(&mut self) -> F::Perm {
        F::own_perm(self)
    }

    crate fn direct_repr(&mut self) -> F::Repr {
        F::direct_repr(self)
    }

    /// Unifies all of the generic arguments from `data` with the
    /// error type.
    crate fn propagate_error(&mut self, cause: impl Into<hir::MetaIndex>, generics: &Generics<F>) {
        let cause = cause.into();
        let error_type = self.error_type();
        for generic in generics.iter() {
            match generic {
                GenericKind::Ty(ty) => self.equate_types(cause, error_type, ty),
            }
        }
    }

    crate fn placeholders_for(&mut self, def_id: Entity) -> Generics<F> {
        let GenericDeclarations {
            parent_item,
            declarations,
        } = &*self
            .db
            .generic_declarations(def_id)
            .into_value()
            .unwrap_or_else(|ErrorReported(_)| Arc::new(GenericDeclarations::default()));

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
                            repr: self.direct_repr(),
                            perm: self.own_perm(),
                            base: F::intern_base_data(self, BaseData::from_placeholder(p)),
                        })
                    }),
            );
        }

        generics
    }

    crate fn inference_variables_for(&mut self, entity: Entity) -> Generics<F> {
        let GenericDeclarations {
            parent_item,
            declarations,
        } = &*self
            .db
            .generic_declarations(entity)
            .into_value()
            .unwrap_or_else(|ErrorReported(_)| Arc::new(GenericDeclarations::default()));

        // If the generics for `entity` extend those of its parent,
        // first create the parent's generics.
        let mut generics = match parent_item {
            Some(entity) => self.inference_variables_for(*entity),
            None => Generics::empty(),
        };

        // Now extend with our own.
        if !declarations.is_empty() {
            generics.extend(
                declarations
                    .indices()
                    .map(|_| GenericKind::Ty(self.new_infer_ty())),
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
    crate fn with_base_data(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        base: F::Base,
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

    fn with_base_data_unify_with(
        &mut self,
        cause: hir::MetaIndex,
        base: F::Base,
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
    crate fn enqueue_op(
        &mut self,
        values: impl IntoIterator<Item = impl Inferable<F::InternTables>>,
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
    crate fn trigger_ops(&mut self, var: InferVar) {
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

    /// Records any inference variables that are have
    /// not-yet-triggered operations. These must all be currently
    /// unresolved.
    crate fn untriggered_ops(&mut self, output: &mut Vec<InferVar>) {
        'var_loop: for (&var, blocked_ops) in &self.ops_blocked {
            assert!(!self.unify.var_is_known(var));
            for &OpIndex { index } in blocked_ops {
                if self.ops_arena.contains(index) {
                    output.push(var);
                    continue 'var_loop;
                }
            }
        }
    }
}
