use super::{Identifier, Spanned};

use derive_new::new;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Type {
    pub mode: Option<Spanned<Mode>>,
    pub name: Identifier,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Mode {
    Owned,
    Shared,
    Borrowed,
}
