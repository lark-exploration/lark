use crate::ty::debug::{DebugIn, TyDebugContext};
use crate::ty::intern::{Intern, Interners, TyInterners, Untern};
use crate::ty::Base;
use crate::ty::Perm;
use crate::ty::Region;
use crate::ty::{InferVar, Inferable};
use indexed_vec::IndexVec;
use std::convert::TryFrom;
use std::fmt;

mod relate;
mod union_find;

#[derive(Clone)]
crate struct UnificationTable {
    interners: TyInterners,

    /// Stores the union-find data for each inference variable.
    /// Used for most efficient lookup.
    infers: IndexVec<InferVar, InferData>,

    /// Stores a more naive trace of which variables were unified
    /// with which. Used for error reporting but makes no effort
    /// to form a balanced tree.
    trace: IndexVec<InferVar, Option<InferVar>>,
    values: IndexVec<Value, ValueData>,

    /// If we need to create a fresh region, what number do we give it?
    next_region: Region,
}

#[derive(Copy, Clone)]
enum InferData {
    /// A root variable that is not yet bound to any value.
    Unbound(Rank),

    /// A variable that is bound to `Value.
    Value(Value),

    /// A leaf variable that is redirected to another variable
    /// (which may or may not still be the root). This value will
    /// eventually get overwritten with `Value` once the value
    /// is known.
    Redirect(InferVar),
}

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

index_type! {
    struct Value { .. }
}

#[derive(Copy, Clone, Debug)]
crate enum ValueData {
    Perm(Perm),
    Base(Base),
}

impl From<Perm> for ValueData {
    fn from(perm: Perm) -> Self {
        ValueData::Perm(perm)
    }
}

impl From<Base> for ValueData {
    fn from(base: Base) -> Self {
        ValueData::Base(base)
    }
}

impl TryFrom<ValueData> for Perm {
    type Error = String;

    fn try_from(value: ValueData) -> Result<Perm, String> {
        if let ValueData::Perm(perm) = value {
            Ok(perm)
        } else {
            Err(format!("expected a Perm, found {:?}", value))
        }
    }
}

impl TryFrom<ValueData> for Base {
    type Error = String;

    fn try_from(value: ValueData) -> Result<Base, String> {
        if let ValueData::Base(base) = value {
            Ok(base)
        } else {
            Err(format!("expected a Base, found {:?}", value))
        }
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

impl UnificationTable {
    fn new(interners: &TyInterners) -> Self {
        Self {
            interners: interners.clone(),
            infers: IndexVec::new(),
            trace: IndexVec::new(),
            values: IndexVec::new(),
            next_region: Region::new(0),
        }
    }

    crate fn next_region(&mut self) -> Region {
        let next_next_region = self.next_region + 1;
        std::mem::replace(&mut self.next_region, next_next_region)
    }

    fn value_as_perm(&self, value: Value) -> Perm {
        if let ValueData::Perm(perm) = self.values[value] {
            perm
        } else {
            panic!("value {:?} is not a type", value)
        }
    }

    fn value_as_base(&self, value: Value) -> Base {
        if let ValueData::Base(base) = self.values[value] {
            base
        } else {
            panic!("value {:?} is not a type", value)
        }
    }

    crate fn shallow_resolve_data<K, V>(&mut self, value: K) -> Result<V, InferVar>
    where
        K: Untern<Data = Inferable<V>> + TryFrom<ValueData, Error = String>,
        Inferable<V>: Intern<Key = K>,
    {
        let data = self.untern(value);
        if let Inferable::Infer(var) = data {
            if let Some(value) = self.probe(var) {
                let value_data = self.values[value];
                let key = K::try_from(value_data).unwrap();
                Ok(self.untern(key).assert_known())
            } else {
                Err(var)
            }
        } else {
            Ok(data.assert_known())
        }
    }

    /// Creates a new inferable thing (permission, base, etc).
    crate fn new_inferable<T, K>(&mut self) -> T
    where
        T: Untern<Data = Inferable<K>>,
        Inferable<K>: Intern<Key = T>,
    {
        let var = self.new_infer_var();
        self.intern(Inferable::Infer(var))
    }
}

impl Interners for UnificationTable {
    fn interners(&self) -> &TyInterners {
        &self.interners
    }
}

impl TyDebugContext for UnificationTable {
    fn write_infer_var(
        &self,
        var: InferVar,
        context: &dyn TyDebugContext,
        fmt: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let (root_var, root_data) = self.find_without_path_compression(var);
        match root_data {
            RootData::Rank(_) => write!(fmt, "{:?}", root_var),
            RootData::Value(v) => match self.values[v] {
                ValueData::Perm(p) => write!(fmt, "{:?}", p.debug_in(context)),
                ValueData::Base(p) => write!(fmt, "{:?}", p.debug_in(context)),
            },
        }
    }
}
