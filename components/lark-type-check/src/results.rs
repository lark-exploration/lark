use lark_entity::Entity;
use lark_hir as hir;
use lark_ty::map_family::{FamilyMapper, Map};
use lark_ty::Generics;
use lark_ty::Ty;
use lark_ty::TypeFamily;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeCheckResults<F: TypeFamily> {
    /// The "maximum type" computed for expressions,
    /// identified-expressions, and other things that have a type. The
    /// maximum type is the full set of permissions available from
    /// that expression.
    pub max_types: std::collections::BTreeMap<hir::MetaIndex, Ty<F>>,

    /// The "access type" for a given expression -- this is the set of
    /// permissions required by that particular expression. These
    /// cannot exceed the "max types" and are determined by how the
    /// result of the expression is used.
    pub access_types: std::collections::BTreeMap<hir::Expression, Ty<F>>,

    /// The "permission variable" recorded for a given expression
    /// instance.
    pub access_permissions: std::collections::BTreeMap<hir::Expression, F::Perm>,

    /// For references to entities, the generics applied.
    pub generics: std::collections::BTreeMap<hir::MetaIndex, Generics<F>>,

    /// For "type-relative" identifiers, stores the entity that we resolved
    /// to. Examples:
    ///
    /// - `foo.bar` -- attached to the identifier `bar`, entity of the field
    /// - `foo.bar(..)` -- attached to the identifier `bar`, entity of the method
    /// - `Foo { a: b }` -- attached to the identifier `a`, entity of the field
    /// - `foo` -- when an identifier refers to an entity
    pub entities: std::collections::BTreeMap<hir::MetaIndex, Entity>,
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

    crate fn record_max_ty(&mut self, index: impl Into<hir::MetaIndex>, max_ty: Ty<F>) {
        let index = index.into();
        let old_ty = self.max_types.insert(index, max_ty);
        assert!(old_ty.is_none(), "index {:?} already had a max type", index);
    }

    crate fn record_access_ty(&mut self, index: hir::Expression, access_ty: Ty<F>) {
        let old = self.access_types.insert(index, access_ty);
        assert!(
            old.is_none(),
            "index {:?} already had an access type",
            index
        );
    }

    crate fn record_access_permission(&mut self, index: hir::Expression, perm: F::Perm) {
        let index = index.into();
        let old = self.access_permissions.insert(index, perm);
        assert!(
            old.is_none(),
            "index {:?} already had an access permission",
            index
        );
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
    /// index of an expression. Indicates the "maximum type".
    pub fn ty(&self, index: impl Into<hir::MetaIndex>) -> Ty<F> {
        self.max_types[&index.into()]
    }

    /// Returns the "access type" of this expression, which indicates
    /// how many permissions were *needed*.
    pub fn access_ty(&self, expression: hir::Expression) -> Ty<F> {
        self.access_types[&expression]
    }

    /// Load the type for `index`, if any is stored, else return `None`.
    pub fn opt_ty(&self, index: impl Into<hir::MetaIndex>) -> Option<Ty<F>> {
        self.max_types.get(&index.into()).cloned()
    }

    /// Check whether there is a type recorded for `index`.
    pub fn has_recorded_ty(&self, index: impl Into<hir::MetaIndex>) -> bool {
        self.max_types.contains_key(&index.into())
    }
}

impl<F: TypeFamily> Default for TypeCheckResults<F> {
    fn default() -> Self {
        Self {
            max_types: Default::default(),
            access_types: Default::default(),
            access_permissions: Default::default(),
            generics: Default::default(),
            entities: Default::default(),
        }
    }
}

impl<S, T> Map<S, T> for TypeCheckResults<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = TypeCheckResults<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        let TypeCheckResults {
            max_types,
            access_types,
            access_permissions,
            generics,
            entities,
        } = self;
        TypeCheckResults {
            max_types: max_types.map(mapper),
            access_types: access_types.map(mapper),
            generics: generics.map(mapper),
            entities: entities.map(mapper),
            access_permissions: access_permissions
                .iter()
                .map(|(&key, &value)| (key, mapper.map_perm(value)))
                .collect(),
        }
    }
}
