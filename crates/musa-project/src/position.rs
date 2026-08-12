//! Byte offsets → the line and column a person would count.
//!
//! [`utf16`](crate::utf16) translates the *measure* a span is stated in.
//! This module translates the *coordinate*: `184` is where the compiler found
//! the problem, `12:5` is where the reader has to look, and turning one into
//! the other is arithmetic over the source text that no frontend should be
//! doing. `docs/rules/desktop/03-interaction.md` §7 lists what the frontend may
//! compute and line numbers are not on it, so they are computed here and
//! travel with the diagnostic.
//!
//! Both coordinates are 1-based, because that is what every editor's status
//! bar and every compiler's output already say. The column counts
//! **characters**, not bytes and not UTF-16 code units: an em dash is one
//! character to the person counting across the line, whatever it costs to
//! store.

/// Where a byte offset falls in a source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub struct Position {
    /// 1-based line, counting `\n`.
    pub line: u32,
    /// 1-based column, counted in characters from the start of the line.
    pub column: u32,
}

/// The line starts of one source text, for repeated lookups.
///
/// A diagnostic has several labels and a session has several diagnostics, so
/// the index is built once per translation and answered in `O(log n)` — the
/// same bargain [`Utf16Offsets`](crate::Utf16Offsets) makes next door.
#[derive(Debug)]
pub(crate) struct Lines<'source> {
    source: &'source str,
    /// Byte offset of the first character of each line, ascending. Always
    /// starts with `0`, so it is never empty and the search never misses.
    starts: Vec<u32>,
}

impl<'source> Lines<'source> {
    /// Index `source`.
    pub(crate) fn new(source: &'source str) -> Self {
        let mut starts = vec![0_u32];
        starts.extend(source.match_indices('\n').filter_map(|(at, _)| {
            let after = at.checked_add(1)?;
            u32::try_from(after).ok()
        }));
        Self { source, starts }
    }

    /// The line and column of `offset`.
    ///
    /// An offset past the end of the text lands at the end of the last line,
    /// which is where a "the file ends here" diagnostic wants to point.
    pub(crate) fn at(&self, offset: u32) -> Position {
        let offset = offset.min(u32::try_from(self.source.len()).unwrap_or(u32::MAX));
        // `partition_point` gives the count of starts at or before `offset`,
        // which is the 1-based line number, and never zero because `starts`
        // begins with `0`.
        let line = self.starts.partition_point(|start| *start <= offset);
        let start = self.starts.get(line.saturating_sub(1)).copied().unwrap_or(0);
        let head = self
            .source
            .get(usize::try_from(start).unwrap_or(0)..usize::try_from(offset).unwrap_or(0))
            .unwrap_or("");
        Position {
            line: u32::try_from(line).unwrap_or(1).max(1),
            column: u32::try_from(head.chars().count().saturating_add(1)).unwrap_or(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Lines;

    #[test]
    fn the_first_character_is_one_one() {
        let lines = Lines::new("piece \"x\" { }");
        assert_eq!((lines.at(0).line, lines.at(0).column), (1, 1));
        assert_eq!((lines.at(6).line, lines.at(6).column), (1, 7));
    }

    #[test]
    fn a_newline_starts_the_next_line() {
        let source = "one\ntwo\nthree";
        let lines = Lines::new(source);
        let at = |needle: &str| {
            let offset = u32::try_from(source.find(needle).unwrap_or(0)).unwrap_or(0);
            let position = lines.at(offset);
            (position.line, position.column)
        };
        assert_eq!(at("one"), (1, 1));
        assert_eq!(at("two"), (2, 1));
        assert_eq!(at("three"), (3, 1));
    }

    #[test]
    fn a_column_counts_characters_not_bytes() {
        // Four characters precede the `c`, but seven bytes do.
        let source = "% —é\ncello";
        let lines = Lines::new(source);
        let offset = u32::try_from(source.find('c').unwrap_or(0)).unwrap_or(0);
        assert_eq!((lines.at(offset).line, lines.at(offset).column), (2, 1));
        let dash = u32::try_from(source.find('é').unwrap_or(0)).unwrap_or(0);
        assert_eq!((lines.at(dash).line, lines.at(dash).column), (1, 4));
    }

    #[test]
    fn the_end_of_the_file_is_the_end_of_the_last_line() {
        let source = "a\nbc";
        let lines = Lines::new(source);
        let end = lines.at(u32::try_from(source.len()).unwrap_or(0));
        assert_eq!((end.line, end.column), (2, 3));
        // Past the end lands in the same place rather than panicking: an
        // empty span at EOF is how an unclosed block reports itself.
        assert_eq!(lines.at(9_999), end);
    }
}
