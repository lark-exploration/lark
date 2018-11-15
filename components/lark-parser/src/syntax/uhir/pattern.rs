use crate::uhir::{Identifier, Mode, Spanned};

use derive_new::new;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub enum Pattern {
    Underscore,
    Identifier(Identifier, Option<Spanned<Mode>>),
}
