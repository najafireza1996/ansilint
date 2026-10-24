//! Byte offset to line/column conversion.

use crate::Span;

/// A one-based line and column position.
///
/// Columns count Unicode scalar values, not bytes and not UTF-16 code units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineCol {
    /// One-based line number.
    pub line: u32,
    /// One-based column number, counted in Unicode scalar values.
    pub col: u32,
}

#[derive(Clone, Copy, Debug)]
struct Line {
    start: u32,
    end: u32,
}

/// Maps byte offsets to lines without rescanning the whole source.
///
/// Line terminators are excluded from line spans. Both `\n` and `\r\n` are
/// recognised; a lone `\r` is treated as an ordinary character.
#[derive(Clone, Debug)]
pub struct LineIndex {
    lines: Vec<Line>,
    len: u32,
}

impl LineIndex {
    /// Builds an index for `text` in a single pass.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let mut lines = Vec::new();
        let mut start = 0_usize;
        for (index, byte) in text.bytes().enumerate() {
            if byte != b'\n' {
                continue;
            }
            let end = if index > start && text.as_bytes().get(index - 1) == Some(&b'\r') {
                index - 1
            } else {
                index
            };
            lines.push(Line {
                start: to_u32(start),
                end: to_u32(end),
            });
            start = index + 1;
        }
        lines.push(Line {
            start: to_u32(start),
            end: to_u32(text.len()),
        });
        Self {
            lines,
            len: to_u32(text.len()),
        }
    }

    /// Returns the number of lines. A file ending in a newline has a final
    /// empty line.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Returns the byte length indexed by this structure.
    #[must_use]
    pub const fn text_len(&self) -> u32 {
        self.len
    }

    /// Returns the span of a zero-based line, excluding its terminator.
    #[must_use]
    pub fn line_span(&self, line: usize) -> Option<Span> {
        self.lines
            .get(line)
            .map(|line| Span::new(line.start, line.end))
    }

    /// Returns the text of a zero-based line, excluding its terminator.
    #[must_use]
    pub fn line_text<'a>(&self, text: &'a str, line: usize) -> Option<&'a str> {
        let span = self.line_span(line)?;
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        text.get(start..end)
    }

    /// Converts a byte offset into a one-based line and column.
    ///
    /// Offsets beyond the end of the source are clamped. Offsets inside a
    /// multi-byte character count the characters that end before the offset.
    #[must_use]
    pub fn line_col(&self, text: &str, offset: u32) -> LineCol {
        let offset = offset.min(self.len);
        let index = self
            .lines
            .partition_point(|line| line.start <= offset)
            .saturating_sub(1);
        let Some(line) = self.lines.get(index) else {
            return LineCol { line: 1, col: 1 };
        };
        let start = usize::try_from(line.start).unwrap_or(usize::MAX);
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        let col = text
            .get(start..offset)
            .map_or(1, |slice| slice.chars().count().saturating_add(1));
        LineCol {
            line: u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX),
            col: u32::try_from(col).unwrap_or(u32::MAX),
        }
    }
}

fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "tests use fixed fixtures and panic on broken assertions"
    )]

    use super::{LineCol, LineIndex};
    use crate::Span;
    use proptest::prelude::*;

    fn text_strategy() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            prop_oneof![Just('\n'), Just('\r'), Just('é'), Just('日'), any::<char>()],
            0..40,
        )
        .prop_map(|characters| characters.into_iter().collect())
    }

    #[test]
    fn counts_lines_and_excludes_terminators() {
        let index = LineIndex::new("a\r\nb\n");
        assert_eq!(index.line_count(), 3);
        assert_eq!(index.line_text("a\r\nb\n", 0), Some("a"));
        assert_eq!(index.line_text("a\r\nb\n", 1), Some("b"));
        assert_eq!(index.line_text("a\r\nb\n", 2), Some(""));
        assert_eq!(index.line_span(0), Some(Span::new(0, 1)));
        assert_eq!(index.line_span(1), Some(Span::new(3, 4)));
    }

    #[test]
    fn columns_count_unicode_scalar_values() {
        let text = "é\n日本";
        let index = LineIndex::new(text);
        assert_eq!(index.line_col(text, 0), LineCol { line: 1, col: 1 });
        assert_eq!(index.line_col(text, 2), LineCol { line: 1, col: 2 });
        assert_eq!(index.line_col(text, 3), LineCol { line: 2, col: 1 });
        assert_eq!(index.line_col(text, 6), LineCol { line: 2, col: 2 });
    }

    #[test]
    fn clamps_offsets_past_the_end() {
        let text = "a\n";
        let index = LineIndex::new(text);
        assert_eq!(index.line_col(text, 99), LineCol { line: 2, col: 1 });
    }

    proptest! {
        #[test]
        fn line_col_matches_a_naive_scan(text in text_strategy()) {
            let index = LineIndex::new(&text);
            for (offset, _) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
                let line = text.get(..offset).map_or(0, |prefix| prefix.matches('\n').count());
                let line_start = text
                    .get(..offset)
                    .and_then(|prefix| prefix.rfind('\n'))
                    .map_or(0, |index| index + 1);
                let col = text
                    .get(line_start..offset)
                    .map_or(0, |slice| slice.chars().count());
                let offset = u32::try_from(offset).expect("test offsets fit in u32");
                let expected = LineCol {
                    line: u32::try_from(line + 1).expect("test line fits in u32"),
                    col: u32::try_from(col + 1).expect("test column fits in u32"),
                };
                prop_assert_eq!(index.line_col(&text, offset), expected);
            }
        }

        #[test]
        fn line_spans_are_ordered_and_bounded(text in text_strategy()) {
            let index = LineIndex::new(&text);
            let mut previous_end = 0;
            for line in 0..index.line_count() {
                let span = index.line_span(line).expect("line exists");
                prop_assert!(span.start >= previous_end);
                prop_assert!(span.end <= u32::try_from(text.len()).expect("test text fits"));
                prop_assert!(span.start <= span.end);
                previous_end = span.end;
            }
        }
    }
}
