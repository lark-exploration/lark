use crate::ty::interners::HasTyInternTables;
use crate::ty::{self, TypeFamily};
use std::sync::Arc;

crate trait Map<S: TypeFamily, T: TypeFamily> {
    type Output;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output;
}

crate trait FamilyMapper<S: TypeFamily, T: TypeFamily>: HasTyInternTables {
    fn map_ty(&mut self, ty: ty::Ty<S>) -> ty::Ty<T>;
}

impl<S, T, V> Map<S, T> for &V
where
    S: TypeFamily,
    T: TypeFamily,
    V: Map<S, T>,
{
    type Output = V::Output;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        <V as Map<S, T>>::map(self, mapper)
    }
}

impl<S, T, V> Map<S, T> for Arc<V>
where
    S: TypeFamily,
    T: TypeFamily,
    V: Map<S, T>,
{
    type Output = Arc<V::Output>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        let this: &V = self;
        Arc::new(this.map(mapper))
    }
}

impl<S, T, V> Map<S, T> for Option<V>
where
    S: TypeFamily,
    T: TypeFamily,
    V: Map<S, T>,
{
    type Output = Option<V::Output>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        match self {
            Some(v) => Some(v.map(mapper)),
            None => None,
        }
    }
}

impl<S, T, V> Map<S, T> for Vec<V>
where
    S: TypeFamily,
    T: TypeFamily,
    V: Map<S, T>,
{
    type Output = Vec<V::Output>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        self.iter().map(|e| e.map(mapper)).collect()
    }
}

impl<S, T> Map<S, T> for ty::Ty<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = ty::Ty<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        mapper.map_ty(*self)
    }
}

impl<S, T> Map<S, T> for ty::BaseData<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = ty::BaseData<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        let ty::BaseData { kind, generics } = self;
        ty::BaseData {
            kind: kind.map(mapper),
            generics: generics.map(mapper),
        }
    }
}

impl<S, T> Map<S, T> for ty::BaseKind
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = ty::BaseKind;

    fn map(&self, _mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        *self
    }
}

impl<S, T> Map<S, T> for ty::Generics<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = ty::Generics<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        let ty::Generics { elements } = self;
        ty::Generics {
            elements: elements.map(mapper),
        }
    }
}

impl<S, T> Map<S, T> for ty::Generic<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = ty::Generic<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        match self {
            ty::Generic::Ty(ty) => ty::Generic::Ty(ty.map(mapper)),
        }
    }
}

impl<S, T> Map<S, T> for ty::Signature<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = ty::Signature<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        let ty::Signature { inputs, output } = self;
        ty::Signature {
            inputs: inputs.map(mapper),
            output: output.map(mapper),
        }
    }
}
