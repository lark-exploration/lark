use crate::arena::Arenas;
use crate::ty::{Kind, Kinds, Ty, TyData, TyKind};
use rustc_hash::FxHashMap;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};

pub struct TyArenas<'arena> {
    type_data_arena: typed_arena::Arena<TyData<'arena>>,
    kind_arena: typed_arena::Arena<Kind<'arena>>,
}

/// The "type context" is a global resource that interns types and
/// other type-related things. Types are allocates in the various
/// global arenas so that they can be freely copied around.
pub struct TyContext<'global> {
    arenas: &'global TyArenas<'global>,
    intern_ty: RefCell<FxHashMap<&'global TyKind<'global>, Ty<'global>>>,
    intern_kinds: RefCell<FxHashMap<Kinds<'global>, Kinds<'global>>>,
}

impl<'global> TyContext<'global> {
    crate fn new(arenas: &'global TyArenas<'global>) -> Self {
        TyContext {
            arenas,
            intern_ty: RefCell::new(FxHashMap::default()),
            intern_kinds: RefCell::new(FxHashMap::default()),
        }
    }

    fn intern<K, V>(
        map: &RefCell<FxHashMap<&'global K, V>>,
        data: &K,
        alloc: impl FnOnce() -> (&'global K, V),
    ) -> V
    where
        K: ?Sized + Eq + Hash,
        V: Copy,
    {
        if let Some(interned_data) = map.borrow().get(data) {
            return *interned_data;
        }

        let (interned_key, interned_value) = alloc();
        map.borrow_mut().insert(interned_key, interned_value);

        interned_value
    }

    crate fn intern_ty(&self, kind: TyKind<'global>) -> Ty<'global> {
        Self::intern(
            &self.intern_ty,
            &kind,
            || {
                let hash = {
                    let mut hasher = rustc_hash::FxHasher::default();
                    kind.hash(&mut hasher);
                    hasher.finish()
                };
                let ty = Ty {
                    data: self.arenas.type_data_arena.alloc(TyData { hash, kind })
                };
                (ty.kind(), ty)
            },
        )
    }

    crate fn intern_kinds(&self, kinds: &[Kind<'global>]) -> Kinds<'global> {
        Self::intern(
            &self.intern_kinds,
            kinds,
            || {
                let p = self.arenas.kind_arena.alloc_extend(kinds.iter().cloned());
                (p, p)
            },
        )
    }
}
