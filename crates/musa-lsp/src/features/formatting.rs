//! Formatting: the canonical layout, without touching the session.
//!
//! A format request is answered by `musa-language`'s formatter directly —
//! never by `ProjectCommand::Format`, which is a source-changing command and
//! would land in the session's undo history. The client applies the edit and
//! the change comes back through `didChange` like any other, so the session's
//! history holds exactly one entry for it: the composer's.

use lsp_types::TextEdit;

use crate::workspace::Document;

/// The whole-document edit that makes the source canonical, or `None` when
/// it already is — the same distinction `musa format` and `musa format
/// --check` draw.
pub(crate) fn format(document: &Document) -> Option<Vec<TextEdit>> {
    let text = document.formatted_source();
    if text == document.snapshot().source() {
        return None;
    }
    Some(vec![TextEdit {
        range: document.lines().whole_document(),
        new_text: text,
    }])
}
