//! Debugging utilities
//!
//! Implement `DebugWith<Cx>` for your type. Then, when using
//! `debug!` or whatever, do `debug!("{}", foo.debug_with(cx))`.

#![feature(box_patterns)]
#![feature(never_type)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]

pub trait DebugWith<Cx: ?Sized> {
    fn debug_with(&'me self, cx: &'me Cx) -> DebugCxPair<'me, Self, Cx> {
        DebugCxPair { value: self, cx }
    }

    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

pub struct DebugCxPair<'me, Value: ?Sized, Cx: ?Sized>
where
    Value: DebugWith<Cx>,
{
    value: &'me Value,
    cx: &'me Cx,
}

impl<Value: ?Sized, Cx: ?Sized> std::fmt::Debug for DebugCxPair<'me, Value, Cx>
where
    Value: DebugWith<Cx>,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt_with(self.cx, fmt)
    }
}

impl<Value: ?Sized, Cx: ?Sized> std::fmt::Display for DebugCxPair<'me, Value, Cx>
where
    Value: DebugWith<Cx>,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt_with(self.cx, fmt)
    }
}

impl<T, Cx: ?Sized> DebugWith<Cx> for Vec<T>
where
    T: DebugWith<Cx>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_list()
            .entries(self.iter().map(|elem| elem.debug_with(cx)))
            .finish()
    }
}

impl<T, Cx: ?Sized> DebugWith<Cx> for &T
where
    T: DebugWith<Cx>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt_with(self, cx, fmt)
    }
}

impl<T, Cx: ?Sized> DebugWith<Cx> for Option<T>
where
    T: DebugWith<Cx>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            None => fmt.debug_struct("None").finish(),
            Some(v) => v.fmt_with(cx, fmt),
        }
    }
}

impl<I, T, Cx: ?Sized> DebugWith<Cx> for indices::IndexVec<I, T>
where
    I: indices::U32Index,
    T: DebugWith<Cx>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_list()
            .entries(self.iter().map(|elem| elem.debug_with(cx)))
            .finish()
    }
}

impl<T, Cx: ?Sized> DebugWith<Cx> for std::sync::Arc<T>
where
    T: DebugWith<Cx>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt_with(self, cx, fmt)
    }
}

impl<T, Cx: ?Sized> DebugWith<Cx> for Box<T>
where
    T: DebugWith<Cx>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt_with(self, cx, fmt)
    }
}

impl<T, Cx: ?Sized> DebugWith<Cx> for std::rc::Rc<T>
where
    T: DebugWith<Cx>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt_with(self, cx, fmt)
    }
}

impl<Cx: ?Sized> DebugWith<Cx> for ! {
    fn fmt_with(&self, _cx: &Cx, _fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unreachable!()
    }
}

/// Generates a `DebugWith` impl that accepts any `Cx` and uses the
/// built-in `Debug` trait.
#[macro_export]
macro_rules! debug_fallback_impl {
    ($($t:ty),* $(,)*) => {
        $(
            impl<Cx: ?Sized> $crate::DebugWith<Cx> for $t {
                default fn fmt_with(
                    &self,
                    _cx: &Cx,
                    fmt: &mut std::fmt::Formatter<'_>,
                ) -> std::fmt::Result {
                    std::fmt::Debug::fmt(self, fmt)
                }
            }
        )*
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
