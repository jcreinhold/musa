//! The open documents: one session per URI, nothing else.
//!
//! The protocol says a server holds documents; the session says a document is
//! one canonical source that recompiles on every change and keeps its last
//! valid reading while the text is broken. This module is where those two
//! sentences meet. It owns the sessions and the line indexes that translate
//! their spans, and it knows the document lifecycle — open, change, close —
//! and no request semantics: handlers ask for a [`Document`] and read facts.

use std::collections::HashMap;

use lsp_types::Uri;
use musa_language::DocumentAlternative;
use musa_project::{ProjectCommand, ProjectSession, ProjectSnapshot};

use crate::convert::LineIndex;

/// One open document: its session, and the index of the source the session
/// holds, rebuilt together so the two can never describe different texts.
pub(crate) struct Document {
    /// The canonical source and everything compiled from it.
    session: ProjectSession,
    /// Byte ↔ LSP-position translation over the current source.
    lines: LineIndex,
    /// The client's version counter, echoed back with diagnostics so a stale
    /// editor can tell a late answer from a wrong one.
    version: i32,
}

impl Document {
    /// Open `source` under `uri`.
    fn new(uri: &Uri, source: String, version: i32) -> Self {
        let lines = LineIndex::new(&source);
        let session = ProjectSession::from_text(source, document_name(uri));
        Self {
            session,
            lines,
            version,
        }
    }

    /// Everything observable right now: diagnostics, facts, the source.
    pub(crate) fn snapshot(&self) -> ProjectSnapshot<'_> {
        self.session.snapshot()
    }

    /// Byte ↔ LSP-position translation over the current source.
    pub(crate) fn lines(&self) -> &LineIndex {
        &self.lines
    }

    /// The session itself, for the one operation that is a question rather
    /// than a fact: an analysis is run on request and is not part of any
    /// snapshot ([`features::analysis`](crate::features::analysis)).
    pub(crate) fn session(&self) -> &ProjectSession {
        &self.session
    }

    /// The canonical layout of this document's text.
    ///
    /// Asked of the session rather than of the formatter, because the layout
    /// a project asks for in its `musa.toml` is the session's to know — a
    /// second caller of `musa_language::format` here is a second answer, and
    /// an editor that disagrees with `musa format` is the whole failure the
    /// setting has to avoid. It is a question, not a command: nothing about
    /// the session changes, so no undo entry appears.
    pub(crate) fn formatted_source(&self) -> String {
        self.session.formatted_source()
    }

    /// The client's version counter at the last change.
    pub(crate) fn version(&self) -> i32 {
        self.version
    }

    /// Which language this document is written in
    /// (`docs/language/01-surface.md` §7).
    ///
    /// Asked of the text rather than of the URI, because a client may open an
    /// unsaved buffer, a renamed file, or a scratch pane, and what a document
    /// *is* has to survive all three. Every feature that reads a surface
    /// syntax tree or maps a compiled fact back to a span in *this* file has
    /// to ask: a kernel document's facts carry provenance into the source that
    /// produced them, which is a different file, so a hover or a definition
    /// resolved that way would point somewhere the user is not.
    pub(crate) fn alternative(&self) -> DocumentAlternative {
        musa_language::alternative(self.session.snapshot().source())
    }
}

/// The name the compiler sees for the document behind `uri`: the file's own
/// path, because imports resolve against it as a filesystem path. Naming the
/// document `file:///…` instead would resolve every `use` into a directory
/// that does not exist, and every directory project would report imports the
/// command line finds. A URI that is not a file on this machine keeps its
/// string: an unsaved buffer has no directory, and `use` inside it finding
/// nothing is true.
fn document_name(uri: &Uri) -> String {
    file_path(uri).unwrap_or_else(|| uri.as_str().to_owned())
}

/// The filesystem path of a `file:` URI naming a file on this machine,
/// percent-decoded. Another scheme, or an authority that is not this
/// machine, is not a file here.
fn file_path(uri: &Uri) -> Option<String> {
    if uri.scheme()?.as_str() != "file" {
        return None;
    }
    if let Some(authority) = uri.authority() {
        let host = authority.as_str();
        if !host.is_empty() && host != "localhost" {
            return None;
        }
    }
    let decoded = fluent_uri::enc::EStr::new(uri.path().as_str())
        .decode()
        .into_string_lossy()
        .into_owned();
    // Windows writes the drive as a leading segment: `/C:/…`.
    let path = match decoded.strip_prefix('/') {
        Some(rest) if matches!(rest.as_bytes(), [drive, b':', ..] if drive.is_ascii_alphabetic()) => rest.to_owned(),
        _ => decoded,
    };
    Some(path)
}

/// The documents the client has open, keyed by URI string.
///
/// Keyed by the URI's own text rather than the `Uri` value: a URI is an
/// identity, not a structure the server inspects, and the string is the
/// identity.
pub(crate) struct Workspace {
    documents: HashMap<String, Document>,
}

impl Workspace {
    /// An empty workspace, as at server start.
    pub(crate) fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Open a document, replacing any session the URI held — a client that
    /// re-opens a URI means the text it just sent, not the old one.
    pub(crate) fn open(&mut self, uri: &Uri, source: String, version: i32) {
        self.documents
            .insert(uri.as_str().to_owned(), Document::new(uri, source, version));
    }

    /// Replace a document's whole text (the sync kind this server declares).
    ///
    /// A change to a URI that was never opened is a client bug; it is opened
    /// rather than dropped, because the text the client sent is complete and
    /// the alternative is a server that pretends not to have heard.
    pub(crate) fn change(&mut self, uri: &Uri, source: String, version: i32) {
        if let Some(document) = self.documents.get_mut(uri.as_str()) {
            // `SetSource` cannot fail; the session is the type that says so.
            if document
                .session
                .apply(ProjectCommand::SetSource(source.clone()))
                .is_ok()
            {
                document.lines = LineIndex::new(&source);
                document.version = version;
            }
        } else {
            self.open(uri, source, version);
        }
    }

    /// Close a document and drop its session.
    pub(crate) fn close(&mut self, uri: &Uri) {
        self.documents.remove(uri.as_str());
    }

    /// The document behind `uri`, when it is open.
    pub(crate) fn document(&self, uri: &Uri) -> Option<&Document> {
        self.documents.get(uri.as_str())
    }
}

#[cfg(test)]
mod tests {
    // Test helpers panic on statically-valid inputs: a failure is a bug in
    // the test itself, and panicking is the correct behavior there.
    #![allow(clippy::panic)]
    #![allow(clippy::expect_used)]

    use std::str::FromStr as _;

    use super::*;

    #[test]
    fn a_file_uri_names_the_file() {
        let uri = Uri::from_str("file:///Users/musa/pieces/opening.musa").expect("uri");
        assert_eq!(document_name(&uri), "/Users/musa/pieces/opening.musa");
    }

    #[test]
    fn a_file_uri_decodes_its_path() {
        let uri = Uri::from_str("file:///Users/musa/my%20piece.musa").expect("uri");
        assert_eq!(document_name(&uri), "/Users/musa/my piece.musa");
    }

    #[test]
    fn a_windows_file_uri_drops_the_slash_before_the_drive() {
        let uri = Uri::from_str("file:///C:/Users/musa/piece.musa").expect("uri");
        assert_eq!(document_name(&uri), "C:/Users/musa/piece.musa");
    }

    #[test]
    fn a_uri_that_is_not_a_file_here_keeps_its_string() {
        let untitled = Uri::from_str("untitled:Untitled-1").expect("uri");
        assert_eq!(document_name(&untitled), "untitled:Untitled-1");
        let remote = Uri::from_str("file://server/share/piece.musa").expect("uri");
        assert_eq!(document_name(&remote), "file://server/share/piece.musa");
    }
}
