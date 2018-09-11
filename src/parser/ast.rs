use codespan::ByteIndex;
use crate::parser::pos::HasSpan;
use crate::parser::pos::Span;
use crate::parser::pos::Spanned;
use crate::parser::{Environment, ModuleTable, StringId, Token};
use derive_new::new;
use std::fmt;

pub type Identifier = Spanned<StringId>;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ast<'input> {
    input: &'input str,
    module: Module,
}

impl Ast<'input> {
    crate fn empty() -> Ast<'input> {
        Ast {
            input: "",
            module: Module { items: vec![] },
        }
    }

    crate fn module(self) -> Module {
        self.module
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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Item {
    Struct(Struct),
    Def(Def),
}

impl DebugModuleTable for Item {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            Item::Struct(s) => write!(f, "{:?}", Debuggable::from(s, table)),
            Item::Def(d) => write!(f, "{:?}", Debuggable::from(d, table)),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Module {
    items: Vec<Item>,
}

impl DebugModuleTable for Module {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        let entries: Vec<_> = self
            .items
            .iter()
            .map(|i| Debuggable::from(i, table))
            .collect();

        write!(f, "{:?}", entries)
    }
}

#[cfg(test)]
impl Module {
    crate fn build() -> Module {
        Module { items: vec![] }
    }

    crate fn add_struct(mut self, s: Struct) -> Module {
        self.items.push(Item::Struct(s));
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Struct {
    name: Spanned<StringId>,
    fields: Vec<Field>,
    span: Span,
}

struct DebugStruct<'a> {
    name: &'a str,
    fields: Vec<DebugField<'a>>,
}

impl DebugModuleTable for Struct {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        f.debug_struct("Struct")
            .field("name", &table.lookup(self.name.node))
            .field(
                "fields",
                &format_args!(
                    "{:?}",
                    self.fields
                        .iter()
                        .map(|f| Debuggable::from(f, table))
                        .collect::<Vec<_>>()
                ),
            ).finish()
    }
}

impl HasSpan for Struct {
    type Inner = Struct;

    fn span(&self) -> Span {
        self.span
    }
}

#[cfg(test)]
impl Struct {
    crate fn build(name: &'input str, table: &mut ModuleTable) -> Struct {
        Struct {
            name: Spanned::synthetic(table.intern(name)),
            fields: vec![],
            span: Span::Synthetic,
        }
    }

    crate fn spanned(mut self, start: u32, end: u32) -> Struct {
        self.span = Span::from(ByteIndex(start), ByteIndex(end));
        self
    }

    crate fn name_spanned(mut self, start: u32, end: u32) -> Struct {
        self.name.span = Span::from(ByteIndex(start), ByteIndex(end));
        self
    }

    crate fn field(mut self, field: Field) -> Struct {
        self.fields.push(field);
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Field {
    name: Spanned<StringId>,
    ty: Spanned<Type>,
}

impl DebugModuleTable for Field {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        f.debug_struct("Field")
            .field("name", &table.lookup(self.name.node))
            .finish()
    }
}

struct DebugField<'a> {
    name: &'a str,
    ty: DebugType<'a>,
}

#[cfg(test)]
impl Field {
    crate fn build(name: &'input str, ty: Type, table: &mut ModuleTable) -> Field {
        Field {
            name: Spanned::synthetic(table.intern(name)),
            ty: Spanned::synthetic(ty),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Type {
    mode: Option<Spanned<Mode>>,
    name: Spanned<StringId>,
}

struct DebugType<'a> {
    name: &'a str,
}

#[cfg(test)]
impl Type {
    crate fn build(name: &'input str, mode: Mode, table: &mut ModuleTable) -> Type {
        Type {
            mode: Some(Spanned::synthetic(mode)),
            name: Spanned::synthetic(table.intern(name)),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Mode {
    Owned,
    Shared,
    Borrowed,
}

pub struct Pattern {}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Path {
    components: Vec<Identifier>,
}

pub enum Statement {}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Def {
    name: Identifier,
    parameters: Vec<Field>,
    ret: Option<Spanned<Type>>,
    body: Block,
    span: Span,
}

impl DebugModuleTable for Def {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        f.debug_struct("Def").finish()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Expression {
    Block(Block),
}

pub struct Let {
    pattern: Pattern,
    ty: Option<Type>,
    init: Option<Expression>,
}

pub enum If {
    If(Box<Expression>, Block, Option<ChainedElse>),
    IfLet(Pattern, Box<Expression>, Block, Option<ChainedElse>),
}

pub enum ChainedElse {
    Block(Block),
    If(Box<If>),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Block {
    expressions: Vec<Expression>,
}
