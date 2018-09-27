use codespan_reporting::Diagnostic;
use crate::hir;
use crate::hir::typed::Typed;
use crate::ty;
use crate::ty::intern::{Interners, TyInterners};
use crate::ty::unify::UnificationTable;
use crate::ty::InferVar;
use generational_arena::Arena;
use rustc_hash::FxHashMap;
use std::rc::Rc;

mod expr;
mod infer;
mod ops;

struct TypeChecker {
    hir: Rc<hir::Hir>,
    typed: Typed,
    interners: TyInterners,
    ops_arena: Arena<Box<dyn ops::BoxedTypeCheckerOp>>,
    ops_blocked: FxHashMap<InferVar, Vec<ops::OpIndex>>,
    unify: UnificationTable,
    errors: Vec<Diagnostic>,
}

impl Interners for TypeChecker {
    fn interners(&self) -> &TyInterners {
        &self.interners
    }
}
