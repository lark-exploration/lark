use crate::ByteIndex;
use derive_new::new;
use lark_debug_derive::DebugWith;

#[derive(Debug, DebugWith, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Location {
    /// 0-based line number
    pub line: usize,

    /// 0-based column number, in utf-8 characters
    pub column: usize,

    /// byte index into file text
    pub byte: ByteIndex,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct OutOfBounds;

impl Location {
    pub fn as_position(&self) -> languageserver_types::Position {
        languageserver_types::Position::new(self.line as u64, self.column as u64)
    }
}
