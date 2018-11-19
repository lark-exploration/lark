use crate::ByteIndex;
use derive_new::new;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Location {
    /// 0-based line number
    line: usize,

    /// 0-based column number, in utf-8 characters
    column: usize,

    /// byte index into file text
    byte: ByteIndex,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct OutOfBounds;

impl Location {
    pub fn from_index(s: &str, i: ByteIndex) -> Result<Location, OutOfBounds> {
        let target = i.0;
        println!("target={}", target);

        let mut seen_lines = 0;
        let mut last = 0;

        for (pos, _) in s.match_indices('\n') {
            let pos = pos + 1;

            println!(
                "pos={} last={} seen_lines={} text={:?}",
                pos,
                last,
                seen_lines,
                &s[..pos]
            );

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

        Err(OutOfBounds)
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
