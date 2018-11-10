use crate::prelude::*;
use intern::Intern;
use intern::Untern;
use lark_string::global::GlobalIdentifierTables;
use lark_string::text::Text;

#[derive(Clone, Default, new)]
pub struct ModuleTable {
    #[new(default)]
    data: GlobalIdentifierTables,
}

impl AsRef<GlobalIdentifierTables> for ModuleTable {
    fn as_ref(&self) -> &GlobalIdentifierTables {
        &self.data
    }
}

impl ModuleTable {
    pub fn lookup(&self, id: &GlobalIdentifier) -> Text {
        id.untern(self)
    }

    pub fn intern<I>(&self, hashable: I) -> I::Key
    where
        I: Intern<GlobalIdentifierTables>,
    {
        hashable.intern(self)
    }
}

impl std::fmt::Debug for ModuleTable {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("ModuleTable").finish()
    }
}
