#![cfg(ignore_me)]

use codespan_reporting::Diagnostic;
use crate::hir;
use crate::map::FxIndexMap;
use crate::parser::Span;
use crate::ty;
use crate::unify::InferVar;
use crate::unify::UnificationTable;
use generational_arena::Arena;
use std::rc::Rc;

mod infer;
mod ops;

struct BaseTypeChecker {
    hir: Rc<hir::Hir>,
    ops_arena: Arena<Box<dyn ops::BoxedTypeCheckerOp>>,
    ops_blocked: FxIndexMap<InferVar, Vec<ops::OpIndex>>,
    errors: Vec<Diagnostic>,
    unify: UnificationTable<TyInterners, Cause>,
}

struct BaseTy;

impl ty::TypeFamily for BaseTy {
    type Perm = ty::Erased;
}

struct Cause {
    span: Span,
}

impl Interners for TypeChecker {
    fn interners(&self) -> &TyInterners {
        &self.interners
    }
}
