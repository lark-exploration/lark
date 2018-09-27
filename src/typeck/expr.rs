use codespan::ByteSpan;
use codespan_reporting::{Diagnostic, Label};
use crate::hir;
use crate::parser::pos::{Span, Spanned};
use crate::parser::StringId;
use crate::ty;
use crate::ty::intern::{Interners, TyInterners};
use crate::typeck::TypeChecker;
use std::rc::Rc;

impl TypeChecker {
    fn report_error(&mut self, span: Span, message: impl FnOnce(ByteSpan) -> Diagnostic) -> ty::Ty {
        self.errors.push(message(span.to_codespan()));
        self.common().error_ty
    }
}
