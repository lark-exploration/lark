use intern::Untern;
use lark_string::global::GlobalIdentifier;
use lark_string::global::GlobalIdentifierTables;
use lark_string::text::Text;

pub trait LookupStringId: Sized + AsRef<GlobalIdentifierTables> {
    fn lookup(&self, id: GlobalIdentifier) -> Text {
        id.untern(self)
    }
}

impl<T> LookupStringId for T where T: AsRef<GlobalIdentifierTables> {}
