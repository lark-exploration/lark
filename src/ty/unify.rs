use crate::ty::intern::{GetFrom, TyInterners};
use crate::ty::Ty;
use crate::ty::{AsInferVar, InferVar};
use crate::ty::{Base, BaseData};
use crate::ty::{Perm, PermData};
use indexed_vec::IndexVec;
use std::convert::TryFrom;

crate struct UnificationTable {
    intern: TyInterners,
    infers: IndexVec<InferVar, InferData>,
    values: IndexVec<Value, ValueData>,
}

enum InferData {
    /// A root variable that is not yet bound to any value.
    Unbound,

    /// A variable that is bound to `Value.
    Value(Value),

    /// A leaf variable that is redirected to another variable
    /// (which may or may not still be the root). This value will
    /// eventually get overwritten with `Value` once the value
    /// is known.
    Redirect(InferVar),
}

index_type! {
    struct Value { .. }
}

#[derive(Copy, Clone, Debug)]
crate enum ValueData {
    Perm(Perm),
    Base(Base),
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

impl UnificationTable {
    fn new(intern: &TyInterners) -> Self {
        Self {
            intern: intern.clone(),
            infers: IndexVec::new(),
            values: IndexVec::new(),
        }
    }

    /// Finds the "root index" associated with `index1`.
    /// In the "union-find" algorithm this is called "find".
    fn find(&mut self, index1: InferVar) -> (InferVar, Option<Value>) {
        match self.infers[index1] {
            InferData::Unbound => (index1, None),
            InferData::Value(value1) => (index1, Some(value1)),
            InferData::Redirect(index2) => {
                let (index3, value3) = self.find(index2);
                if index2 != index3 {
                    // This is the "path compression" step of union-find:InferData
                    // basically, if we were redireced to X, and X was later
                    // redirected to Y, then we should redirect ourselves to Y too.
                    match value3 {
                        Some(v) => self.infers[index1] = InferData::Value(v),
                        None => self.infers[index1] = InferData::Redirect(index3),
                    }
                }
                (index3, value3)
            }
        }
    }

    /// Checks whether `index` has been assigned to a value yet.
    /// If so, returns it.
    fn probe(&mut self, index: InferVar) -> Option<Value> {
        let (_root, value) = self.find(index);
        value
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

    crate fn shallow_resolve<T>(&mut self, value: T) -> T
    where
        T: ShallowResolveable,
    {
        value.shallow_resolve_in(self)
    }

    fn unify_var_var(&mut self, index1: InferVar, index2: InferVar) {}
}

crate trait ShallowResolveable {
    fn shallow_resolve_in(self, unify: &mut UnificationTable) -> Self;
}

crate trait ShallowResolveableVar: Copy + GetFrom + TryFrom<ValueData, Error = String>
where
    Self::Data: AsInferVar,
{
}

impl ShallowResolveable for Ty {
    fn shallow_resolve_in(self, unify: &mut UnificationTable) -> Self {
        let Ty {
            perm,
            base,
            generics,
        } = self;
        let perm = unify.shallow_resolve(perm);
        let base = unify.shallow_resolve(base);
        Ty {
            perm,
            base,
            generics,
        }
    }
}

impl<T> ShallowResolveable for T
where
    T: ShallowResolveableVar,
    T::Data: AsInferVar,
{
    fn shallow_resolve_in(self, unify: &mut UnificationTable) -> T {
        let data = unify.intern.get(self);
        if let Some(var) = data.as_infer_var() {
            if let Some(value) = unify.probe(var) {
                let value_data = unify.values[value];
                return T::try_from(value_data).unwrap();
            }
        }

        self
    }
}

impl ShallowResolveableVar for Perm {}

impl ShallowResolveableVar for Base {}
