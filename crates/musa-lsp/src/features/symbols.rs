//! Document symbols: the piece's structure as the outline pane knows it.
//!
//! A section and a phrase are the two things a composer navigates by
//! (roadmap §8.2's annotations read as navigation), so they are the
//! symbols. Parts and voices carry no source spans in the facts, and a symbol
//! without a location is a guess — they are left out rather than invented.

use lsp_types::{DocumentSymbolResponse, Location, SymbolInformation, SymbolKind, Uri};
use musa_project::OutlineKind;

use crate::workspace::Document;

/// The outline of the document's last valid compile.
pub(crate) fn symbols(document: &Document, uri: &Uri) -> Option<DocumentSymbolResponse> {
    let snapshot = document.snapshot();
    let score = snapshot.score()?;
    let lines = document.lines();
    let symbols = score
        .outline
        .iter()
        .map(|entry| {
            let kind = match entry.kind {
                OutlineKind::Section => SymbolKind::NAMESPACE,
                OutlineKind::Phrase => SymbolKind::FUNCTION,
            };
            #[allow(deprecated)] // `deprecated` is a field the protocol still carries.
            SymbolInformation {
                name: entry.name.clone(),
                kind,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range: lines.range(entry.span),
                },
                container_name: None,
            }
        })
        .collect();
    Some(DocumentSymbolResponse::Flat(symbols))
}
