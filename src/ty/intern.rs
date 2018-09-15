use crate::arena::Arenas;
use crate::ty::{Generic, Generics, Mode, ModeKind, Ty, TyData, TyKey, TyKind};
use crate::ty::query::TyQueries;
use rustc_hash::FxHashMap;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};

crate struct TyArenas<'arena> {
    type_data_arena: typed_arena::Arena<TyData<'arena>>,
    generic_arena: typed_arena::Arena<Generic<'arena>>,
    mode_kind_arena: typed_arena::Arena<ModeKind<'arena>>,
}

/// The "type context" is a global resource that interns types and
/// other type-related things. Types are allocates in the various
/// global arenas so that they can be freely copied around.
crate struct TyInterners<'global> {
    arenas: &'global TyArenas<'global>,
    intern_ty: RefCell<FxHashMap<&'global TyKey<'global>, Ty<'global>>>,
    intern_generics: RefCell<FxHashMap<Generics<'global>, Generics<'global>>>,
    intern_modes: RefCell<FxHashMap<&'global ModeKind<'global>, Mode<'global>>>,
}

impl<'global> TyInterners<'global> {
    crate fn new(arenas: &'global TyArenas<'global>) -> Self {
        TyInterners {
            arenas,
            intern_ty: RefCell::new(FxHashMap::default()),
            intern_generics: RefCell::new(FxHashMap::default()),
            intern_modes: RefCell::new(FxHashMap::default()),
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

    crate fn ty(&self, kind: TyKind<'global>, generics: Generics<'global>) -> Ty<'global> {
        let key = TyKey { kind, generics };
        Self::intern(&self.intern_ty, &key, || {
            let hash = {
                let mut hasher = rustc_hash::FxHasher::default();
                kind.hash(&mut hasher);
                hasher.finish()
            };
            let data = self.arenas.type_data_arena.alloc(TyData { hash, key });
            let ty = Ty { data };
            (&data.key, ty)
        })
    }

    /// Given some type T and a mode m, constructs the type `m T`.
    crate fn ty_in_mode(&self, ty: Ty<'global>, mode: Mode<'global>) -> Ty<'global> {
        let generics = self.generics(&[ty.into()]);
        self.ty(TyKind::Mode { mode }, generics)
    }

    crate fn generics(&self, generics: &[Generic<'global>]) -> Generics<'global> {
        Self::intern(&self.intern_generics, generics, || {
            let p = self
                .arenas
                .generic_arena
                .alloc_extend(generics.iter().cloned());
            (p, p)
        })
    }

    crate fn mode(&self, mode_kind: ModeKind<'global>) -> Mode<'global> {
        Self::intern(&self.intern_modes, &mode_kind, || {
            let p = Mode {
                kind: self.arenas.mode_kind_arena.alloc(mode_kind),
            };
            (p.kind(), p)
        })
    }
}
