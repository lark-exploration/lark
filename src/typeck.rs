use codespan_reporting::Diagnostic;
use crate::hir;
use crate::hir::typed::Typed;
use crate::parser::Span;
use crate::ty;
use crate::ty::intern::{Interners, TyInterners};
use crate::ty::InferVar;
use crate::unify::UnificationTable;
use generational_arena::Arena;
use rustc_hash::FxHashMap;
use std::rc::Rc;

mod infer;
mod ops;

struct TypeChecker {
    hir: Rc<hir::Hir>,
    typed: Typed,
    interners: TyInterners,
    ops_arena: Arena<Box<dyn ops::BoxedTypeCheckerOp>>,
    ops_blocked: FxHashMap<InferVar, Vec<ops::OpIndex>>,
    errors: Vec<Diagnostic>,
    unify: UnificationTable<TyInterners, Cause>,
}

struct Cause {
    span: Span,
}

impl Interners for TypeChecker {
    fn interners(&self) -> &TyInterners {
        &self.interners
    }
}
