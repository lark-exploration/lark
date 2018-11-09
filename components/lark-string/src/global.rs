//! Global string interning.

use debug::FmtWithSpecialized;
use intern::Intern;
use intern::Untern;
use std::sync::Arc;

indices::index_type! {
    /// A "global ident" is an identifier that is valid across files
    /// and contexts. These are interned globally and as a result are
    /// intended to be used "sparingly".
    pub struct GlobalIdentifier { .. }
}

debug::debug_fallback_impl!(GlobalIdentifier);

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlobalIdentifierData {
    data: Arc<String>,
}

impl std::ops::Deref for GlobalIdentifierData {
    type Target = str;

    fn deref(&self) -> &str {
        &self.data
    }
}

impl AsRef<str> for GlobalIdentifierData {
    fn as_ref(&self) -> &str {
        &self.data
    }
}

impl std::borrow::Borrow<str> for GlobalIdentifierData {
    fn borrow(&self) -> &str {
        &self.data
    }
}

intern::intern_tables! {
    pub struct GlobalIdentifierTables {
        struct GlobalIdentifierTablesData {
            strings: map(GlobalIdentifier, GlobalIdentifierData),
        }
    }
}

impl Intern<GlobalIdentifierTables> for &str {
    type Key = GlobalIdentifier;

    fn intern(self, interner: &dyn AsRef<GlobalIdentifierTables>) -> Self::Key {
        intern::intern_impl(
            self,
            interner,
            |d| &d[..],
            |d| GlobalIdentifierData {
                data: Arc::new(d.to_string()),
            },
        )
    }
}

impl Intern<GlobalIdentifierTables> for String {
    type Key = GlobalIdentifier;

    fn intern(self, interner: &dyn AsRef<GlobalIdentifierTables>) -> Self::Key {
        intern::intern_impl(
            self,
            interner,
            |d| &d[..],
            |d| GlobalIdentifierData { data: Arc::new(d) },
        )
    }
}

impl<Cx> FmtWithSpecialized<Cx> for GlobalIdentifier
where
    Cx: AsRef<GlobalIdentifierTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.untern(cx);
        write!(fmt, "{:?}", &data[..])
    }
}
