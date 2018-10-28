use crate::prelude::*;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StringId {
    crate position: usize,
}

pub trait LookupStringId {
    fn lookup(&self, id: StringId) -> Arc<String>;
}

debug::debug_fallback_impl!(StringId);

impl<Cx: LookupStringId> FmtWithSpecialized<Cx> for StringId {
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&cx.lookup(*self), fmt)
    }
}

impl fmt::Debug for StringId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.position)
    }
}
