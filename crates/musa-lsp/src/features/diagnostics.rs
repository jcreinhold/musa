//! Diagnostics: the session's restated, teachable diagnostics → LSP's flat list.
//!
//! The mapping keeps everything the client can use: the code (so a composer
//! can ask `musa explain` for the long form), the help folded into the
//! message (LSP has one message slot and the session has two kinds of
//! advice), and the secondary labels as related information (the places that
//! explain the primary one).
//!
//! A *cause* — a fault in an adapter module this piece imports — becomes
//! related information too, at that module's own URI, which is the whole
//! reason `Location` carries one. A key with no file behind it folds into the
//! message instead of being dropped: a composer who cannot click through still
//! has to be told what the module's checker said.

use lsp_types::{DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Range, Uri};
use musa_project::{Cause, CauseLabel, Diagnostic, Severity};

use crate::convert::LineIndex;
use crate::workspace::Document;

/// Every diagnostic of the document's current source, in LSP shape.
pub(crate) fn all(document: &Document, uri: &Uri) -> Vec<lsp_types::Diagnostic> {
    document
        .snapshot()
        .diagnostics()
        .iter()
        .map(|diagnostic| to_lsp(diagnostic, uri, document.lines()))
        .collect()
}

/// One diagnostic, in LSP shape.
///
/// The range is the primary span's; a diagnostic aimed at the whole file
/// points at its start, because "somewhere" is where a squiggle has to be.
pub(crate) fn to_lsp(diagnostic: &Diagnostic, uri: &Uri, lines: &LineIndex) -> lsp_types::Diagnostic {
    let range = diagnostic.span.map(|span| lines.range(span)).unwrap_or_default();
    let mut message = match &diagnostic.help {
        Some(help) => format!("{}\n{help}", diagnostic.message),
        None => diagnostic.message.clone(),
    };
    let mut related = diagnostic
        .labels
        .iter()
        .filter(|label| !label.primary)
        .map(|label| DiagnosticRelatedInformation {
            location: Location {
                uri: uri.clone(),
                range: lines.range(label.span),
            },
            message: label.text.clone(),
        })
        .collect::<Vec<_>>();
    for cause in &diagnostic.causes {
        // Nothing to click through to, so the words go where they will at
        // least be read. A composer who cannot open the module still has to be
        // told what its checker said.
        let Some(document) = file_uri(&cause.document) else {
            message.push('\n');
            message.push_str(&folded(cause));
            continue;
        };
        let places: Vec<(Range, String)> = if cause.labels.is_empty() {
            // A fault about the module as a whole — it does not parse, it is
            // not a `library` — has no place inside it. The file's start is
            // where an editor opens it, which is the nearest true answer.
            vec![(Range::default(), folded(cause))]
        } else {
            cause
                .labels
                .iter()
                .map(|label| (span_of(label), related_message(cause, label)))
                .collect()
        };
        related.extend(places.into_iter().map(|(range, message)| DiagnosticRelatedInformation {
            location: Location {
                uri: document.clone(),
                range,
            },
            message,
        }));
    }
    lsp_types::Diagnostic {
        range,
        severity: Some(match diagnostic.severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
        }),
        code: Some(NumberOrString::String(diagnostic.code.clone())),
        code_description: None,
        source: Some("musa".to_owned()),
        message,
        related_information: if related.is_empty() { None } else { Some(related) },
        tags: None,
        data: None,
    }
}

/// A cause's document as a URI an editor can open, when it is a real file.
///
/// An import resolves to a filesystem path or to a bundled module's virtual
/// URI (`musa-import:/std/staff.musa`). Only the first is somewhere the client
/// can go, and a `Location` pointing at the second would be a link that opens
/// nothing.
fn file_uri(document: &str) -> Option<Uri> {
    if !document.starts_with('/') {
        return None;
    }
    format!("file://{document}").parse().ok()
}

/// One cause label's range in its own document.
///
/// A label whose positions are absent — a document this compilation was not
/// handed — points at the file's start rather than being dropped, for the same
/// reason a cause with no labels does.
fn span_of(label: &CauseLabel) -> Range {
    let point = |position: Option<musa_project::Position>| {
        position
            .map(|position| lsp_types::Position {
                line: position.line.saturating_sub(1),
                character: position.column.saturating_sub(1),
            })
            .unwrap_or_default()
    };
    Range {
        start: point(label.at),
        end: point(label.to),
    }
}

/// What one related-information entry says: the cause's own small document,
/// with this label's words where the label stands.
///
/// The same folding the primary already does with its help, for the same
/// reason: LSP has one message slot per entry and a cause has four kinds of
/// thing to say.
fn related_message(cause: &Cause, label: &CauseLabel) -> String {
    advised(format!("{}: {}", label.text, cause.message), cause)
}

/// A whole cause as prose, for the places a range cannot carry it.
fn folded(cause: &Cause) -> String {
    advised(format!("{}: {}", cause.document, cause.message), cause)
}

fn advised(mut text: String, cause: &Cause) -> String {
    for line in [cause.note.as_deref(), cause.help.as_deref()].into_iter().flatten() {
        text.push('\n');
        text.push_str(line);
    }
    text
}
