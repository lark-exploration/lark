//! Debugging utilities
//!
//! Implement `DebugWith<Cx>` for your type. Then, when using
//! `debug!` or whatever, do `debug!("{}", foo.debug_with(cx))`.

#![feature(never_type)]
#![feature(in_band_lifetimes)]

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

impl<Cx: ?Sized> DebugWith<Cx> for ! {
    fn fmt_with(&self, _cx: &Cx, _fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unreachable!()
    }
}
