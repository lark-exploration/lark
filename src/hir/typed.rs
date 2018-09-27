use crate::hir;
use crate::ty;
use rustc_hash::FxHashMap;

/// Extra data about the HIR that results from typeck.
crate struct Typed {
    /// - Result type of an expression
    /// - Type assigned to a variable
    /// - ...
    types: FxHashMap<hir::MetaIndex, ty::Ty>,

    /// "Place" expressions have an associated permission
    /// that is often inferred; this maps from the expression
    /// to the permission.
    perms: FxHashMap<hir::Expression, ty::Perm>,
}

impl Typed {
    crate fn insert_ty(&mut self, index: impl Into<hir::MetaIndex>, ty: ty::Ty) {
        let meta_index: hir::MetaIndex = index.into();
        if let Some(old_value) = self.types.insert(meta_index, ty) {
            panic!(
                "already had a type for `{:?}`: `{:?}`",
                meta_index, old_value
            );
        }
    }

    crate fn ty(&self, index: impl Into<hir::MetaIndex>) -> ty::Ty {
        let meta_index: hir::MetaIndex = index.into();
        self.types[&meta_index]
    }
}
