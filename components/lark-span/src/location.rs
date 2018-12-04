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
    pub fn from_index(
        source_line_indices: &Vec<usize>,
        source_len: usize,
        i: ByteIndex,
    ) -> Result<Location, OutOfBounds> {
        let target = i.0;

        if target > source_len {
            return Err(OutOfBounds);
        }

        // Binary search for range
        let mut pivot = source_line_indices.len() / 2;
        let mut step = pivot / 2;

        loop {
            if step == 0 {
                while source_line_indices[pivot] > target && pivot > 0 {
                    pivot -= 1;
                }

                while pivot < (source_line_indices.len() - 1)
                    && source_line_indices[pivot] < target
                    && source_line_indices[pivot + 1] <= target
                {
                    pivot += 1;
                }
            }

            if source_line_indices[pivot] == target {
                let location = Location {
                    line: pivot,
                    column: 0,
                    byte: i,
                };
                return Ok(location);
            }

            if step == 0 {
                let location = Location {
                    line: pivot,
                    column: target - source_line_indices[pivot],
                    byte: i,
                };
                return Ok(location);
            }

            if source_line_indices[pivot] > target {
                pivot -= step;
                step = step / 2;
            } else {
                pivot += step;
                step = step / 2;
            }
        }
    }

    pub fn as_position(&self) -> languageserver_types::Position {
        languageserver_types::Position::new(self.line as u64, self.column as u64)
    }
}
