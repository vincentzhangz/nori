//! Byte-offset spans and source-position helpers.

/// Half-open byte range into the original source (`[start, end)`).
///
/// Line/column are computed lazily via [`SourceMap`] when rendering diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    #[inline]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    #[inline]
    pub const fn empty(at: u32) -> Self {
        Self { start: at, end: at }
    }

    #[inline]
    pub const fn size(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    #[inline]
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    #[inline]
    pub fn shrink_to_end(self) -> Self {
        Self::empty(self.end)
    }

    #[inline]
    pub fn as_usize(self) -> (usize, usize) {
        (self.start as usize, self.end as usize)
    }

    /// Slice `source` by this span. Panics if out of bounds.
    #[inline]
    pub fn source_text(self, source: &str) -> &str {
        let (start, end) = self.as_usize();
        &source[start..end]
    }
}

impl From<(u32, u32)> for Span {
    fn from((start, end): (u32, u32)) -> Self {
        Self::new(start, end)
    }
}

impl From<(usize, usize)> for Span {
    fn from((start, end): (usize, usize)) -> Self {
        Self::new(start as u32, end as u32)
    }
}

/// 1-based line and column for a byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourcePosition {
    pub line: u32,
    pub column: u32,
}

/// Precomputed line starts for O(log n) offset → line/column lookup.
#[derive(Debug, Clone)]
pub struct SourceMap {
    /// Byte offset of the first character of each line (0-based).
    line_starts: Vec<u32>,
}

impl SourceMap {
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        Self { line_starts }
    }

    pub fn position(&self, offset: u32) -> SourcePosition {
        let idx = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line_start = self.line_starts[idx];
        SourcePosition {
            line: (idx as u32) + 1,
            column: offset.saturating_sub(line_start) + 1,
        }
    }

    pub fn span_start(&self, span: Span) -> SourcePosition {
        self.position(span.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_map_tracks_lines() {
        let map = SourceMap::new("a\nbc\n");
        assert_eq!(map.position(0), SourcePosition { line: 1, column: 1 });
        assert_eq!(map.position(2), SourcePosition { line: 2, column: 1 });
        assert_eq!(map.position(3), SourcePosition { line: 2, column: 2 });
    }

    #[test]
    fn span_source_text() {
        let span = Span::new(1, 4);
        assert_eq!(span.source_text("abcdef"), "bcd");
    }
}
