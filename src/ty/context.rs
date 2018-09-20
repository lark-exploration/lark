use crate::parser::ast::Mode;
use crate::ty::intern::TyInterners;
use crate::ty::query::TyQueries;
use std::rc::Rc;

#[derive(Clone)]
crate struct TyContextData {
    crate intern: TyInterners,
    crate engine: Rc<dyn TyQueries>,
}
