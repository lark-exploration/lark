#![feature(const_fn)]
#![feature(specialization)]

mod global;
mod text;

pub use self::global::{GlobalIdentifier, GlobalIdentifierTables};
pub use self::text::Text;
