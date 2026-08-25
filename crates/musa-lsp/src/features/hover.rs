//! Hover: the musical facts of the thing under the caret, in markdown.
//!
//! The lookup order is the order of specificity: a `use` site (one statement,
//! one occurrence) before the event it produced (the note is a fact of the
//! score) before the statement inside a motif body that spells a generated
//! note (provenance reading inward); then a keyword's own documentation,
//! because a `use` keyword is its use site and the fact is the better answer;
//! and studio values last because their spans never overlap the score's.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};
use musa_project::{EventFacts, EventKind, EventsTokenClass, ScoreFacts, Span, events_classify, events_keyword_doc};
use musa_syntax::DocumentAlternative;

use crate::convert::{LineIndex, covers};
use crate::workspace::Document;

/// What is true of the thing at `position`, or `None` when no fact reaches it.
pub(crate) fn hover(document: &Document, position: Position) -> Option<Hover> {
    let byte = document.lines().byte(position);
    let snapshot = document.snapshot();
    let lines = document.lines();
    if document.alternative() == DocumentAlternative::Events {
        return at_events_word(&snapshot, byte, lines);
    }
    if let Some(score) = snapshot.score() {
        if let Some(found) = at_use_site(score, byte, lines) {
            return Some(found);
        }
        if let Some(found) = at_event(score, byte, lines) {
            return Some(found);
        }
    }
    if let Some(found) = at_item(&snapshot, byte, lines) {
        return Some(found);
    }
    if let Some(found) = at_studio_catalogue(&snapshot, byte, lines) {
        return Some(found);
    }
    if let Some(found) = at_keyword(&snapshot, byte, lines) {
        return Some(found);
    }
    if let Some(found) = at_vocabulary(&snapshot, byte, lines) {
        return Some(found);
    }
    if let Some(found) = at_builtin(&snapshot, byte, lines) {
        return Some(found);
    }
    at_studio(document, byte, lines)
}

/// A studio processor, parameter, or concept, read from invalid syntax but
/// answered by the same catalogue the compiler checks.
fn at_studio_catalogue(snapshot: &musa_project::ProjectSnapshot<'_>, byte: u32, lines: &LineIndex) -> Option<Hover> {
    let parsed = musa_syntax::parse(snapshot.source());
    let token = parsed.syntax().token_at_offset(byte.into()).find(|token| {
        matches!(
            token.kind(),
            musa_syntax::SyntaxKind::Identifier
                | musa_syntax::SyntaxKind::ScaleKw
                | musa_syntax::SyntaxKind::StudioKw
                | musa_syntax::SyntaxKind::PatchKw
                | musa_syntax::SyntaxKind::ModulateKw
                | musa_syntax::SyntaxKind::BusKw
                | musa_syntax::SyntaxKind::AssignKw
                | musa_syntax::SyntaxKind::SendKw
                | musa_syntax::SyntaxKind::RouteKw
                | musa_syntax::SyntaxKind::MasterKw
                | musa_syntax::SyntaxKind::AtKw
                | musa_syntax::SyntaxKind::OutputKw
                | musa_syntax::SyntaxKind::UnitHz
                | musa_syntax::SyntaxKind::UnitMs
                | musa_syntax::SyntaxKind::UnitS
                | musa_syntax::SyntaxKind::UnitDb
        )
    })?;
    let word = token.text();
    let vocabulary = musa_project::standard_studio_vocabulary().ok()?;
    let markdown = if let Some(doc) = vocabulary.processor(word) {
        processor_markdown(doc)
    } else if let Some(doc) = vocabulary.term(word) {
        term_markdown(doc)
    } else {
        let call = super::call::at(&parsed.syntax(), byte)?;
        let processor = vocabulary.processor(&call.name)?;
        let parameter = processor
            .parameters()
            .iter()
            .find(|parameter| parameter.name() == word)?;
        format!(
            "**{}** — *parameter of {}*\n\n{}\n\nUnit: `{}` · default: `{}` · range: `{}`–`{}`\n\nOrigin: bundled Musa source",
            parameter.name(),
            processor.name(),
            parameter.summary(),
            parameter.default().unit().spelling().unwrap_or("Ratio"),
            musa_project::format_studio_ratio(*parameter.default().magnitude()),
            musa_project::format_studio_ratio(*parameter.minimum().magnitude()),
            musa_project::format_studio_ratio(*parameter.maximum().magnitude()),
        )
    };
    let range = token.text_range();
    Some(answer(
        lines,
        Span {
            start: u32::from(range.start()),
            end: u32::from(range.end()),
        },
        markdown,
    ))
}

pub(crate) fn processor_markdown(doc: &musa_dsp::ProcessorContract) -> String {
    let primitives = doc
        .primitives()
        .iter()
        .map(|primitive| format!("{}@{}", primitive.id(), primitive.version()))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "**{}** — *{}*\n\n{} {}\n\n```musa\n{}\n```\n\n{}\n\nOrigin: bundled Musa source · registered primitives: {}\n\nExample: `{}`",
        doc.name(),
        doc.role().label(),
        doc.summary(),
        doc.note(),
        doc.signature(),
        parameters_markdown(doc.parameters()),
        if primitives.is_empty() {
            "none (source composition)"
        } else {
            &primitives
        },
        doc.example(),
    )
}

pub(crate) fn term_markdown(doc: &musa_dsp::StudioTermContract) -> String {
    format!(
        "**{}** — *studio concept*\n\n{} {}\n\n```musa\n{}\n```\n\nOrigin: bundled Musa source\n\nExample: `{}`",
        doc.spelling(),
        doc.summary(),
        doc.note(),
        doc.signature(),
        doc.example()
    )
}

fn parameters_markdown(params: &[musa_dsp::StudioParameterContract]) -> String {
    if params.is_empty() {
        return "Parameters: none.".to_owned();
    }
    let rows = params
        .iter()
        .map(|param| {
            format!(
                "- `{}` — {}; unit `{}`, default `{}`, range `{}`–`{}`",
                param.name(),
                param.summary(),
                param.default().unit().spelling().unwrap_or("Ratio"),
                musa_project::format_studio_ratio(*param.default().magnitude()),
                musa_project::format_studio_ratio(*param.minimum().magnitude()),
                musa_project::format_studio_ratio(*param.maximum().magnitude())
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("Parameters:\n\n{rows}")
}

/// A declared name — here or in an imported library: its signature, its
/// summary, what its type is not to be confused with, and where it is
/// written.
///
/// The one hover that answers for the whole elaboration language, because the
/// session already knows all of it. Before the caret reaches an event or a
/// keyword there is nothing to prefer: `perfect_fifth` is a name, and what a
/// reader wants to know about a name is what it is.
fn at_item(snapshot: &musa_project::ProjectSnapshot<'_>, byte: u32, lines: &LineIndex) -> Option<Hover> {
    let (item, span) = super::items::at(snapshot, byte)?;
    Some(answer(lines, span, super::items::markdown(item)))
}

fn at_builtin(snapshot: &musa_project::ProjectSnapshot<'_>, byte: u32, lines: &LineIndex) -> Option<Hover> {
    let parsed = musa_syntax::parse(snapshot.source());
    let token = parsed.syntax().token_at_offset(byte.into()).find(|token| {
        token.kind() == musa_syntax::SyntaxKind::Identifier && musa_syntax::builtin_doc(token.text()).is_some()
    })?;
    let doc = musa_syntax::builtin_doc(token.text())?;
    let range = token.text_range();
    Some(answer(
        lines,
        Span {
            start: u32::from(range.start()),
            end: u32::from(range.end()),
        },
        format!("**{}** — *{}*\n\n{}", doc.spelling, doc.summary, doc.doc),
    ))
}

/// A keyword: its own documentation. After the score's facts —
/// a `use` keyword *is* its use site, and the expansion is the better answer
/// there — and before the studio's, whose spans never overlap a keyword.
fn at_keyword(snapshot: &musa_project::ProjectSnapshot<'_>, byte: u32, lines: &LineIndex) -> Option<Hover> {
    let parsed = musa_syntax::parse(snapshot.source());
    let token = parsed
        .syntax()
        .token_at_offset(byte.into())
        .find(|token| musa_syntax::keyword_doc(token.kind()).is_some())?;
    let doc = musa_syntax::keyword_doc(token.kind())?;
    let range = token.text_range();
    let span = Span {
        start: u32::from(range.start()),
        end: u32::from(range.end()),
    };
    Some(answer(
        lines,
        span,
        format!("**{}** — *{}*\n\n{}", doc.spelling, doc.summary, doc.doc),
    ))
}

/// The word after `scale` or `chord` names something from a closed
/// vocabulary, not a free identifier: it is answered from the compiler's own
/// tables, so the hover cannot describe a collection or a chord type the
/// language does not have.
///
/// One word may name both — `major` is a collection and a chord type — and
/// both sentences are shown. Which one the writer meant is settled by the
/// keyword before it, and saying both is more useful than guessing.
fn at_vocabulary(snapshot: &musa_project::ProjectSnapshot<'_>, byte: u32, lines: &LineIndex) -> Option<Hover> {
    let parsed = musa_syntax::parse(snapshot.source());
    let token = parsed
        .syntax()
        .token_at_offset(byte.into())
        .find(|token| token.kind() == musa_syntax::SyntaxKind::Identifier)?;
    let word = token.text();
    let mut said = Vec::new();
    if let Some((name, doc)) = musa_project::scale_collections().find(|(spelling, _)| *spelling == word) {
        said.push(format!(
            "**{name}** — *scale collection*\n\n`scale c {name}` collects {doc}."
        ));
    }
    if let Some((name, doc)) = musa_project::chord_types().find(|(spelling, _)| *spelling == word) {
        said.push(format!("**{name}** — *chord type*\n\n`chord c {name}` stacks {doc}."));
    }
    if said.is_empty() {
        return None;
    }
    let range = token.text_range();
    let span = Span {
        start: u32::from(range.start()),
        end: u32::from(range.end()),
    };
    Some(answer(lines, span, said.join("\n\n---\n\n")))
}

/// Build the hover answer: markdown, ranged at the span that matched, so the
/// client highlights the thing the text is about.
fn answer(lines: &LineIndex, span: Span, value: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(lines.range(span)),
    }
}

/// A `use` statement: the expansion it ran, and where its motif lives.
fn at_use_site(score: &ScoreFacts, byte: u32, lines: &LineIndex) -> Option<Hover> {
    let occurrence = score
        .occurrences
        .iter()
        .find(|occurrence| covers(occurrence.use_site, byte))?;
    let mut text = vec![format!("**{}**", occurrence.label)];
    if let (Some(motif), Some(declaration)) = (&occurrence.motif, occurrence.declaration) {
        let line = lines.position(declaration.start).line.saturating_add(1);
        text.push(format!("motif `{motif}` · declared at line {line}"));
    }
    Some(answer(lines, occurrence.use_site, text.join("\n")))
}

/// A note, rest, or chord — authored where the caret sits, or spelled by the
/// statement under the caret inside a motif body.
fn at_event(score: &ScoreFacts, byte: u32, lines: &LineIndex) -> Option<Hover> {
    let authored = score
        .events
        .iter()
        .find(|event| !event.origin.generated && covers(event.origin.span, byte));
    if let Some(event) = authored {
        return Some(answer(lines, event.origin.span, describe_event(event)));
    }
    let spelled = score
        .events
        .iter()
        .find(|event| event.origin.generated && covers(event.origin.definition_span, byte))?;
    let mut text = describe_event(spelled);
    if !spelled.origin.path.is_empty() {
        text.push_str("\ngenerated: ");
        let steps: Vec<&str> = spelled.origin.path.iter().map(|step| step.label.as_str()).collect();
        text.push_str(&steps.join(" ▸ "));
    }
    Some(answer(lines, spelled.origin.definition_span, text))
}

/// The event as two lines: what it sounds like, and where it stands.
///
/// ```text
/// **G♯4, B4** — 1/4
/// bar 12 · beat 3/2 · A minor · treble
/// ```
fn describe_event(event: &EventFacts) -> String {
    let what = match event.kind {
        EventKind::Rest => "*rest*".to_owned(),
        EventKind::Note | EventKind::Chord => format!("**{}**", event.pitches.join(", ")),
    };
    let mut place = format!("bar {} · beat {}", event.bar, super::fraction(&event.beat));
    if let Some(key) = &event.key {
        place.push_str(" · ");
        place.push_str(key);
    }
    if let Some(clef) = &event.clef {
        place.push_str(" · ");
        place.push_str(clef);
    }
    format!("{what} — {}\n{place}", event.duration_spelling)
}

/// A written studio value: a parameter's number, or a send's level.
fn at_studio(document: &Document, byte: u32, lines: &LineIndex) -> Option<Hover> {
    let snapshot = document.snapshot();
    let studio = snapshot.studio()?;
    for container in studio.patches.iter().chain(&studio.buses).chain(&studio.signals) {
        for stage in &container.stages {
            for parameter in &stage.params {
                if let Some(span) = parameter.span.filter(|span| covers(*span, byte)) {
                    let mut text = if parameter.unit.is_empty() {
                        format!("**{}** — {}", parameter.name, parameter.value)
                    } else {
                        format!("**{}** — {} {}", parameter.name, parameter.value, parameter.unit)
                    };
                    if let Some(signal) = &parameter.modulated_by {
                        text.push_str("\nmodulated by `");
                        text.push_str(signal);
                        text.push('`');
                    }
                    return Some(answer(lines, span, text));
                }
            }
        }
    }
    let (send, span) = studio
        .sends
        .iter()
        .find_map(|send| send.span.filter(|span| covers(*span, byte)).map(|span| (send, span)))?;
    let text = format!("**send** {} → {} · {} dB", send.source, send.bus, send.decibels);
    Some(answer(lines, span, text))
}

/// What an event track construct denotes, under the caret.
///
/// Only the grammar's own words answer here, and the score's facts answer not
/// at all — an event track document's occurrences carry provenance into the file
/// that produced them, so "the note at this offset" is a question about a
/// different document. What is left is the language itself, which is exactly
/// what a reader of unfamiliar interchange text is asking about.
fn at_events_word(snapshot: &musa_project::ProjectSnapshot<'_>, byte: u32, lines: &LineIndex) -> Option<Hover> {
    let source = snapshot.source();
    let at = usize::try_from(byte).unwrap_or(usize::MAX);
    let (range, class) = events_classify(source)
        .into_iter()
        .find(|(range, _)| range.contains(&at))?;
    if !matches!(class, EventsTokenClass::Keyword | EventsTokenClass::Type) {
        return None;
    }
    let word = source.get(range.clone())?;
    let doc = events_keyword_doc(word)?;
    let span = Span {
        start: u32::try_from(range.start).unwrap_or(u32::MAX),
        end: u32::try_from(range.end).unwrap_or(u32::MAX),
    };
    Some(answer(lines, span, format!("```text\n{word}\n```\n\n{doc}")))
}
