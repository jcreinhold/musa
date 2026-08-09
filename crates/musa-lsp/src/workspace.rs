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
        // The URI is the only name the server knows; the session uses it in
        // diagnostics' display, where a path a person opened is the right
        // thing to show.
        let session = ProjectSession::from_text(source, uri.as_str().to_owned());
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

    /// The client's version counter at the last change.
    pub(crate) fn version(&self) -> i32 {
        self.version
    }
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
