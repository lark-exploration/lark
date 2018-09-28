use crate::query::dyn_descriptor::DynDescriptor;
use crate::query::BaseQueryContext;
use crate::query::Query;
use crate::query::QueryTable;
use crate::typeck::query::TypeckQueryContext;
use rustc_hash::FxHashMap;
use std::any::Any;
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Write;
use std::hash::Hash;

crate struct QueryContextImpl {
    storage: QueryContextImplStorage,
    execution_stack: RefCell<Vec<DynDescriptor>>,
}

#[allow(non_snake_case)]
struct QueryContextImplStorage {
    Fields: <crate::typeck::query::Fields as Query<QueryContextImpl>>::Storage,
    Ty: <crate::typeck::query::Ty as Query<QueryContextImpl>>::Storage,
}

impl BaseQueryContext for QueryContextImpl {
    type QueryDescriptor = DynDescriptor;

    fn execute_query_implementation<Q>(
        &self,
        descriptor: Self::QueryDescriptor,
        key: &Q::Key,
    ) -> Q::Value
    where
        Q: Query<Self>,
    {
        self.execution_stack.borrow_mut().push(descriptor);
        let value = Q::execute(self, key.clone());
        self.execution_stack.borrow_mut().pop();
        value
    }

    fn report_unexpected_cycle(&self, descriptor: Self::QueryDescriptor) -> ! {
        let execution_stack = self.execution_stack.borrow();
        let start_index = (0..execution_stack.len())
            .rev()
            .filter(|&i| execution_stack[i] == descriptor)
            .next()
            .unwrap();

        let mut message = format!("Internal error, cycle detected:\n");
        for descriptor in &execution_stack[start_index..] {
            writeln!(message, "- {:?}\n", descriptor).unwrap();
        }
        panic!(message)
    }
}

impl TypeckQueryContext for QueryContextImpl {
    fn fields(&self) -> QueryTable<'_, Self, crate::typeck::query::Fields> {
        QueryTable::new(
            self,
            &self.storage.Fields,
            DynDescriptor::from_key::<Self, crate::typeck::query::Fields>,
        )
    }

    fn ty(&self) -> QueryTable<'_, Self, crate::typeck::query::Ty> {
        QueryTable::new(
            self,
            &self.storage.Ty,
            DynDescriptor::from_key::<Self, crate::typeck::query::Ty>,
        )
    }
}
