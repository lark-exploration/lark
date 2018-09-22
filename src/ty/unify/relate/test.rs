#![cfg(test)]

use crate::ir::DefId;
use crate::ty::intern::Interner;
use crate::ty::intern::TyInterners;
use crate::ty::unify::UnificationTable;
use crate::ty::Generic;
use crate::ty::ParameterIndex;
use crate::ty::Placeholder;
use crate::ty::Region;
use crate::ty::Ty;
use crate::ty::UniverseIndex;
use crate::ty::{Base, BaseData, BaseKind};
use crate::ty::{Perm, PermData};
use rustc_hash::FxHashMap;

struct TestContext {
    intern: TyInterners,
    unify: UnificationTable,
    region: Region,
    type_names: FxHashMap<String, DefId>,
    type_variables: FxHashMap<String, Ty>,
}

impl TestContext {
    fn share(&mut self) -> Perm {
        self.intern.intern(PermData::Shared {
            region: self.region,
        })
    }

    fn borrow(&mut self) -> Perm {
        self.intern.intern(PermData::Borrow {
            region: self.region,
        })
    }

    fn own(&mut self) -> Perm {
        self.intern.common().own
    }

    fn def_id(&mut self, name: &str) -> DefId {
        let next = self.type_names.len();
        *self.type_names.entry(name.to_string()).or_insert(next)
    }

    fn type_variable(&mut self, name: &str) -> Ty {
        let TestContext {
            type_variables,
            intern,
            ..
        } = self;
        let next_index = type_variables.len();
        *type_variables.entry(name.to_string()).or_insert_with(|| {
            let placeholder = Placeholder {
                universe: UniverseIndex::ROOT,
                index: ParameterIndex::new(next_index),
            };

            Ty {
                perm: intern.common().own,
                base: intern.intern(BaseData {
                    kind: BaseKind::Placeholder { placeholder },
                    generics: intern.common().empty_generics,
                }),
            }
        })
    }

    fn base(&mut self, name: &str, tys: Vec<Ty>) -> Base {
        let generics = self
            .intern
            .intern_generics(tys.into_iter().map(Generic::Ty));
        let name = self.def_id(name);
        let kind = BaseKind::Named { name };
        self.intern.intern(BaseData { kind, generics })
    }
}

macro_rules! ir {
    ($cx:expr, ty[$($tokens:tt)*]) => {
        ir! {
            @cx[$cx],
            @ty[$($tokens)*]
        }
    };

    (@cx[$cx:expr], @ty[[$($tokens:tt)*]]) => {
        ir! { @cx[$cx], @ty[$($tokens)*] }
    };

    (@cx[$cx:expr], @ty[?$name:ident]) => {
        $cx.type_variable(stringify!($name))
    };

    (@cx[$cx:expr], @ty[$perm:ident $name:ident]) => {
        Ty {
            perm: ir!(@cx[$cx], @perm[$perm]),
            base: ir!(@cx[$cx], @base[$name[]]),
        }
    };

    (@cx[$cx:expr], @ty[$perm:tt $name:tt < $($arg:tt),* >]) => {
        Ty {
            perm: ir!(@cx[$cx], @perm[$perm]),
            base: ir!(@cx[$cx], @base[$name[$($arg)*]]),
        }
    };

    (@cx[$cx:expr], @perm[$name:ident]) => {
        $cx.$name()
    };

    (@cx[$cx:expr], @base[$name:ident [ $($arg:tt)* ]]) => {
        {
            let tys = vec![
                $(ir!(@cx[$cx], @ty[$arg])),*
            ];
            $cx.base(stringify!($name), tys)
        }
    };
}

fn setup(op: impl FnOnce(&mut TestContext)) {
    let mut intern = TyInterners::new();
    let mut unify = UnificationTable::new(&intern);
    let region = Region::new(0);
    let mut cx = TestContext {
        intern,
        unify,
        region,
        type_names: FxHashMap::default(),
        type_variables: FxHashMap::default(),
    };
    op(&mut cx);
}

#[test]
fn vec_bar_not_base_eq_vec_baz() {
    setup(|cx| {
        let a = ir!(cx, ty[share Vec<[own Bar]>]);
        let x = ir!(cx, ty[share Vec<[own Baz]>]);
        assert!(cx.unify.ty_base_eq(a, x).is_err());
    });
}

#[test]
fn share_vec_own_bar_base_eq_share_vec_own_bar() {
    setup(|cx| {
        let a = ir!(cx, ty[share Vec<[own Bar]>]);
        let b = ir!(cx, ty[share Vec<[own Bar]>]);
        assert!(cx.unify.ty_base_eq(a, b).is_ok());
    });
}

/// We are only testing base-eq: here we see that
/// permissions don't matter much.
#[test]
fn share_vec_own_bar_base_eq_own_vec_share_bar() {
    setup(|cx| {
        let a = ir!(cx, ty[share Vec<[own Bar]>]);
        let b = ir!(cx, ty[own Vec<[share Bar]>]);
        assert!(cx.unify.ty_base_eq(a, b).is_ok());
    });
}

/// Even `borrow` and `share` are base-eq, despite
/// having different representation.
#[test]
fn share_vec_borrow_bar_base_eq_borrow_vec_share_bar() {
    setup(|cx| {
        let a = ir!(cx, ty[share Vec<[borrow Bar]>]);
        let b = ir!(cx, ty[borrow Vec<[share Bar]>]);
        assert!(cx.unify.ty_base_eq(a, b).is_ok());
    });
}
