//! Diagnostics: the session's restated, teachable diagnostics → LSP's flat list.
//!
//! The mapping keeps everything the client can use: the code (so a composer
//! can ask `musa explain` for the long form), the help folded into the
//! message (LSP has one message slot and the session has two kinds of
//! advice), and the secondary labels as related information (the places that
//! explain the primary one).

use lsp_types::{DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Uri};
use musa_project::{Diagnostic, Severity};

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
    let message = match &diagnostic.help {
        Some(help) => format!("{}\n{help}", diagnostic.message),
        None => diagnostic.message.clone(),
    };
    let related = diagnostic
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
