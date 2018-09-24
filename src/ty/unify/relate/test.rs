#![cfg(test)]

use crate::ir::DefId;
use crate::ty::debug::{DebugIn, TyDebugContext};
use crate::ty::intern::{Interners, TyInterners};
use crate::ty::unify::UnificationTable;
use crate::ty::Generic;
use crate::ty::InferVar;
use crate::ty::Inferable;
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
    types: FxHashMap<String, DefId>,
    type_names: FxHashMap<DefId, String>,
    type_variables: FxHashMap<String, Ty>,
    placeholders: FxHashMap<String, Ty>,
    placeholder_names: FxHashMap<Placeholder, String>,
}

impl TyDebugContext for TestContext {
    fn write_infer_var(
        &self,
        var: InferVar,
        context: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.unify.write_infer_var(var, context, fmt)
    }

    fn write_placeholder(
        &self,
        placeholder: Placeholder,
        _context: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(fmt, "{}", self.placeholder_names[&placeholder])
    }

    fn write_type_name(
        &self,
        def_id: DefId,
        _context: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(fmt, "{}", self.type_names[&def_id])
    }
}

impl Interners for TestContext {
    fn interners(&self) -> &TyInterners {
        self.unify.interners()
    }
}

impl TestContext {
    fn share(&mut self) -> Perm {
        self.intern(Inferable::Known(PermData::Shared(self.region)))
    }

    fn borrow(&mut self) -> Perm {
        self.intern(Inferable::Known(PermData::Borrow(self.region)))
    }

    fn own(&mut self) -> Perm {
        self.common().own
    }

    fn def_id(&mut self, name: &str) -> DefId {
        let TestContext {
            types, type_names, ..
        } = self;

        let next = types.len();
        *types.entry(name.to_string()).or_insert_with(|| {
            type_names.insert(next, name.to_string());
            next
        })
    }

    fn type_variable(&mut self, name: &str) -> Ty {
        let TestContext {
            type_variables,
            unify,
            ..
        } = self;

        *type_variables
            .entry(name.to_string())
            .or_insert_with(|| Ty {
                perm: unify.new_inferable(),
                base: unify.new_inferable(),
            })
    }

    fn placeholder(&mut self, name: &str) -> Ty {
        let TestContext {
            intern,
            placeholders,
            placeholder_names,
            ..
        } = self;

        let next_index = placeholders.len();
        *placeholders.entry(name.to_string()).or_insert_with(|| {
            let placeholder = Placeholder {
                universe: UniverseIndex::ROOT,
                index: ParameterIndex::new(next_index),
            };

            placeholder_names.insert(placeholder, name.to_string());

            Ty {
                perm: intern.common().own,
                base: intern.intern(Inferable::Known(BaseData {
                    kind: BaseKind::Placeholder(placeholder),
                    generics: intern.common().empty_generics,
                })),
            }
        })
    }

    fn base(&mut self, name: &str, tys: Vec<Ty>) -> Base {
        let generics = self.intern_generics(tys.into_iter().map(Generic::Ty));
        let name = self.def_id(name);
        let kind = BaseKind::Named(name);
        self.intern(Inferable::Known(BaseData { kind, generics }))
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

    (@cx[$cx:expr], @ty[!$name:ident]) => {
        $cx.placeholder(stringify!($name))
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
    let intern = TyInterners::new();
    let mut unify = UnificationTable::new(&intern);
    let region = unify.next_region();
    let mut cx = TestContext {
        intern,
        unify,
        region,
        types: FxHashMap::default(),
        type_names: FxHashMap::default(),
        type_variables: FxHashMap::default(),
        placeholders: FxHashMap::default(),
        placeholder_names: FxHashMap::default(),
    };
    op(&mut cx);
}

#[test]
fn vec_bar_not_repr_eq_vec_baz() {
    setup(|cx| {
        let a = ir!(cx, ty[share Vec<[own Bar]>]);
        let x = ir!(cx, ty[share Vec<[own Baz]>]);
        assert!(cx.unify.ty_repr_eq(a, x).is_err());
    });
}

#[test]
fn share_vec_own_bar_repr_eq_share_vec_own_bar() {
    setup(|cx| {
        let a = ir!(cx, ty[share Vec<[own Bar]>]);
        let b = ir!(cx, ty[share Vec<[own Bar]>]);
        assert!(cx.unify.ty_repr_eq(a, b).is_ok());
    });
}

/// We are only testing base-eq: here we see that
/// permissions don't matter much.
#[test]
fn share_vec_own_bar_repr_eq_own_vec_share_bar() {
    setup(|cx| {
        let a = ir!(cx, ty[share Vec<[own Bar]>]);
        let b = ir!(cx, ty[own Vec<[share Bar]>]);
        assert!(cx.unify.ty_repr_eq(a, b).is_ok());
    });
}

/// Even `borrow` and `share` are base-eq, despite
/// having different representation.
#[test]
fn share_vec_borrow_bar_repr_eq_borrow_vec_share_bar() {
    setup(|cx| {
        let a = ir!(cx, ty[share Vec<[?X]>]);
        let b = ir!(cx, ty[borrow Vec<[share Bar]>]);
        assert!(cx.unify.ty_repr_eq(a, b).is_err());

        // Even though got an error, we still inferred
        // that `?X` must be `Bar`:
        assert_eq!(
            format!("{:?}", a.debug_in(cx)),
            format!("shared(Region(0)) Vec<?(0) Bar>")
        );
    });
}

#[test]
fn instantiate_spine_repr() {
    setup(|cx| {
        let a = ir!(cx, ty[?X]);
        let b = ir!(cx, ty[share Vec<[own Bar]>]);
        assert!(cx.unify.ty_repr_eq(a, b).is_ok());
        assert_eq!(
            format!("{:?}", a.debug_in(cx)),
            format!("?(0) Vec<?(2) Bar>")
        );
        let c = ir!(cx, ty[own Vec<[own Bar]>]);
        assert!(cx.unify.ty_repr_eq(a, c).is_ok());
        let d = ir!(cx, ty[own Vec<[own Baz]>]);
        assert!(cx.unify.ty_repr_eq(a, d).is_err());
    });
}
