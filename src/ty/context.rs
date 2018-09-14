use crate::ty::intern::TyInterners;
use crate::ty::query::TyQueries;
use crate::ty::Mode;

crate struct TyContextData<'global> {
    crate intern: &'global TyInterners<'global>,
    crate common: &'global Common<'global>,
    crate engine: &'global dyn TyQueries<'global>,
}

crate struct Common<'global> {
    crate own_mode: Mode<'global>,
}

