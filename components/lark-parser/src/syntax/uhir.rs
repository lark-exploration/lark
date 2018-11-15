mod block;
mod entity;
mod pattern;
mod ty;

use lark_span::{FileName, Span as GenericSpan, Spanned as GenericSpanned};
use lark_string::GlobalIdentifier;

pub type Spanned<T> = GenericSpanned<T, FileName>;
pub type Span = GenericSpan<FileName>;
pub type Identifier = Spanned<GlobalIdentifier>;

pub use self::block::*;
pub use self::entity::*;
pub use self::pattern::*;
pub use self::ty::*;
