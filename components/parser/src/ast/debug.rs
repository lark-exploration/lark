#![allow(unused_variables)]

use super::*;
use crate::ModuleTable;

use std::fmt;

pub struct DebuggableVec<'owner, T: DebugModuleTable> {
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

pub struct Debuggable<'owner, T: DebugModuleTable> {
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
            .field("body", &Debuggable::from(&self.body, table))
            .finish()
    }
}

impl<T: DebugModuleTable> DebugModuleTable for Spanned<T> {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        self.node.debug(f, table)
    }
}

impl<T: DebugModuleTable> DebugModuleTable for Arc<T> {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        T::debug(self, f, table)
    }
}

impl DebugModuleTable for Block {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        write!(f, "{:#?}", DebuggableVec::from(&self.expressions, table))
    }
}

impl DebugModuleTable for BlockItem {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            BlockItem::Item(item) => write!(f, "{:?}", Debuggable::from(item, table)),
            BlockItem::Decl(decl) => write!(f, "{:?}", Debuggable::from(decl, table)),
            BlockItem::Expr(expr) => write!(f, "{:?}", Debuggable::from(expr, table)),
        }
    }
}

impl DebugModuleTable for Declaration {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            Declaration::Let(let_decl) => let_decl.debug(f, table),
        }
    }
}

impl DebugModuleTable for Let {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        write!(f, "let ")?;
        self.pattern.node.debug(f, table)?;

        match &self.ty {
            None => {}
            Some(ty) => unimplemented!("Debug output for annotated lets"),
        };

        match &self.init {
            None => {}
            Some(init) => {
                write!(f, " = ");
                init.debug(f, table)?
            }
        };

        Ok(())
    }
}

impl DebugModuleTable for Pattern {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            Pattern::Underscore => write!(f, "_"),
            Pattern::Identifier(id, Some(mode)) => {
                mode.debug(f, table)?;
                write!(f, " ");
                id.debug(f, table)
            }
            Pattern::Identifier(id, None) => id.debug(f, table),
        }
    }
}

impl DebugModuleTable for Mode {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        let out: &str = (*self).into();
        write!(f, "{}", out)
    }
}

impl DebugModuleTable for Expression {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        use self::Expression::*;

        match self {
            Block(block) => write!(f, "<block>"),
            ConstructStruct(construct) => {
                construct.name.debug(f, table)?;
                write!(f, " {{ ... }}")
            }
            Call(call) => call.debug(f, table),
            Ref(id) => id.debug(f, table),
            Binary(op, box left, box right) => {
                left.debug(f, table)?;
                write!(f, " {} ", op.node)?;
                right.debug(f, table)
            }
            Interpolation(elements, span) => write!(f, "<interpolation>"),
            Literal(literal) => literal.debug(f, table),
        }
    }
}

impl DebugModuleTable for Literal {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            Literal::String(id) => id.debug(f, table),
        }
    }
}

impl DebugModuleTable for Call {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self.callee {
            Callee::Identifier(id) => id.debug(f, table)?,
        };

        write!(f, "( ... )")
    }
}
