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
    pub fn from_index_with_line_indices(
        source_line_indices: &Vec<usize>,
        source_len: usize,
        i: ByteIndex,
    ) -> Result<Location, OutOfBounds> {
        let target = i.0 - 1;

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
                    line: pivot + 1,
                    column: 0,
                    byte: i,
                };
                return Ok(location);
            }

            if step == 0 {
                let location = Location {
                    line: pivot + 1,
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

    pub fn from_index(s: &str, i: ByteIndex) -> Result<Location, OutOfBounds> {
        let target = i.0;

        let mut seen_lines = 0;
        let mut last = 0;

        if target > s.len() {
            return Err(OutOfBounds);
        }

        for (pos, _) in s.match_indices('\n') {
            let pos = pos + 1;

            if pos == target {
                return Ok(Location {
                    line: seen_lines + 1,
                    column: 0,
                    byte: i,
                });
            } else if pos > target {
                return Ok(Location {
                    line: seen_lines,
                    column: target - last,
                    byte: i,
                });
            } else {
                last = pos;
                seen_lines += 1;
            }
        }

        Ok(Location {
            line: 0,
            column: target,
            byte: i,
        })
    }

    pub fn as_position(&self) -> languageserver_types::Position {
        languageserver_types::Position::new(self.line as u64, self.column as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::{Location, OutOfBounds};
    use crate::ByteIndex;

    struct TestData {
        string: String,
        lines: Vec<ByteIndex>,
    }

    fn data() -> TestData {
        let mut out = String::new();
        let mut offsets = vec![];
        let mut offset = 0;

        for s in vec!["hello!\n", "howdy\n", "\n", "hi萤\n", "bloop\n"] {
            offsets.push(ByteIndex(offset));
            offset += s.len();
            out.push_str(s);
        }

        offsets.push(ByteIndex(offset));

        // Go off the edge
        offsets.push(ByteIndex(offset + 1));

        TestData {
            string: out,
            lines: offsets,
        }
    }

    #[test]
    fn start_line_location() {
        let test_data = data();
        let offsets: Vec<_> = test_data
            .lines
            .iter()
            .map(|&i| Location::from_index(&test_data.string, i))
            .collect();

        assert_eq!(
            offsets,
            vec![
                Ok(Location::new(0, 0, ByteIndex(0))),
                Ok(Location::new(1, 0, ByteIndex(7))),
                Ok(Location::new(2, 0, ByteIndex(13))),
                Ok(Location::new(3, 0, ByteIndex(14))),
                Ok(Location::new(4, 0, ByteIndex(20))),
                Ok(Location::new(5, 0, ByteIndex(26))),
                Err(OutOfBounds)
            ],
        );
    }

    #[test]
    fn mid_line_location() {
        let test_data = data();

        let l = Location::from_index(&test_data.string, ByteIndex(0));
        assert_eq!(l, Ok(Location::new(0, 0, ByteIndex(0))));
    }

}
