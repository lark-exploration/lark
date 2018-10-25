use lark_debug_derive::DebugWith;
use parser::pos::Span;
use parser::ParseError;
use ty::declaration::Declaration;
use ty::interners::TyInternTables;
use ty::{Ty, TypeFamily};

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
    value: T,
    errors: Vec<Span>,
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

impl<DB> ErrorSentinel<&DB> for Ty<Declaration>
where
    DB: AsRef<TyInternTables>,
{
    fn error_sentinel(db: &DB) -> Self {
        Declaration::error_ty(db)
    }
}

pub macro or_sentinel($cx:expr, $v:expr) {
    match $v {
        Ok(v) => v,
        Err(_) => return WithError::error_sentinel($cx),
    }
}
