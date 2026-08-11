//! Document symbols: the piece's structure as the outline pane knows it.
//!
//! A section and a phrase are the two things a composer navigates by
//! (roadmap §8.2's annotations read as navigation), so they are the
//! symbols. Parts and voices carry no source spans in the facts, and a symbol
//! without a location is a guess — they are left out rather than invented.

use lsp_types::{DocumentSymbolResponse, Location, SymbolInformation, SymbolKind, Uri};
use musa_language::DocumentAlternative;
use musa_project::{OutlineKind, Span, kernel_bindings};

use crate::workspace::Document;

/// The outline of the document's last valid compile.
pub(crate) fn symbols(document: &Document, uri: &Uri) -> Option<DocumentSymbolResponse> {
    if document.alternative() == DocumentAlternative::Kernel {
        return kernel_symbols(document, uri);
    }
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

/// The outline of a kernel document: the material its `let`s name.
///
/// Read off the *text*, not off the compiled score, and that is the whole
/// point. A kernel document's facts carry the source spans of the piece that
/// produced them — offsets into a `.musa` file that may not even be on this
/// machine — so an outline built from the score would send every jump to a
/// position in the wrong document. The bindings are in this file, at these
/// offsets, and they are what a reader navigates a term by.
fn kernel_symbols(document: &Document, uri: &Uri) -> Option<DocumentSymbolResponse> {
    let snapshot = document.snapshot();
    let lines = document.lines();
    let symbols = kernel_bindings(snapshot.source())
        .into_iter()
        .map(|(range, name)| {
            let span = Span {
                start: u32::try_from(range.start).unwrap_or(u32::MAX),
                end: u32::try_from(range.end).unwrap_or(u32::MAX),
            };
            #[allow(deprecated)] // `deprecated` is a field the protocol still carries.
            SymbolInformation {
                name,
                kind: SymbolKind::CONSTANT,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range: lines.range(span),
                },
                container_name: None,
            }
        })
        .collect();
    Some(DocumentSymbolResponse::Flat(symbols))
}
