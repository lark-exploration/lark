//! Debugging utilities
//!
//! Implement `DebugWith<Cx>` for your type. Then, when using
//! `debug!` or whatever, do `debug!("{}", foo.debug_with(cx))`.

#![feature(box_patterns)]
#![feature(never_type)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]

/// A `Debug` trait that carries a context. Most types in Lark
/// implement it, and you can use `derive(DebugWith)` to get
/// Debug-like behavior (from the lark-debug-derive crate).
///
/// To use it, do something like `format!("{}",
/// value.debug_with(cx))`.
pub trait DebugWith {
    fn debug_with<Cx: ?Sized>(&'me self, cx: &'me Cx) -> DebugCxPair<'me, Self, Cx> {
        DebugCxPair { value: self, cx }
    }

    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

/// Useful trait for writing `DebugWith` implementations that are
/// specialized to different contexts. Just derive `Debug` and use the
/// macro `debug_specialized_impl`; then you can implement
/// `FmtWithSpecialized<Cx>` for various specialized contexts as you
/// choose.
pub trait FmtWithSpecialized<Cx: ?Sized> {
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl<Cx: ?Sized, T: ?Sized> FmtWithSpecialized<Cx> for T
where
    T: std::fmt::Debug,
{
    default fn fmt_with_specialized(
        &self,
        _cx: &Cx,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        <T as std::fmt::Debug>::fmt(self, fmt)
    }
}

pub struct DebugCxPair<'me, Value: ?Sized, Cx: ?Sized>
where
    Value: DebugWith,
{
    value: &'me Value,
    cx: &'me Cx,
}

impl<Value: ?Sized, Cx: ?Sized> std::fmt::Debug for DebugCxPair<'me, Value, Cx>
where
    Value: DebugWith,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt_with(self.cx, fmt)
    }
}

impl<Value: ?Sized, Cx: ?Sized> std::fmt::Display for DebugCxPair<'me, Value, Cx>
where
    Value: DebugWith,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt_with(self.cx, fmt)
    }
}

impl<T> DebugWith for Vec<T>
where
    T: DebugWith,
{
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_list()
            .entries(self.iter().map(|elem| elem.debug_with(cx)))
            .finish()
    }
}

impl<T> DebugWith for &T
where
    T: DebugWith,
{
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt_with(self, cx, fmt)
    }
}

impl<T> DebugWith for Option<T>
where
    T: DebugWith,
{
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            None => fmt.debug_struct("None").finish(),
            Some(v) => v.fmt_with(cx, fmt),
        }
    }
}

impl<I, T> DebugWith for indices::IndexVec<I, T>
where
    I: indices::U32Index,
    T: DebugWith,
{
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_list()
            .entries(self.iter().map(|elem| elem.debug_with(cx)))
            .finish()
    }
}

impl<T> DebugWith for std::sync::Arc<T>
where
    T: DebugWith,
{
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt_with(self, cx, fmt)
    }
}

impl<T> DebugWith for Box<T>
where
    T: DebugWith,
{
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt_with(self, cx, fmt)
    }
}

impl<T> DebugWith for std::rc::Rc<T>
where
    T: DebugWith,
{
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt_with(self, cx, fmt)
    }
}

impl DebugWith for ! {
    fn fmt_with<Cx: ?Sized>(
        &self,
        _cx: &Cx,
        _fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        unreachable!()
    }
}

/// Generates a `DebugWith` impl that accepts any `Cx` and uses the
/// built-in `Debug` trait. You can specialize this to particular
/// contexts by implementing `FmtWithSpecialized<Cx>` to yield a
/// specialized impl.
#[macro_export]
macro_rules! debug_fallback_impl {
    ($(for[$($param:tt)*] $t:ty),* $(,)*) => {
        $(
            impl <$($param)*> $crate::DebugWith for $t {
                default fn fmt_with<Cx: ?Sized>(
                    &self,
                    cx: &Cx,
                    fmt: &mut std::fmt::Formatter<'_>,
                ) -> std::fmt::Result {
                    <$t as $crate::FmtWithSpecialized<Cx>>::fmt_with_specialized(self, cx, fmt)
                }
            }
        )*
    };
    ($($t:ty),* $(,)*) => {
        $crate::debug_fallback_impl!($(for[] $t),*);
    };
}

debug_fallback_impl! {
    i8,
    i16,
    i32,
    i64,
    isize,
    u8,
    u16,
    u32,
    u64,
    usize,
    char,
    bool,
    String,
    str,
}
