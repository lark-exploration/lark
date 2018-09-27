use crate::intern::Untern;
use crate::ty::intern::{Interners, TyInterners};
use crate::ty::unify::{InferData, InferValue, Rank, RootData, UnificationTable};
use crate::ty::unify::{Value, ValueData};
use crate::ty::InferVar;
use std::convert::TryFrom;

// Core union-find algorithms.

impl UnificationTable {
    /// Creates a new inference variable.
    pub(super) fn new_infer_var(&mut self) -> InferVar {
        self.trace.push(None);
        self.infers.push(InferData::Unbound(Rank::default()))
    }

    /// Finds the "root index" associated with `index1`.
    /// In the "union-find" algorithm this is called "find".
    fn find(&mut self, index1: InferVar) -> (InferVar, RootData) {
        match self.infers[index1] {
            InferData::Unbound(rank) => (index1, RootData::Rank(rank)),
            InferData::Value(value1) => (index1, RootData::Value(value1)),
            InferData::Redirect(index2) => {
                let (index3, value3) = self.find(index2);
                if index2 != index3 {
                    // This is the "path compression" step of union-find:InferData
                    // basically, if we were redireced to X, and X was later
                    // redirected to Y, then we should redirect ourselves to Y too.
                    match value3 {
                        RootData::Value(v) => self.infers[index1] = InferData::Value(v),
                        RootData::Rank(_) => self.infers[index1] = InferData::Redirect(index3),
                    }
                }
                (index3, value3)
            }
        }
    }

    /// Finds the root data associated with `index1`; does not do path compression,
    /// so only requires `&self`. Not the fastest thing ever.
    pub(super) fn find_without_path_compression(&self, index1: InferVar) -> (InferVar, RootData) {
        match self.infers[index1] {
            InferData::Unbound(rank) => (index1, RootData::Rank(rank)),
            InferData::Value(value1) => (index1, RootData::Value(value1)),
            InferData::Redirect(index2) => self.find_without_path_compression(index2),
        }
    }

    /// Checks whether `index` has been assigned to a value yet.
    /// If so, returns it.
    pub(super) fn probe(&mut self, index: InferVar) -> Option<Value> {
        let (_root, root_data) = self.find(index);
        root_data.value()
    }

    /// True if `index` has been assigned to a value, false otherwise.
    crate fn is_bound(&mut self, index: InferVar) -> bool {
        self.probe(index).is_some()
    }

    /// Checks whether `index` has been assigned to a value yet.
    /// If so, returns it.
    pub(super) fn probe_data<K>(&mut self, index: InferVar) -> Option<K::Data>
    where
        K: Untern<TyInterners> + TryFrom<ValueData, Error = String>,
    {
        let (_root, root_data) = self.find(index);
        root_data.value().map(|v| {
            let key = K::try_from(self.values[v]).unwrap();
            self.untern(key)
        })
    }

    /// Given two unbound inference variables, unify them for evermore. It is best
    /// **not** to use the variables that result from (e.g.) a `find` operation,
    /// but rather the variables that "arose naturally" when doing inference, because
    /// it helps when issuing blame annotations later.
    pub(super) fn unify_unbound_vars(&mut self, index1: InferVar, index2: InferVar) {
        let (root1, root_data1) = self.find(index1);
        let (root2, root_data2) = self.find(index2);
        let rank1 = root_data1
            .rank()
            .unwrap_or_else(|| panic!("index1 ({:?}) was bound", index1));
        let rank2 = root_data2
            .rank()
            .unwrap_or_else(|| panic!("index2 ({:?}) was bound", index2));

        if rank1 < rank2 {
            self.redirect(index2, root2, rank2, index1, root1, rank1);
        } else {
            self.trace[index1] = Some(index2);
            self.redirect(index1, root1, rank1, index2, root2, rank2);
        }
    }

    pub(super) fn bind_unbound_var_to_value(
        &mut self,
        unbound_var: InferVar,
        value: impl InferValue,
    ) {
        match value.deref(self.interners()) {
            Ok(other_var) => match self.find(other_var) {
                (_, RootData::Rank(_)) => self.unify_unbound_vars(unbound_var, other_var),
                (_, RootData::Value(_)) => {
                    self.bind_unbound_var_to_bound_var(unbound_var, other_var)
                }
            },

            Err(_) => {
                let value_data: ValueData = value.into();
                self.bind_unbound_var_to_value_data(unbound_var, value_data);
            }
        }
    }

    /// Binds `unbound_var`, which must not yet be bound to anything, to `bound_var`, which is.
    fn bind_unbound_var_to_bound_var(&mut self, unbound_var: InferVar, bound_var: InferVar) {
        debug_assert!(self.probe(unbound_var).is_none());
        debug_assert!(self.probe(bound_var).is_some());

        let (root_unbound_var, _) = self.find(unbound_var);
        self.trace[unbound_var] = Some(bound_var);
        self.infers[root_unbound_var] = InferData::Redirect(bound_var);
        self.events.push(unbound_var);
    }

    /// Binds `unbound_var`, which must not yet be bound to anything, to a value.
    fn bind_unbound_var_to_value_data(&mut self, unbound_var: InferVar, value_data: ValueData) {
        debug_assert!(self.probe(unbound_var).is_none());
        let value = self.values.push(value_data.into());
        let (root_unbound_var, _) = self.find(unbound_var);
        self.infers[root_unbound_var] = InferData::Value(value);
        // FIXME: trace information?
        self.events.push(unbound_var);
    }

    /// Redirects the (root) variable `root_from` to another root variable (`root_to`).
    /// Adjusts `root_to`'s rank to indicate its new depth.
    fn redirect(
        &mut self,
        index_from: InferVar,
        root_from: InferVar,
        rank_from: Rank,
        index_to: InferVar,
        root_to: InferVar,
        rank_to: Rank,
    ) {
        assert!(self.trace[index_from].is_none());

        self.trace[index_from] = Some(index_to);
        self.infers[root_from] = InferData::Redirect(root_to);

        // Before we had two trees with depth `rank_from` and `rank_to`.
        // We are making `rank_from` a child of the other tree, so that has depth `rank_from + 1`.
        // This may or may not change the depth of the new root (depending on what its rank was before).
        let rank_max = std::cmp::max(rank_from.next(), rank_to);
        self.infers[root_to] = InferData::Unbound(rank_max);

        self.events.push(root_from);
    }
}
