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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Item {
    Struct(Struct),
    Def(Def),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockItem {
    Item(Item),
    Decl(Declaration),
    Expr(Expression),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Declaration {
    Let,
}

impl DebugModuleTable for Item {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            Item::Struct(s) => write!(f, "{:#?}", Debuggable::from(s, table)),
            Item::Def(d) => write!(f, "{:#?}", Debuggable::from(d, table)),
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

        write!(f, "{:#?}", entries)
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

    crate fn def(mut self, def: Def) -> Module {
        self.items.push(Item::Def(def));
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Struct {
    name: Spanned<StringId>,
    fields: Vec<Field>,
    span: Span,
}

impl DebugModuleTable for Struct {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        f.debug_struct("Struct")
            .field("name", &table.lookup(self.name.node))
            .field("fields", &DebuggableVec::from(&self.fields, table))
            .finish()
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
    name: Identifier,
    ty: Spanned<Type>,
    span: Span,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ConstructField {
    Longhand(Field),
    Shorthand(Identifier),
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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Type {
    mode: Option<Spanned<Mode>>,
    name: Spanned<StringId>,
}

impl DebugModuleTable for Type {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self.mode {
            None => write!(f, "{:?}", &Debuggable::from(&self.mode, table)),
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

struct DebugType<'a> {
    name: &'a str,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Mode {
    Owned,
    Shared,
    Borrowed,
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

#[cfg(test)]
impl Def {
    pub fn parameter(mut self, field: Field) -> Self {
        self.parameters.push(field);
        self
    }

    pub fn ret(mut self, ty: Option<Spanned<Type>>) -> Self {
        self.ret = ty;
        self
    }

    crate fn spanned(mut self, start: u32, end: u32) -> Self {
        self.span = Span::from(ByteIndex(start), ByteIndex(end));
        self
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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Expression {
    Block(Block),
    ConstructStruct(ConstructStruct),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct ConstructStruct {
    name: Identifier,
    fields: Vec<ConstructField>,
    span: Span,
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
    expressions: Vec<BlockItem>,
}
