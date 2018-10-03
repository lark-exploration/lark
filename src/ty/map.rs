use crate::ty::intern::Interners;
use crate::ty::Base;
use crate::ty::Generic;
use crate::ty::Perm;
use crate::ty::Ty;

crate trait Map {
    type Output;

    fn map_with(&self, mapper: &mut impl Mapper) -> Self::Output;
}

crate trait Mapper: Interners {
    fn map_perm(&mut self, perm: Perm) -> Perm;

    fn map_base(&mut self, base: Base) -> Base;
}

impl<T> Map for Option<T>
where
    T: Map,
{
    type Output = Option<T::Output>;

    fn map_with(&self, mapper: &mut impl Mapper) -> Self::Output {
        match self {
            Some(v) => Some(v.map_with(mapper)),
            None => None,
        }
    }
}

impl<T> Map for Vec<T>
where
    T: Map,
{
    type Output = Vec<T::Output>;

    fn map_with(&self, mapper: &mut impl Mapper) -> Self::Output {
        self.iter().map(|t| t.map_with(mapper)).collect()
    }
}

impl Map for Base {
    type Output = Self;

    fn map_with(&self, mapper: &mut impl Mapper) -> Self::Output {
        mapper.map_base(*self)
    }
}

impl Map for Perm {
    type Output = Self;

    fn map_with(&self, mapper: &mut impl Mapper) -> Self::Output {
        mapper.map_perm(*self)
    }
}

impl Map for Ty {
    type Output = Self;

    fn map_with(&self, mapper: &mut impl Mapper) -> Self::Output {
        let Ty { perm, base } = self;
        let perm = perm.map_with(mapper);
        let base = base.map_with(mapper);
        Ty { perm, base }
    }
}

impl Map for Generic {
    type Output = Self;
    fn map_with(&self, mapper: &mut impl Mapper) -> Self::Output {
        match self {
            Generic::Ty(ty) => Generic::Ty(ty.map_with(mapper)),
        }
    }
}
