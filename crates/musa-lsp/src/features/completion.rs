//! Completion: the language's vocabulary, plus the names the composer gave.
//!
//! Two sources, and each is the honest one for its kind. Keywords and unit
//! suffixes come from `SPELLINGS` — the list the lexer itself is checked
//! against, so the server can neither offer a word the lexer does not know
//! nor miss one it does. A keyword carries its own documentation:
//! the summary is the menu's detail line and the whole doc is the item's
//! markdown, so the menu itself teaches. Names come from the last valid
//! compile's facts: motifs, parts, voices, and the studio's containers —
//! offered with the kind of thing they are, so the menu reads as music
//! rather than as text. The words that name a scale collection are neither:
//! they are identifiers the lexer cannot tell from any other, so they come
//! from the compiler's own collection table, which is the only place that
//! knows which collections exist. Type names are the same case for the same
//! reason — a type is spelled with a capital, which makes it an identifier —
//! so they come from `BASE_TYPES`.

use std::collections::BTreeMap;

use lsp_types::{CompletionItem, CompletionItemKind, CompletionItemTag, CompletionResponse, Position};
use musa_project::NameKind;
use musa_syntax::{BASE_TYPES, SPELLINGS, SyntaxKind, TokenClass};

use crate::workspace::Document;

/// The completion menu for the caret's position.
///
/// Two layers. The document's whole vocabulary is always on offer, because
/// the client filters by prefix and a menu that guessed wrong would hide the
/// word the writer is typing. On top of it sits whatever the *site* knows:
/// the parameter names of the call being written, the three realization
/// policies where a policy is the argument, the voice-leading rule ids where
/// a rule is, and the music-typed names an event track hole may splice. Site items
/// sort first, and nothing is taken away.
pub(crate) fn completions(document: &Document, position: Position) -> CompletionResponse {
    let mut items: BTreeMap<String, CompletionItem> = BTreeMap::new();
    at_site(document, position, &mut items);
    for doc in musa_project::standard_studio_vocabulary()
        .into_iter()
        .flat_map(musa_dsp::StudioVocabulary::processors)
    {
        items.entry(doc.name().to_owned()).or_insert_with(|| CompletionItem {
            label: doc.name().to_owned(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(doc.signature().to_owned()),
            documentation: Some(lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: super::hover::processor_markdown(doc),
            })),
            ..CompletionItem::default()
        });
    }
    for instrument in musa_project::standard_instrument_contracts()
        .into_iter()
        .flat_map(musa_dsp::InstrumentContracts::declarations)
    {
        for control in instrument.controls() {
            let name = control.name().to_owned();
            items.entry(name.clone()).or_insert_with(|| CompletionItem {
                label: name,
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(format!("{} control · {}", control.kind(), control.update_rate())),
                documentation: Some(lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                    kind: lsp_types::MarkupKind::Markdown,
                    value: super::hover::control_markdown(control),
                })),
                ..CompletionItem::default()
            });
        }
    }
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
    // The type names. A type is spelled with a capital and is therefore an
    // identifier, so it is not in `SPELLINGS` — the language's own type
    // vocabulary is, and it is the same list the compiler reads a type from.
    for (name, doc) in BASE_TYPES {
        items.entry((*name).to_owned()).or_insert_with(|| CompletionItem {
            label: (*name).to_owned(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some("type".to_owned()),
            documentation: Some(lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: format!("**{name}** — *type*\n\n{doc}."),
            })),
            ..CompletionItem::default()
        });
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
    // The same, for the words that may follow `chord` and `stack`.
    for (name, doc) in musa_project::chord_types() {
        items.entry(name.to_owned()).or_insert_with(|| CompletionItem {
            label: name.to_owned(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some("chord type".to_owned()),
            documentation: Some(lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: format!("**{name}** — *chord type*\n\n`chord c {name}` stacks {doc}."),
            })),
            ..CompletionItem::default()
        });
    }
    // Every declaration the last valid compile knows: values, functions,
    // motifs, and the modules and templates a piece is assembled from, each
    // offered with the signature the checker settled on rather than with a
    // guess about what it is.
    let snapshot = document.snapshot();
    for item in snapshot.items() {
        let kind = match item.kind {
            NameKind::Value => CompletionItemKind::CONSTANT,
            NameKind::Function => CompletionItemKind::FUNCTION,
            NameKind::Motif | NameKind::Fragment | NameKind::Bar => CompletionItemKind::SNIPPET,
            NameKind::Part => CompletionItemKind::MODULE,
            NameKind::Voice => CompletionItemKind::VARIABLE,
            NameKind::Patch => CompletionItemKind::CLASS,
            NameKind::Module | NameKind::Template => CompletionItemKind::MODULE,
        };
        items.entry(item.name.clone()).or_insert_with(|| CompletionItem {
            label: item.name.clone(),
            kind: Some(kind),
            detail: Some(item.signature.clone()),
            documentation: Some(lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: super::items::markdown(&snapshot, item),
            })),
            tags: item.deprecation.as_ref().map(|_| vec![CompletionItemTag::DEPRECATED]),
            ..CompletionItem::default()
        });
    }
    let snapshot = document.snapshot();
    if let Some(score) = snapshot.score() {
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
    if let Some(studio) = snapshot.studio() {
        for patch in &studio.patches {
            offer(&mut items, &patch.name, CompletionItemKind::CLASS, "patch");
        }
        for bus in &studio.buses {
            offer(&mut items, &bus.name, CompletionItemKind::MODULE, "bus");
        }
        for signal in &studio.signals {
            offer(&mut items, &signal.name, CompletionItemKind::VARIABLE, "signal");
        }
        for assignment in &studio.assignments {
            offer(
                &mut items,
                &assignment.instrument,
                CompletionItemKind::CLASS,
                if assignment.explicit {
                    "source instrument"
                } else {
                    "inherited edition instrument"
                },
            );
            offer(
                &mut items,
                &assignment.profile,
                CompletionItemKind::VALUE,
                if assignment.explicit {
                    "performance profile"
                } else {
                    "inherited edition profile"
                },
            );
        }
        for media in &studio.media {
            offer(&mut items, &media.name, CompletionItemKind::FILE, &media.kind);
        }
    }
    CompletionResponse::Array(items.into_values().collect())
}

/// What the caret's own position knows, offered ahead of the vocabulary.
///
/// The site is read from the lossless tree, because completion is asked for
/// while the text does not compile; what each site *offers* comes from the
/// session's facts and the compiler's own registries, because the shell knows
/// no music. Sorting: `0` puts these above every general word without
/// removing any of them.
fn at_site(document: &Document, position: Position, items: &mut BTreeMap<String, CompletionItem>) {
    let byte = document.lines().byte(position);
    let snapshot = document.snapshot();
    let parsed = musa_syntax::parse(snapshot.source());
    if in_events_hole(&parsed.syntax(), byte) {
        // A hole splices music and nothing else, so the names that fit are
        // exactly the ones whose declared result is `EventTrack<WrittenTime>`.
        for item in snapshot.items().iter().filter(|item| {
            item.result
                .as_ref()
                .is_some_and(|result| result.name == "EventTrack(WrittenTime)")
        }) {
            site(items, &item.name, CompletionItemKind::VALUE, item.signature.clone());
        }
        return;
    }
    let Some(call) = super::call::at(&parsed.syntax(), byte) else {
        return;
    };
    if let Some(processor) = musa_project::standard_studio_vocabulary()
        .ok()
        .and_then(|vocabulary| vocabulary.processor(&call.name))
    {
        for parameter in processor.parameters() {
            site(
                items,
                &format!("{}:", parameter.name()),
                CompletionItemKind::FIELD,
                parameter.summary().to_owned(),
            );
        }
        return;
    }
    // A claim's arguments are words the registry reads, not values: where one
    // is expected, the words themselves are the vocabulary.
    if let Some(claim) = musa_project::assertion_claims().find(|claim| claim.name == call.name) {
        match claim.parameters.get(call.argument).copied() {
            Some("exactly|may_omit|may_add") => {
                for (policy, asks) in musa_project::realization_policies() {
                    site(items, policy, CompletionItemKind::ENUM_MEMBER, asks.to_owned());
                }
            }
            Some("rule id") => {
                for rule in musa_project::rule_names() {
                    site(
                        items,
                        rule.id(),
                        CompletionItemKind::ENUM_MEMBER,
                        format!("{} — {}", rule.states(), rule.cites()),
                    );
                }
            }
            _ => {}
        }
        return;
    }
    // An ordinary call: the parameters it has left to be given, offered as
    // the named arguments they are written as. Every one of them will be
    // written, since a call supplies them all.
    let Some(item) = super::items::named(&snapshot, &call.name) else {
        return;
    };
    for parameter in &item.parameters {
        site(
            items,
            &format!("{}:", parameter.name),
            CompletionItemKind::FIELD,
            parameter.ty.name.clone(),
        );
    }
}

/// Whether the caret sits inside a `${ … }` events hole.
fn in_events_hole(tree: &musa_syntax::SyntaxNode, byte: u32) -> bool {
    tree.descendants().any(|node| {
        node.kind() == SyntaxKind::EventsHole && {
            let range = node.text_range();
            byte > u32::from(range.start()) && byte <= u32::from(range.end())
        }
    })
}

/// One item the site itself supplied, sorted above the general vocabulary.
fn site(items: &mut BTreeMap<String, CompletionItem>, label: &str, kind: CompletionItemKind, detail: String) {
    items.insert(
        label.to_owned(),
        CompletionItem {
            label: label.to_owned(),
            kind: Some(kind),
            detail: Some(detail),
            sort_text: Some(format!("0{label}")),
            ..CompletionItem::default()
        },
    );
}

/// One vocabulary item — a keyword with its own documentation when it has
/// one, a unit with only its class.
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
    if let Some(doc) = musa_project::standard_studio_vocabulary()
        .ok()
        .and_then(|vocabulary| vocabulary.term(spelling))
    {
        return CompletionItem {
            detail: Some(doc.signature().to_owned()),
            documentation: Some(lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: super::hover::term_markdown(doc),
            })),
            ..base
        };
    }
    let Some(doc) = musa_syntax::keyword_doc(kind) else {
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
fn offer(items: &mut BTreeMap<String, CompletionItem>, name: &str, kind: CompletionItemKind, detail: &str) {
    items.entry(name.to_owned()).or_insert_with(|| CompletionItem {
        label: name.to_owned(),
        kind: Some(kind),
        detail: Some(detail.to_owned()),
        ..CompletionItem::default()
    });
}
