//! Byte ranges into a source file.

use std::fmt;

/// A half-open byte range `[start, end)` into a source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    /// First byte included in the range.
    pub start: u32,
    /// First byte after the range.
    pub end: u32,
}

impl Span {
    /// Creates a span from two byte offsets.
    ///
    /// The offsets are not validated against a source file; callers are expected
    /// to derive them from the same buffer.
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Creates an empty span at `offset`.
    #[must_use]
    pub const fn empty(offset: u32) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    /// Returns the length in bytes.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Returns `true` when the range is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Returns `true` when `offset` is inside the half-open range.
    #[must_use]
    pub const fn contains(self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Returns the covered source text, or `None` when the span is out of bounds.
    #[must_use]
    pub fn text(self, source: &str) -> Option<&str> {
        let start = usize::try_from(self.start).ok()?;
        let end = usize::try_from(self.end).ok()?;
        source.get(start..end)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}..{}", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::Span;

    #[test]
    fn contains_is_half_open() {
        let span = Span::new(2, 4);
        assert!(!span.contains(1));
        assert!(span.contains(2));
        assert!(span.contains(3));
        assert!(!span.contains(4));
    }

    #[test]
    fn text_returns_the_covered_bytes() {
        assert_eq!(Span::new(1, 4).text("abcde"), Some("bcd"));
        assert_eq!(Span::new(1, 9).text("abcde"), None);
    }
}
