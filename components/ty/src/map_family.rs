use crate::BaseData;
use crate::BaseKind;
use crate::Generic;
use crate::GenericKind;
use crate::Generics;
use crate::Signature;
use crate::Ty;
use crate::TypeFamily;
use lark_entity::Entity;
use std::sync::Arc;

pub trait Map<S: TypeFamily, T: TypeFamily>: Clone {
    type Output;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output;
}

pub trait FamilyMapper<S: TypeFamily, T: TypeFamily> {
    fn map_ty(&mut self, ty: Ty<S>) -> Ty<T>;

    fn map_placeholder(&mut self, placeholder: S::Placeholder) -> T::Placeholder;
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

impl<S, T> Map<S, T> for Ty<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = Ty<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        mapper.map_ty(*self)
    }
}

impl<S, T> Map<S, T> for BaseData<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = BaseData<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        let BaseData { kind, generics } = self;
        BaseData {
            kind: kind.map(mapper),
            generics: generics.map(mapper),
        }
    }
}

impl<S, T> Map<S, T> for Entity
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = Entity;

    fn map(&self, _mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        *self
    }
}

impl<S, T> Map<S, T> for BaseKind<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = BaseKind<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        match self {
            BaseKind::Named(def_id) => BaseKind::Named(def_id.map(mapper)),

            BaseKind::Placeholder(placeholder) => {
                BaseKind::Placeholder(mapper.map_placeholder(*placeholder))
            }

            BaseKind::Error => BaseKind::Error,
        }
    }
}

impl<S, T> Map<S, T> for Generics<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = Generics<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        let Generics { elements } = self;
        Generics {
            elements: elements.map(mapper),
        }
    }
}

impl<S, T> Map<S, T> for Generic<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = Generic<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        match self {
            GenericKind::Ty(ty) => GenericKind::Ty(ty.map(mapper)),
        }
    }
}

impl<S, T> Map<S, T> for Signature<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = Signature<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        let Signature { inputs, output } = self;
        Signature {
            inputs: inputs.map(mapper),
            output: output.map(mapper),
        }
    }
}
