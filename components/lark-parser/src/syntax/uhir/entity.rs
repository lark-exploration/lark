use crate::uhir::{Block, Identifier, Span, Spanned, Type};

use derive_new::new;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub enum Entity {
    Struct(Struct),
    Def(Def),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Struct {
    pub name: Identifier,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Field {
    pub name: Identifier,
    pub ty: Spanned<Type>,
    pub span: Span,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Def {
    pub name: Identifier,
    pub parameters: Vec<Field>,
    pub ret: Option<Spanned<Type>>,
    pub body: Spanned<Block>,
    pub span: Span,
}
