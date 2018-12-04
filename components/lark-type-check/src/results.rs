use lark_entity::Entity;
use lark_hir as hir;
use lark_ty::map_family::{FamilyMapper, Map};
use lark_ty::Generics;
use lark_ty::Ty;
use lark_ty::TypeFamily;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeCheckResults<F: TypeFamily> {
    /// The type computed for expressions, identified-expressions, and
    /// other things that have a type.
    crate types: std::collections::BTreeMap<hir::MetaIndex, Ty<F>>,

    /// For references to entities, the generics applied.
    crate generics: std::collections::BTreeMap<hir::MetaIndex, Generics<F>>,

    /// For "type-relative" identifiers, stores the entity that we resolved
    /// to. Examples:
    ///
    /// - `foo.bar` -- attached to the identifier `bar`, entity of the field
    /// - `foo.bar(..)` -- attached to the identifier `bar`, entity of the method
    /// - `Foo { a: b }` -- attached to the identifier `a`, entity of the field
    /// - `foo` -- when an identifier refers to an entity
    crate entities: std::collections::BTreeMap<hir::MetaIndex, Entity>,

    crate access_permissions: std::collections::BTreeMap<hir::Expression, F::Perm>,
}

impl<F: TypeFamily> TypeCheckResults<F> {
    /// Record the entity assigned with a given element of the HIR
    /// (e.g. the identifier of a field).
    crate fn record_entity(&mut self, index: impl Into<hir::MetaIndex>, entity: Entity) {
        let index = index.into();
        let old_entity = self.entities.insert(index, entity);
        assert!(
            old_entity.is_none(),
            "index {:?} already had an entity",
            index
        );
    }

    /// Record the type assigned with a given element of the HIR
    /// (typically an expression).
    crate fn record_ty(&mut self, index: impl Into<hir::MetaIndex>, ty: Ty<F>) {
        let index = index.into();
        let old_ty = self.types.insert(index, ty);
        assert!(old_ty.is_none(), "index {:?} already had a type", index);
    }

    /// Record the generics for a given element of the HIR
    /// (typically an expression).
    crate fn record_generics(&mut self, index: impl Into<hir::MetaIndex>, g: &Generics<F>) {
        let index = index.into();
        let old_generics = self.generics.insert(index, g.clone());
        assert!(
            old_generics.is_none(),
            "index {:?} already had generics",
            index
        );
    }

    /// Access the type stored for the given `index`, usually the
    /// index of an expression.
    pub fn ty(&self, index: impl Into<hir::MetaIndex>) -> Ty<F> {
        self.types[&index.into()]
    }

    /// Load the type for `index`, if any is stored, else return `None`.
    pub fn opt_ty(&self, index: impl Into<hir::MetaIndex>) -> Option<Ty<F>> {
        self.types.get(&index.into()).cloned()
    }

    /// Check whether there is a type recorded for `index`.
    pub fn has_recorded_ty(&self, index: impl Into<hir::MetaIndex>) -> bool {
        self.types.contains_key(&index.into())
    }
}

impl<F: TypeFamily> Default for TypeCheckResults<F> {
    fn default() -> Self {
        Self {
            types: Default::default(),
            generics: Default::default(),
            entities: Default::default(),
            access_permissions: Default::default(),
        }
    }
}

impl<S, T> Map<S, T> for TypeCheckResults<S>
where
    S: TypeFamily,
    T: TypeFamily,
    S::Perm: Map<S, T, Output = T::Perm>,
{
    type Output = TypeCheckResults<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        let TypeCheckResults {
            types,
            generics,
            entities,
            access_permissions,
        } = self;
        TypeCheckResults {
            types: types.map(mapper),
            generics: generics.map(mapper),
            entities: entities.map(mapper),
            access_permissions: access_permissions.map(mapper),
        }
    }
}
