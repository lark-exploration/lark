use crate::parser::ast::Modifiers;
use crate::parser::StringId;
use derive_new::new;
use smart_default::SmartDefault;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ast<'input> {
    input: &'input str,
}

impl Ast<'input> {
    crate fn empty() -> Ast<'input> {
        Ast { input: "" }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Item {
    Struct(Struct),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Module {
    items: Vec<Item>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Struct {
    name: StringId,
    fields: Vec<Field>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Field {
    name: StringId,
    modifiers: Modifiers,
    ty: Type,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Type {
    name: StringId,
}
