//! Types for tracking errors through queries.
//!
//! Here is how it works:
//!
//! - Queries that can themselves report errors return `WithError<T>`;
//!   the errors contained in that value were discovered during executing
//!   that query.
//! - If another query uses the result of a query that returns
//!   `WithError`, it can just use `into_value` to ignore the errors
//!   -- `WithError` always includes some form of sentinel value that
//!   you can use (i.e., you can just ignore the errors and try to get
//!   on with life).
//! - In the worst case, one can you `Result<T, ErrorReported>` and
//!   have `Err(ErrorReported)` act as a sentintel value for "failed
//!   to produce a value because of some error". This is not preferred
//!   because now downstream queries have to care whether you
//!   propagated an error or not, but sometimes it's the best/easiest
//!   thing to do.
//!   - To help with this, the `or_sentinel!` query acts as a kind of `?` operator
//!     for bridging a `Result<T, ErrorReported>` into a `U` where `U` has
//!     a proper sentinel -- if the result is `Err(ErrorReported)`, it creates the
//!     error-sentinel for `U` and returns it.
//!   - This relies on the `ErrorSentinel` trait, which defines the
//!     error-sentinel for a given type.
//!
//! This scheme is not the most ergonomic and I would like to change it,
//! but it will do for now. -nikomatsakis

#![feature(decl_macro)]

use lark_debug_derive::DebugWith;
use parser::pos::Span;
use std::sync::Arc;

/// Unit type used in `Result` to indicate a value derived from other
/// value where an error was already reported.
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ErrorReported(pub Vec<Diagnostic>);

impl ErrorReported {
    pub fn at_diagnostic(s: Diagnostic) -> Self {
        ErrorReported(vec![s])
    }

    pub fn at_diagnostics(s: Vec<Diagnostic>) -> Self {
        ErrorReported(s)
    }

    pub fn some_diagnostic(&self) -> Diagnostic {
        // Pick the first error arbitrarily
        self.diagnostics()[0].clone()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.0
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.0
    }
}

/// A span with an associated label.
/// TODO: We may want to merge this with what's available in error
/// reporting
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Diagnostic {
    pub span: Span,
    pub label: String,
}
impl Diagnostic {
    pub fn new(label: String, span: Span) -> Diagnostic {
        Diagnostic { label, span }
    }
}

/// Used to indicate an operation that may report an error.  Note that
/// there is a subtle -- but important! -- difference between
/// `ErrorReported` and this type -- returning `Err(ErrorReported)`
/// indicates that the query invoked some *other* operation which
/// failed. Returning `WithError<X>` (where `error` is Some) indicates
/// that the operation itself is reporting the error. Confusing the
/// two will result in too many or too few error reports being shown
/// to the user.
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WithError<T> {
    pub value: T,
    pub errors: Vec<Diagnostic>,
}

impl<T> WithError<T> {
    /// Convenience function: generates a `WithError` with a result
    /// that has no error at all.
    pub fn ok(value: T) -> WithError<T> {
        WithError {
            value,
            errors: vec![],
        }
    }

    /// Convenience function: generates a `WithError` indicating that
    /// this query found an error that was not yet reported. The value
    /// is the error-sentinel for this type.
    pub fn report_error<Cx>(cx: Cx, label: String, span: Span) -> WithError<T>
    where
        T: ErrorSentinel<Cx>,
    {
        let diagnostic = Diagnostic::new(label, span);
        WithError {
            value: T::error_sentinel(cx, &[diagnostic.clone()]),
            errors: vec![diagnostic],
        }
    }

    /// Append any errors into `vec` and return our wrapped value.
    pub fn accumulate_errors_into(self, vec: &mut Vec<Diagnostic>) -> T {
        vec.extend(self.errors);
        self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn into_result(self) -> Result<T, ErrorReported> {
        if !self.errors.is_empty() {
            Err(ErrorReported(self.errors.clone()))
        } else {
            Ok(self.value)
        }
    }
}

/// A kind of `?` operator for `Result<T, ErrorReported>` values -- if
/// `$v` is an `Err`, then returns `WithError::error_sentinel($cx)`
/// from the surrounding function.
pub macro or_return_sentinel($cx:expr, $v:expr) {
    match $v {
        Ok(v) => v,
        Err(ErrorReported(spans)) => {
            log::debug!("or_return_sentinel: returning error sentinel");
            return ErrorSentinel::error_sentinel($cx, &spans);
        }
    }
}

pub trait ErrorSentinel<Cx> {
    fn error_sentinel(cx: Cx, error_spans: &[Diagnostic]) -> Self;
}

impl<T, Cx> ErrorSentinel<Cx> for Result<T, ErrorReported> {
    fn error_sentinel(_cx: Cx, spans: &[Diagnostic]) -> Self {
        Err(ErrorReported(spans.to_owned()))
    }
}

impl<T, Cx> ErrorSentinel<Cx> for Arc<T>
where
    T: ErrorSentinel<Cx>,
{
    fn error_sentinel(cx: Cx, spans: &[Diagnostic]) -> Self {
        Arc::new(T::error_sentinel(cx, spans))
    }
}

impl<T, Cx> ErrorSentinel<Cx> for Vec<T>
where
    T: ErrorSentinel<Cx>,
{
    fn error_sentinel(cx: Cx, spans: &[Diagnostic]) -> Self {
        vec![T::error_sentinel(cx, spans)]
    }
}

impl<T, Cx> ErrorSentinel<Cx> for WithError<T>
where
    T: ErrorSentinel<Cx>,
{
    fn error_sentinel(cx: Cx, spans: &[Diagnostic]) -> WithError<T>
    where
        T: ErrorSentinel<Cx>,
    {
        WithError::ok(T::error_sentinel(cx, spans))
    }
}
