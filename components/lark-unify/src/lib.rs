#![feature(const_fn)]
#![feature(specialization)]

use lark_collections::{IndexVec, U32Index};

lark_collections::index_type! {
    pub struct InferVar {
        debug_name["?"],
        ..
    }
}

lark_debug_with::debug_fallback_impl!(InferVar);

/// Each "inferable" value represents something which can be inferred.
/// For example, the `crate::ty::Perm` and `crate::ty::Base` types implement
/// inferable.
///
/// Each inferable value either corresponds to "known data" or else to an
/// inference variable (this variable may itself be unified etc). The Inferable
/// trait lets us check whether this value represents an inference variable
/// or not (via `as_infer_var`) and also to extract the known data (via `assert_known`).
pub trait Inferable<Interners>: U32Index {
    type KnownData;
    type Data;

    /// Check if this is an inference variable and return the inference
    /// index if so.
    fn as_infer_var(self, interners: &Interners) -> Option<InferVar>;

    /// Create an inferable representing the inference variable `var`.
    fn from_infer_var(var: InferVar, interners: &Interners) -> Self;

    /// Asserts that this is not an inference variable and returns the
    /// "known data" that it represents.
    fn assert_known(self, interners: &Interners) -> Self::KnownData;
}

#[derive(Clone)]
pub struct UnificationTable<Interners, Cause> {
    interners: Interners,

    /// Stores the union-find data for each inference variable.
    /// Used for most efficient lookup.
    infers: IndexVec<InferVar, InferData>,

    /// Stores a more naive trace of which variables were unified
    /// with which. Used for error reporting but makes no effort
    /// to form a balanced tree.
    trace: IndexVec<InferVar, Option<UnificationTrace<Cause>>>,

    /// Each time an inference variable is bound, we push it into
    /// this vector. External watchers can query this list and use it
    /// to track what happened and trigger work.
    events: Vec<InferVar>,
}

#[derive(Clone, Debug)]
struct UnificationTrace<Cause> {
    /// Why did this unification happen?
    cause: Cause,

    /// Were we unified with another unification variable?
    /// (Otherwise, we must have been unified with a root value)
    other_variable: Option<InferVar>,
}

#[derive(Copy, Clone)]
enum InferData {
    /// A root variable that is not yet bound to any value.
    Unbound(Rank),

    /// A variable that is bound to `Value`.
    Value(Value),

    /// A leaf variable that is redirected to another variable
    /// (which may or may not still be the root). This value will
    /// eventually get overwritten with `Value` once the value
    /// is known.
    Redirect(InferVar),
}

/// Rank tracks the maximum height of the unification tree underneath
/// an unbound variable. This is used to maintain a balanced tree
/// during unification.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
    value: u32,
}

impl Rank {
    fn next(self) -> Rank {
        Rank {
            value: self.value + 1,
        }
    }
}

/// Represents some kind of inferable that has a known value.
/// The precise type of the inferable has been stripped; it is known
/// in context based on the type of the key that is used to access it.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Value {
    untyped_index: u32,
}

impl Value {
    /// Create a `Value` from an `Inferable` (erasing its origin type).
    fn cast_from(value: impl U32Index) -> Value {
        Value {
            untyped_index: value.as_u32(),
        }
    }

    /// Cast `Value` into an instance of the inferable type that it came from.
    /// This is an unchecked cast; it relies on the fact that we never unify
    /// values of type `K` with a value of some other type `K'`. So if we lookup
    /// an inference variable of type `K` and find it was bound to some known value,
    /// that value must originally have had type `K`.
    fn cast_to<K: U32Index>(self) -> K {
        K::from_u32(self.untyped_index)
    }
}

#[derive(Copy, Clone, Debug)]
enum RootData {
    Rank(Rank),
    Value(Value),
}

impl RootData {
    fn value(self) -> Option<Value> {
        match self {
            RootData::Rank(_) => None,
            RootData::Value(v) => Some(v),
        }
    }

    fn rank(self) -> Option<Rank> {
        match self {
            RootData::Rank(r) => Some(r),
            RootData::Value(_) => None,
        }
    }
}

impl<Interners, Cause> UnificationTable<Interners, Cause> {
    pub fn new(interners: Interners) -> Self {
        Self {
            interners: interners,
            infers: IndexVec::new(),
            trace: IndexVec::new(),
            events: Vec::new(),
        }
    }

    /// `value` to a known-value, if possible. Else, it must be an inference variable,
    /// so return that `InferVar`.
    pub fn shallow_resolve_data<K>(&mut self, value: K) -> Result<K::KnownData, InferVar>
    where
        K: Inferable<Interners>,
    {
        if let Some(var) = value.as_infer_var(&self.interners) {
            if let Some(value) = self.probe(var) {
                let known_key = value.cast_to::<K>();
                Ok(known_key.assert_known(&self.interners))
            } else {
                Err(var)
            }
        } else {
            Ok(value.assert_known(&self.interners))
        }
    }

    /// True if `index` has been assigned to a value, false otherwise.
    pub fn is_known(&mut self, index: impl Inferable<Interners>) -> bool {
        self.shallow_resolve_data(index).is_ok()
    }

    /// True if `var` has been assigned to a value, false otherwise.
    pub fn var_is_known(&mut self, var: InferVar) -> bool {
        self.probe(var).is_some()
    }

    /// Creates a new inferable thing.
    pub fn new_inferable<K>(&mut self) -> K
    where
        K: Inferable<Interners>,
    {
        let var = self.new_infer_var();
        K::from_infer_var(var, &self.interners)
    }

    /// Read out all the variables that may have been unified
    /// since the last invocation to `drain_events`.
    pub fn drain_events(&mut self) -> impl Iterator<Item = InferVar> + '_ {
        self.events.drain(..)
    }

    /// Tries to unify `key1` and `key2` -- if one or both is an unbound inference variable,
    /// we will record the connection between them. But if they both represent known values,
    /// then we will return the two known values so you can recursively unify those.
    pub fn unify<K>(
        &mut self,
        cause: Cause,
        key1: K,
        key2: K,
    ) -> Result<(), (K::KnownData, K::KnownData)>
    where
        K: Inferable<Interners>,
    {
        match (
            self.shallow_resolve_data(key1),
            self.shallow_resolve_data(key2),
        ) {
            (Ok(kv1), Ok(kv2)) => Err((kv1, kv2)),

            (Err(var1), Err(var2)) => {
                self.unify_unbound_vars(cause, var1, var2);
                Ok(())
            }

            (Err(var1), Ok(_)) => {
                self.bind_unbound_var_to_value(cause, var1, Value::cast_from(key2));
                Ok(())
            }

            (Ok(_), Err(var2)) => {
                self.bind_unbound_var_to_value(cause, var2, Value::cast_from(key1));
                Ok(())
            }
        }
    }

    /// Creates a new inference variable.
    fn new_infer_var(&mut self) -> InferVar {
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

    /// Checks whether `index` has been assigned to a value yet.
    /// If so, returns it.
    fn probe(&mut self, index: InferVar) -> Option<Value> {
        let (_root, root_data) = self.find(index);
        root_data.value()
    }

    /// Given two unbound inference variables, unify them for evermore. It is best
    /// **not** to use the variables that result from (e.g.) a `find` operation,
    /// but rather the variables that "arose naturally" when doing inference, because
    /// it helps when issuing blame annotations later.
    fn unify_unbound_vars(&mut self, cause: Cause, index1: InferVar, index2: InferVar) {
        let (root1, root_data1) = self.find(index1);
        let (root2, root_data2) = self.find(index2);
        let rank1 = root_data1
            .rank()
            .unwrap_or_else(|| panic!("index1 ({:?}) was bound", index1));
        let rank2 = root_data2
            .rank()
            .unwrap_or_else(|| panic!("index2 ({:?}) was bound", index2));

        if rank1 < rank2 {
            self.redirect(cause, index2, root2, rank2, index1, root1, rank1);
        } else {
            self.redirect(cause, index1, root1, rank1, index2, root2, rank2);
        }
    }

    /// Binds `unbound_var`, which must not yet be bound to anything, to a value.
    fn bind_unbound_var_to_value(&mut self, cause: Cause, unbound_var: InferVar, value: Value) {
        debug_assert!(self.probe(unbound_var).is_none());
        let (root_unbound_var, _) = self.find(unbound_var);
        self.infers[root_unbound_var] = InferData::Value(value);
        self.trace[root_unbound_var] = Some(UnificationTrace {
            cause,
            other_variable: None,
        });
        self.events.push(unbound_var);
    }

    /// Redirects the (root) variable `root_from` to another root variable (`root_to`).
    /// Adjusts `root_to`'s rank to indicate its new depth.
    fn redirect(
        &mut self,
        cause: Cause,
        index_from: InferVar,
        root_from: InferVar,
        rank_from: Rank,
        index_to: InferVar,
        root_to: InferVar,
        rank_to: Rank,
    ) {
        assert!(self.trace[index_from].is_none());

        self.infers[root_from] = InferData::Redirect(root_to);
        self.trace[root_from] = Some(UnificationTrace {
            cause,
            other_variable: Some(index_to),
        });

        // Before we had two trees with depth `rank_from` and `rank_to`.
        // We are making `rank_from` a child of the other tree, so that has depth `rank_from + 1`.
        // This may or may not change the depth of the new root (depending on what its rank was before).
        let rank_max = std::cmp::max(rank_from.next(), rank_to);
        self.infers[root_to] = InferData::Unbound(rank_max);

        self.events.push(root_from);
    }
}
