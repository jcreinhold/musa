//! Completion: the language's vocabulary, plus the names the composer gave.
//!
//! Two sources, and each is the honest one for its kind. Keywords and unit
//! suffixes come from `SPELLINGS` — the list the lexer itself is checked
//! against, so the server can neither offer a word the lexer does not know
//! nor miss one it does. A keyword carries its own documentation (prompt 84):
//! the summary is the menu's detail line and the whole doc is the item's
//! markdown, so the menu itself teaches. Names come from the last valid
//! compile's facts: motifs, parts, voices, and the studio's containers —
//! offered with the kind of thing they are, so the menu reads as music
//! rather than as text. The words that name a scale collection are neither:
//! they are identifiers the lexer cannot tell from any other, so they come
//! from the compiler's own collection table, which is the only place that
//! knows which collections exist.

use std::collections::BTreeMap;

use lsp_types::{CompletionItem, CompletionItemKind, CompletionResponse};
use musa_language::{SPELLINGS, SyntaxKind, TokenClass};

use crate::workspace::Document;

/// The completion menu for the document.
///
/// Not position-aware: what is on offer does not depend on where the caret
/// sits, and the client filters by prefix. Context-aware completion is a
/// later prompt's work, not a guess made here.
pub(crate) fn completions(document: &Document) -> CompletionResponse {
    let mut items: BTreeMap<String, CompletionItem> = BTreeMap::new();
    for (spelling, kind) in SPELLINGS {
        let (item_kind, class) = match TokenClass::of(*kind) {
            Some(TokenClass::Keyword | TokenClass::Use) => (CompletionItemKind::KEYWORD, "keyword"),
            Some(TokenClass::Unit) => (CompletionItemKind::UNIT, "unit"),
            _ => continue,
        };
        items
            .entry((*spelling).to_owned())
            .or_insert_with(|| keyword_item(spelling, *kind, item_kind, class));
    }
    // The words that may follow `scale`. They are identifiers to the lexer,
    // so the vocabulary comes from the compiler's collection table rather
    // than from `SPELLINGS`.
    for (name, doc) in musa_project::scale_collections() {
        items.entry(name.to_owned()).or_insert_with(|| CompletionItem {
            label: name.to_owned(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some("scale collection".to_owned()),
            documentation: Some(lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: format!("**{name}** — *scale collection*\n\n`scale c {name}` collects {doc}."),
            })),
            ..CompletionItem::default()
        });
    }
    if let Some(score) = document.snapshot().score() {
        for part in &score.parts {
            offer(&mut items, &part.name, CompletionItemKind::MODULE, "part");
            for voice in &part.voices {
                offer(&mut items, &voice.name, CompletionItemKind::VARIABLE, "voice");
            }
        }
        for occurrence in &score.occurrences {
            if let Some(motif) = &occurrence.motif {
                offer(&mut items, motif, CompletionItemKind::FUNCTION, "motif");
            }
        }
    }
    if let Some(studio) = document.snapshot().studio() {
        for patch in &studio.patches {
            offer(&mut items, &patch.name, CompletionItemKind::CLASS, "patch");
        }
        for bus in &studio.buses {
            offer(&mut items, &bus.name, CompletionItemKind::MODULE, "bus");
        }
        for signal in &studio.signals {
            offer(&mut items, &signal.name, CompletionItemKind::VARIABLE, "signal");
        }
    }
    CompletionResponse::Array(items.into_values().collect())
}

/// One vocabulary item — a keyword with its own documentation when it has
/// one (prompt 84), a unit with only its class.
fn keyword_item(
    spelling: &str,
    kind: SyntaxKind,
    item_kind: CompletionItemKind,
    class: &'static str,
) -> CompletionItem {
    let base = CompletionItem {
        label: spelling.to_owned(),
        kind: Some(item_kind),
        ..CompletionItem::default()
    };
    let Some(doc) = musa_language::keyword_doc(kind) else {
        return CompletionItem {
            detail: Some(class.to_owned()),
            ..base
        };
    };
    CompletionItem {
        detail: Some(doc.summary.to_owned()),
        documentation: Some(lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: format!("**{}** — *{}*\n\n{}", doc.spelling, doc.summary, doc.doc),
        })),
        ..base
    }
}

/// Offer a name, keeping the first kind a label was offered with — a motif
/// and a part that share a name collide in the menu, and either description
/// alone is more honest than two rows that look identical.
fn offer(items: &mut BTreeMap<String, CompletionItem>, name: &str, kind: CompletionItemKind, detail: &'static str) {
    items.entry(name.to_owned()).or_insert_with(|| CompletionItem {
        label: name.to_owned(),
        kind: Some(kind),
        detail: Some(detail.to_owned()),
        ..CompletionItem::default()
    });
}
