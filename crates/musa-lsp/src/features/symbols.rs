//! Document symbols: the piece's structure as the outline pane knows it.
//!
//! A section and a phrase are the two things a composer navigates by
//! (roadmap §8.2's annotations read as navigation), so they are the
//! symbols. Parts and voices carry no source spans in the facts, and a symbol
//! without a location is a guess — they are left out rather than invented.

use lsp_types::{DocumentSymbolResponse, Location, SymbolInformation, SymbolKind, SymbolTag, Uri};
use musa_project::{NameKind, OutlineKind, Span, events_bindings};
use musa_syntax::DocumentAlternative;

use crate::workspace::Document;

/// The outline of the document's last valid compile.
pub(crate) fn symbols(document: &Document, uri: &Uri) -> Option<DocumentSymbolResponse> {
    if document.alternative() == DocumentAlternative::Events {
        return events_symbols(document, uri);
    }
    let snapshot = document.snapshot();
    let score = snapshot.score()?;
    let lines = document.lines();
    let mut symbols: Vec<SymbolInformation> = score
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
    // The declarations, beside the music. An outline is what a reader jumps
    // by, and a piece assembled from functions, modules, signatures, and
    // templates is navigated by those as much as by its sections. Only the
    // ones written *here*: an imported declaration has a record whose span
    // indexes another document, and an outline entry pointing into it would
    // send every jump to an offset in the wrong file.
    symbols.extend(snapshot.items().iter().filter(|item| item.uri.is_none()).map(|item| {
        #[allow(deprecated)] // `deprecated` is a field the protocol still carries.
        SymbolInformation {
            name: item.signature.clone(),
            kind: symbol_kind(item.kind),
            tags: item.deprecation.as_ref().map(|_| vec![SymbolTag::DEPRECATED]),
            deprecated: None,
            location: Location {
                uri: uri.clone(),
                range: lines.range(item.span),
            },
            container_name: None,
        }
    }));
    symbols.sort_by_key(|symbol| (symbol.location.range.start.line, symbol.location.range.start.character));
    Some(DocumentSymbolResponse::Flat(symbols))
}

/// The outline of an event track document: the material its `let`s name.
///
/// Read off the *text*, not off the compiled score, and that is the whole
/// point. An event track document's facts carry the source spans of the piece that
/// produced them — offsets into a `.musa` file that may not even be on this
/// machine — so an outline built from the score would send every jump to a
/// position in the wrong document. The bindings are in this file, at these
/// offsets, and they are what a reader navigates a term by.
fn events_symbols(document: &Document, uri: &Uri) -> Option<DocumentSymbolResponse> {
    let snapshot = document.snapshot();
    let lines = document.lines();
    let symbols = events_bindings(snapshot.source())
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

/// How the protocol names each kind of declaration.
///
/// A motif and a fragment are `SymbolKind::FUNCTION` because that is what a
/// reader does with them — they are called — and a value is a `CONSTANT`
/// because Musa has no other kind of binding.
fn symbol_kind(kind: NameKind) -> SymbolKind {
    match kind {
        NameKind::Value => SymbolKind::CONSTANT,
        NameKind::Function | NameKind::Motif | NameKind::Fragment => SymbolKind::FUNCTION,
        NameKind::Bar => SymbolKind::EVENT,
        NameKind::Part | NameKind::Module => SymbolKind::NAMESPACE,
        NameKind::Voice => SymbolKind::VARIABLE,
        NameKind::Patch => SymbolKind::CLASS,
        NameKind::Template => SymbolKind::INTERFACE,
    }
}
