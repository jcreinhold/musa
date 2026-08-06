//! Syntax-aware text edits (roadmap §11): the project's canonical mutation
//! mechanism. Score-editing commands (prompt 16) resolve to these.

use text_size::TextRange;

/// A replacement of one source range with new text.
pub struct TextEdit {
    /// Byte range of the original text to replace.
    pub range: TextRange,
    /// Text to put in its place (empty for a deletion).
    pub replacement: String,
}

impl TextEdit {
    /// Create an edit replacing `range` with `replacement`.
    pub fn new(range: TextRange, replacement: impl Into<String>) -> Self {
        Self {
            range,
            replacement: replacement.into(),
        }
    }
}

/// Apply `edits` to `source`, returning the new text.
///
/// Edits may be given in any order; they are applied in source order.
/// Overlapping or out-of-bounds edits are skipped — computing edits so they
/// cannot overlap is the caller's job (an edit command is transactional).
pub fn apply_edits(source: &str, edits: &[TextEdit]) -> String {
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|edit| usize::from(edit.range.start()));
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0usize;
    for edit in sorted {
        let start = usize::from(edit.range.start());
        let end = usize::from(edit.range.end());
        if start < cursor || start > end || end > source.len() {
            continue;
        }
        if let Some(before) = source.get(cursor..start) {
            out.push_str(before);
            out.push_str(&edit.replacement);
            cursor = end;
        }
    }
    if let Some(rest) = source.get(cursor..) {
        out.push_str(rest);
    }
    out
}
