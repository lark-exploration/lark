//! Global string interning.

use crate::text::Text;
use debug::FmtWithSpecialized;
use intern::Intern;
use intern::Untern;

indices::index_type! {
    /// A "global ident" is an identifier that is valid across files
    /// and contexts. These are interned globally and as a result are
    /// intended to be used "sparingly".
    pub struct GlobalIdentifier { .. }
}

debug::debug_fallback_impl!(GlobalIdentifier);

intern::intern_tables! {
    pub struct GlobalIdentifierTables {
        struct GlobalIdentifierTablesData {
            strings: map(GlobalIdentifier, Text),
        }
    }
}

impl Intern<GlobalIdentifierTables> for &str {
    type Key = GlobalIdentifier;

    fn intern(self, interner: &(impl AsRef<GlobalIdentifierTables> + ?Sized)) -> Self::Key {
        intern::intern_impl(self, interner, |d| &d[..], |d| Text::from(d))
    }
}

impl Intern<GlobalIdentifierTables> for String {
    type Key = GlobalIdentifier;

    fn intern(self, interner: &(impl AsRef<GlobalIdentifierTables> + ?Sized)) -> Self::Key {
        intern::intern_impl(self, interner, |d| &d[..], |d| Text::from(d))
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
