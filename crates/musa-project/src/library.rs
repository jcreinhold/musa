//! Bundled library documents, as an interface opens one.
//!
//! A standard-library module is compiled into the binary, so there is no file
//! for a window to open. A reader following `triad` to its declaration is
//! shown the module's text in a document the application makes — read-only,
//! because an edit would have nowhere to land
//! (`docs/rules/desktop/08-elaboration.md` §3).
//!
//! This module answers with the text *and* the place inside it, because those
//! two facts are measured against each other. A caller that asked for the text
//! and then restated a byte offset in its own units would be doing the one
//! translation this crate exists to keep in one place (`crate::utf16`).

use crate::diagnostic::Span;

/// A bundled module's text, and where in it the reader was going.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDocument {
    /// The readable virtual URI it was asked for by.
    pub uri: String,
    /// The module as the language spells it: `pitch`, `theory::cadence`.
    pub name: String,
    /// The module's whole source.
    pub text: String,
    /// What to reveal in it, in `text`'s own UTF-16 code units. Absent when
    /// the module was opened for its own sake rather than followed into.
    pub span: Option<Span>,
}

/// The bundled module `uri` names, with `at` restated in its own measure.
///
/// `at` is a byte range in that module — the range a
/// [`ProjectSnapshot`](crate::ProjectSnapshot) handed out with the URI beside
/// it, which the caller passes back untouched. Nothing else may be asked for:
/// a range this module did not produce would be a caller measuring another
/// document's text.
///
/// `None` for a URI no bundled module answers to. Refused rather than answered
/// with an empty document, for the reason the language server refuses it: a
/// reader shown a blank standard library would conclude the module is empty.
#[must_use]
pub fn library_document(uri: &str, at: Option<(u32, u32)>) -> Option<LibraryDocument> {
    let text = musa_compiler::standard_library_source(uri)?;
    let name = musa_compiler::standard_library_module(uri)?;
    let offsets = crate::Utf16Offsets::new(text);
    Some(LibraryDocument {
        uri: uri.to_owned(),
        name,
        text: text.to_owned(),
        span: at.map(|(start, end)| Span {
            start: offsets.to_utf16(start),
            end: offsets.to_utf16(end),
        }),
    })
}
