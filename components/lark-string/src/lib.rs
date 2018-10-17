//! String interning

#![feature(macro_at_most_once_rep)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(specialization)]

use debug::DebugWith;
use intern::Has;
use intern::Intern;
use intern::Untern;
use std::sync::Arc;

indices::index_type! {
    pub struct StringId { .. }
}

debug::debug_fallback_impl!(StringId);

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringData {
    data: Arc<String>,
}

impl std::ops::Deref for StringData {
    type Target = str;

    fn deref(&self) -> &str {
        &self.data
    }
}

impl AsRef<str> for StringData {
    fn as_ref(&self) -> &str {
        &self.data
    }
}

impl std::borrow::Borrow<str> for StringData {
    fn borrow(&self) -> &str {
        &self.data
    }
}

intern::intern_tables! {
    pub struct StringTables {
        struct StringTablesData {
            strings: map(StringId, StringData),
        }
    }
}

impl Intern<StringTables> for &str {
    type Key = StringId;

    fn intern(self, interner: &dyn Has<StringTables>) -> Self::Key {
        intern::intern_impl(
            self,
            interner,
            |d| &d[..],
            |d| StringData {
                data: Arc::new(d.to_string()),
            },
        )
    }
}

impl Intern<StringTables> for String {
    type Key = StringId;

    fn intern(self, interner: &dyn Has<StringTables>) -> Self::Key {
        intern::intern_impl(
            self,
            interner,
            |d| &d[..],
            |d| StringData { data: Arc::new(d) },
        )
    }
}

impl<Cx> DebugWith<Cx> for StringId
where
    Cx: Has<StringTables>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.untern(cx);
        write!(fmt, "{:?}", &data[..])
    }
}
