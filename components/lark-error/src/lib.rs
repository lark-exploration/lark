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
//!     for bridging a `Result<T, ErrorReported>` into a `WithError<U>` where `U` has
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
use parser::ParseError;

/// Unit type used in `Result` to indicate a value derived from other
/// value where an error was already reported.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ErrorReported;

impl From<ParseError> for ErrorReported {
    fn from(_: ParseError) -> ErrorReported {
        ErrorReported
    }
}

pub trait ErrorSentinel<Cx> {
    fn error_sentinel(cx: Cx) -> Self;
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
    pub errors: Vec<Span>,
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
    pub fn report_error<Cx>(cx: Cx, span: Span) -> WithError<T>
    where
        T: ErrorSentinel<Cx>,
    {
        WithError {
            value: T::error_sentinel(cx),
            errors: vec![span],
        }
    }

    /// Convenience function: generates a `WithError` that uses a
    /// sentinel value to indicate that an error has already been
    /// reported.
    pub fn error_sentinel<Cx>(cx: Cx) -> WithError<T>
    where
        T: ErrorSentinel<Cx>,
    {
        WithError::ok(T::error_sentinel(cx))
    }

    /// Append any errors into `vec` and return our wrapped value.
    pub fn accumulate_errors_into(self, vec: &mut Vec<Span>) -> T {
        vec.extend(self.errors);
        self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn into_result(self) -> Result<T, ErrorReported> {
        if !self.errors.is_empty() {
            Err(ErrorReported)
        } else {
            Ok(self.value)
        }
    }
}

impl<T, DB> ErrorSentinel<&DB> for Result<T, ErrorReported> {
    fn error_sentinel(_db: &DB) -> Self {
        Err(ErrorReported)
    }
}

/// A kind of `?` operator for `Result<T, ErrorReported>` values -- if
/// `$v` is an `Err`, then returns `WithError::error_sentinel($cx)`
/// from the surrounding function.
pub macro or_sentinel($cx:expr, $v:expr) {
    match $v {
        Ok(v) => v,
        Err(_) => return WithError::error_sentinel($cx),
    }
}
