use super::*;
use crate::parser::ModuleTable;

use std::fmt;

pub struct DebuggableVec<'owner, T: DebugModuleTable + 'owner> {
    inner: &'owner Vec<T>,
    table: &'owner ModuleTable,
}

impl<T: DebugModuleTable + 'owner> DebuggableVec<'owner, T> {
    pub fn from(inner: &'owner Vec<T>, table: &'owner ModuleTable) -> DebuggableVec<'owner, T> {
        DebuggableVec { inner, table }
    }
}

impl<T: DebugModuleTable> fmt::Debug for DebuggableVec<'table, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}",
            self.inner
                .iter()
                .map(|f| Debuggable::from(f, self.table))
                .collect::<Vec<_>>()
        )
    }
}

pub struct Debuggable<'owner, T: DebugModuleTable + 'owner> {
    inner: &'owner T,
    table: &'owner ModuleTable,
}

impl<T: DebugModuleTable + 'owner> Debuggable<'owner, T> {
    pub fn from(inner: &'owner T, table: &'owner ModuleTable) -> Debuggable<'owner, T> {
        Debuggable { inner, table }
    }
}

impl<T: DebugModuleTable> fmt::Debug for Debuggable<'table, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.debug(f, &self.table)
    }
}

pub trait DebugModuleTable {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result;
}

impl DebugModuleTable for Item {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            Item::Struct(s) => write!(f, "{:#?}", Debuggable::from(s, table)),
            Item::Def(d) => write!(f, "{:#?}", Debuggable::from(d, table)),
        }
    }
}

impl DebugModuleTable for Module {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        let entries: Vec<_> = self
            .items
            .iter()
            .map(|i| Debuggable::from(i, table))
            .collect();

        write!(f, "{:#?}", entries)
    }
}

impl DebugModuleTable for Struct {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        f.debug_struct("Struct")
            .field("name", &table.lookup(self.name.node))
            .field("fields", &DebuggableVec::from(&self.fields, table))
            .finish()
    }
}

impl DebugModuleTable for Field {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        write!(
            f,
            "{}: {:?}",
            &table.lookup(self.name.node),
            &Debuggable::from(&self.ty.node, table)
        )
    }
}

impl DebugModuleTable for Type {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self.mode {
            None => write!(f, "{:?}", &Debuggable::from(&self.name.node, table)),
            Some(mode) => write!(
                f,
                "{:?} {:?}",
                &Debuggable::from(&self.mode, table),
                &Debuggable::from(&self.name.node, table)
            ),
        }
    }
}

impl DebugModuleTable for Option<Spanned<Type>> {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            None => write!(f, "none"),
            Some(ty) => ty.node.debug(f, table),
        }
    }
}

impl DebugModuleTable for StringId {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        write!(f, "{}", table.lookup(*self))
    }
}

impl DebugModuleTable for Option<Spanned<Mode>> {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        let mode = self.map(|i| i.node);

        let result = match mode {
            Some(Mode::Owned) => "owned",
            Some(Mode::Shared) => "shared",
            Some(Mode::Borrowed) => "borrowed",
            None => "none",
        };

        write!(f, "{}", result)
    }
}

impl DebugModuleTable for Def {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        f.debug_struct("Def")
            .field("name", &table.lookup(self.name.node))
            .field("parameters", &DebuggableVec::from(&self.parameters, table))
            .field("ret", &Debuggable::from(&self.ret, table))
            .finish()
    }
}
