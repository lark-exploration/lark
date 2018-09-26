use crate::hir;
use crate::ty;
use rustc_hash::FxHashMap;

/// Extra data about the HIR that results from typeck.
crate struct Typed {
    crate types: FxHashMap<hir::MetaIndex, ty::Ty>,
}

impl Typed {
    crate fn ty(&self, index: impl Into<hir::MetaIndex>) -> ty::Ty {
        let meta_index: hir::MetaIndex = index.into();
        self.types[&meta_index]
    }
}
