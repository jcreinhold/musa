//! Byte offsets ↔ LSP positions: the one module that knows LSP's coordinate system.
//!
//! Everything else in this crate works in the session's own measure — bytes —
//! and calls here to cross the protocol boundary. LSP positions are 0-based
//! lines with columns counted in UTF-16 code units; the session's spans are
//! byte ranges. The translation is arithmetic over the source text, which is
//! why it lives beside the text it translates and nowhere else: a handler
//! that did its own line counting would be a second, drifting copy of this
//! arithmetic.

use lsp_types::{Position, Range};
use musa_project::{Span, Utf16Offsets};

/// Byte offsets ↔ LSP positions for one source text.
///
/// Built once per document version and answered in `O(log n)` per query —
/// the same bargain the session's own indexes make, for the same reason: a
/// hover asks once, but diagnostics ask per label on every keystroke.
pub(crate) struct LineIndex {
    /// Byte offset of the first character of each line, ascending. Always
    /// starts with `0`, so it is never empty and the search never misses.
    line_starts: Vec<u32>,
    /// One past the last byte of the text.
    end: u32,
    /// The byte ↔ UTF-16 translation over the whole text.
    utf16: Utf16Offsets,
}

impl LineIndex {
    /// Index `source`.
    pub(crate) fn new(source: &str) -> Self {
        let mut line_starts = vec![0_u32];
        line_starts.extend(
            source
                .match_indices('\n')
                .filter_map(|(at, _)| u32::try_from(at.checked_add(1)?).ok()),
        );
        Self {
            line_starts,
            end: u32::try_from(source.len()).unwrap_or(u32::MAX),
            utf16: Utf16Offsets::new(source),
        }
    }

    /// The LSP position of a byte offset.
    ///
    /// A byte offset past the end of the text lands at the end of the last
    /// line, which is where an end-of-file diagnostic wants to point.
    pub(crate) fn position(&self, byte: u32) -> Position {
        let byte = byte.min(self.end);
        // `partition_point` counts the line starts at or before `byte`, which
        // is the 1-based line number; it is never zero because `line_starts`
        // begins with `0`.
        let line = self
            .line_starts
            .partition_point(|start| *start <= byte)
            .saturating_sub(1);
        let start = self.line_starts.get(line).copied().unwrap_or(0);
        Position {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            character: self.utf16.to_utf16(byte).saturating_sub(self.utf16.to_utf16(start)),
        }
    }

    /// The byte offset of an LSP position.
    ///
    /// Clients answer with positions they were given, but a position computed
    /// from a stale version of the document must still land somewhere sane:
    /// a column past the end of its line clamps to the line's end, and a line
    /// past the end of the text clamps to the end of the text.
    pub(crate) fn byte(&self, position: Position) -> u32 {
        let line = usize::try_from(position.line).unwrap_or(usize::MAX);
        let Some(&start) = self.line_starts.get(line) else {
            return self.end;
        };
        let line_end = match self.line_starts.get(line.saturating_add(1)) {
            // The newline byte itself: the end of the line, as a position.
            Some(&next) => next.saturating_sub(1),
            None => self.end,
        };
        let column_start = self.utf16.to_utf16(start);
        let within = self.utf16.to_bytes(column_start.saturating_add(position.character));
        within.min(line_end).min(self.end)
    }

    /// The LSP range of a byte span.
    pub(crate) fn range(&self, span: Span) -> Range {
        Range {
            start: self.position(span.start),
            end: self.position(span.end),
        }
    }

    /// The range covering the whole text — the target of a whole-document
    /// edit such as the formatter's.
    pub(crate) fn whole_document(&self) -> Range {
        Range {
            start: Position::new(0, 0),
            end: self.position(self.end),
        }
    }

    /// Split a span into per-line segments `(line, start_column, length)`,
    /// columns and lengths in UTF-16 code units.
    ///
    /// Semantic tokens occupy one line each, and the source has exactly one
    /// multi-line token kind — the block comment — so the split lives here,
    /// where the line arithmetic lives, rather than beside the legend.
    pub(crate) fn lines_of(&self, span: Span) -> Vec<(u32, u32, u32)> {
        let start = self.position(span.start);
        let end = self.position(span.end);
        if start.line == end.line {
            return vec![(
                start.line,
                start.character,
                end.character.saturating_sub(start.character),
            )];
        }
        let mut segments = Vec::new();
        for line in start.line..=end.line {
            let line_start = usize::try_from(line)
                .ok()
                .and_then(|at| self.line_starts.get(at))
                .copied();
            let Some(line_start) = line_start else { break };
            let segment_start = if line == start.line { span.start } else { line_start };
            // The segment runs to the next line's start — one byte further
            // than the newline — or to the span's end on its last line.
            let next = self
                .line_starts
                .get(usize::try_from(line).unwrap_or(usize::MAX).saturating_add(1))
                .copied()
                .unwrap_or(self.end);
            let segment_end = if line == end.line {
                span.end
            } else {
                next.saturating_sub(1)
            };
            let column = if line == start.line { start.character } else { 0 };
            let length = self
                .utf16
                .to_utf16(segment_end)
                .saturating_sub(self.utf16.to_utf16(segment_start));
            segments.push((line, column, length));
        }
        segments
    }
}

/// Whether `span` covers `byte`: start inclusive, end exclusive, as every
/// range in the session is.
pub(crate) fn covers(span: Span, byte: u32) -> bool {
    span.start <= byte && byte < span.end
}

#[cfg(test)]
mod tests {
    // Test helpers panic on statically-valid inputs: a failure is a bug in
    // the test itself, and panicking is the correct behavior there.
    #![allow(clippy::panic)]

    use super::*;

    /// Byte → position → byte is the identity at character boundaries, in
    /// both measures — the law the whole module hangs on.
    #[test]
    fn position_round_trip_in_ascii_source() {
        let source = "piece \"x\" {\n    meter 4/4;\n}\n";
        let index = LineIndex::new(source);
        for (byte, _) in source.char_indices() {
            assert_eq!(
                index.byte(index.position(u32::try_from(byte).unwrap_or(0))),
                byte as u32
            );
        }
    }

    /// An em dash is three bytes and one code unit; a sharp sign (`♯`, in the
    /// BMP) likewise. Every offset after one disagrees in the two measures,
    /// which is the whole reason this module exists.
    #[test]
    fn utf16_columns_count_code_units_not_bytes() {
        let source = "a — b\n";
        let index = LineIndex::new(source);
        // The `b` sits at byte 6, UTF-16 column 4.
        assert_eq!(index.position(6), Position::new(0, 4));
        assert_eq!(index.byte(Position::new(0, 4)), 6);
        // Past the line's end clamps to the end of the line — the position
        // after `b` — rather than wandering into the next line.
        assert_eq!(index.byte(Position::new(0, 40)), 7);
    }

    /// A supplementary-plane character is two UTF-16 code units, so the
    /// column after it advances by two while the byte offset advances by four.
    #[test]
    fn supplementary_plane_characters_cost_two_code_units() {
        let source = "𝄞 clef\n";
        let index = LineIndex::new(source);
        assert_eq!(index.position(4), Position::new(0, 2));
        assert_eq!(index.byte(Position::new(0, 2)), 4);
    }

    #[test]
    fn multi_line_spans_split_at_line_boundaries() {
        let source = "one\ntwo three\nfour\n";
        let index = LineIndex::new(source);
        // The span "o three\nfo" crosses one newline.
        let span = Span { start: 6, end: 16 };
        assert_eq!(index.lines_of(span), vec![(1, 2, 7), (2, 0, 2)]);
    }

    /// Every committed example converts without a panic, and its end maps to
    /// the last line — the property an end-of-file diagnostic relies on.
    #[test]
    fn examples_convert_end_to_end() {
        let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let mut entries: Vec<_> = std::fs::read_dir(&examples)
            .unwrap_or_else(|_| panic!("examples directory"))
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "musa"))
            .collect();
        entries.sort();
        assert!(!entries.is_empty());
        for path in entries {
            let source = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {path:?}"));
            let index = LineIndex::new(&source);
            let end = u32::try_from(source.len()).unwrap_or(0);
            let position = index.position(end);
            assert_eq!(index.byte(position), end, "{path:?}");
        }
    }
}
