use crate::ty::interners::HasTyInternTables;
use crate::ty::{self, TypeFamily};
use std::sync::Arc;

crate trait Map<M: FamilyMapper> {
    type Output;

    fn map(&self, mapper: &mut M) -> Self::Output;
}

crate trait FamilyMapper: HasTyInternTables {
    type Source: TypeFamily;
    type Target: TypeFamily;

    fn map_ty(&mut self, ty: ty::Ty<Self::Source>) -> ty::Ty<Self::Target>;
}

#[allow(type_alias_bounds)]
type SourcePerm<M: FamilyMapper> = <<M as FamilyMapper>::Source as TypeFamily>::Perm;

#[allow(type_alias_bounds)]
type SourceBase<M: FamilyMapper> = <<M as FamilyMapper>::Source as TypeFamily>::Base;

#[allow(type_alias_bounds)]
type TargetPerm<M: FamilyMapper> = <<M as FamilyMapper>::Target as TypeFamily>::Perm;

#[allow(type_alias_bounds)]
type TargetBase<M: FamilyMapper> = <<M as FamilyMapper>::Target as TypeFamily>::Base;

impl<M, V> Map<M> for &V
where
    M: FamilyMapper,
    V: Map<M>,
{
    type Output = V::Output;

    fn map(&self, mapper: &mut M) -> Self::Output {
        <V as Map<M>>::map(self, mapper)
    }
}

impl<M, V> Map<M> for Arc<V>
where
    M: FamilyMapper,
    V: Map<M>,
{
    type Output = Arc<V::Output>;

    fn map(&self, mapper: &mut M) -> Self::Output {
        let this: &V = self;
        Arc::new(this.map(mapper))
    }
}

impl<M, V> Map<M> for Option<V>
where
    M: FamilyMapper,
    V: Map<M>,
{
    type Output = Option<V::Output>;

    fn map(&self, mapper: &mut M) -> Self::Output {
        match self {
            Some(v) => Some(v.map(mapper)),
            None => None,
        }
    }
}

impl<M, V> Map<M> for Vec<V>
where
    M: FamilyMapper,
    V: Map<M>,
{
    type Output = Vec<V::Output>;

    fn map(&self, mapper: &mut M) -> Self::Output {
        self.iter().map(|e| e.map(mapper)).collect()
    }
}

impl<M> Map<M> for ty::Ty<M::Source>
where
    M: FamilyMapper,
{
    type Output = ty::Ty<M::Target>;

    fn map(&self, mapper: &mut M) -> Self::Output {
        mapper.map_ty(*self)
    }
}

impl<M> Map<M> for ty::BaseData<M::Source>
where
    M: FamilyMapper,
{
    type Output = ty::BaseData<M::Target>;

    fn map(&self, mapper: &mut M) -> Self::Output {
        let ty::BaseData { kind, generics } = self;
        ty::BaseData {
            kind: kind.map(mapper),
            generics: generics.map(mapper),
        }
    }
}

impl<M> Map<M> for ty::BaseKind
where
    M: FamilyMapper,
{
    type Output = ty::BaseKind;

    fn map(&self, _mapper: &mut M) -> Self::Output {
        *self
    }
}

impl<M> Map<M> for ty::Generics<M::Source>
where
    M: FamilyMapper,
{
    type Output = ty::Generics<M::Target>;

    fn map(&self, mapper: &mut M) -> Self::Output {
        let ty::Generics { elements } = self;
        ty::Generics {
            elements: elements.map(mapper),
        }
    }
}

impl<M> Map<M> for ty::Generic<M::Source>
where
    M: FamilyMapper,
{
    type Output = ty::Generic<M::Target>;

    fn map(&self, mapper: &mut M) -> Self::Output {
        match self {
            ty::Generic::Ty(ty) => ty::Generic::Ty(ty.map(mapper)),
        }
    }
}
