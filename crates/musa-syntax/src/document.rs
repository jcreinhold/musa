//! Which of the two alternatives a source document is
//! (`docs/rules/language/01-surface.md` §7).
//!
//! A musa document is written in one of two languages: the surface language
//! this crate parses, or the event track interchange language `musa-events` parses.
//! Telling them apart is a *lexical* question — the first line either is the
//! events version marker or it is not — so it is answered here, above both
//! parsers, and answered before either runs.
//!
//! This module deliberately knows nothing else about events text. It cannot
//! parse it, format it, or say what it means; it can only say that something
//! else should. Reading the marker is the whole of the coupling, and
//! `musa-compiler`'s `events_document_marker_matches_the_events` law holds
//! this string to `musa_events::FORMAT_VERSION` so the two cannot drift.

/// The first line of every event track interchange file.
///
/// Spelled here rather than imported because `musa-syntax` does not depend
/// on `musa-events` and should not: a frontend that could reach the event track
/// would eventually reach past the marker. The copy is checked, not trusted.
pub const EVENTS_MARKER: &str = "% musa-events-3";

/// Which language a document's text is written in.
///
/// Two variants and no `Unknown`: text that is not marked as event track is surface
/// text, possibly bad surface text, and "possibly bad" is what the parser's
/// error recovery is for. A third variant would make every caller handle a
/// case that has no behaviour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DocumentAlternative {
    /// `.musa` — the surface language ([`crate::parse`]).
    #[default]
    Surface,
    /// `.musa.events` — the interchange language, marked by [`EVENTS_MARKER`].
    Events,
}

/// Which alternative `text` is written in.
///
/// Decided by the first line alone, and by its content rather than by a file
/// extension: an unsaved buffer has no name, a pasted fragment has no path,
/// and a file that has been renamed is still whatever it says it is. The
/// marker is a comment in neither language's other half, which is what makes
/// the test total — surface musa cannot begin with `%`.
///
/// Trailing whitespace is ignored, because an editor may add it and a document
/// does not change language when it does. Leading whitespace is *not*: the
/// marker is a version header, and a version header that can be indented is a
/// version header a producer can write wrong.
pub fn alternative(text: &str) -> DocumentAlternative {
    if text.lines().next().unwrap_or_default().trim_end() == EVENTS_MARKER {
        DocumentAlternative::Events
    } else {
        DocumentAlternative::Surface
    }
}
