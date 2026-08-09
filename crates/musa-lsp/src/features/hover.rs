//! Hover: the musical facts of the thing under the caret, in markdown.
//!
//! The lookup order is the order of specificity: a `use` site (one statement,
//! one occurrence) before the event it produced (the note is a fact of the
//! score) before the statement inside a motif body that spells a generated
//! note (provenance reading inward), and studio values last because their
//! spans never overlap the score's.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};
use musa_project::{EventFacts, EventKind, ScoreFacts, Span};

use crate::convert::{LineIndex, covers};
use crate::workspace::Document;

/// What is true of the thing at `position`, or `None` when no fact reaches it.
pub(crate) fn hover(document: &Document, position: Position) -> Option<Hover> {
    let byte = document.lines().byte(position);
    let snapshot = document.snapshot();
    let lines = document.lines();
    if let Some(score) = snapshot.score() {
        if let Some(found) = at_use_site(score, byte, lines) {
            return Some(found);
        }
        if let Some(found) = at_event(score, byte, lines) {
            return Some(found);
        }
    }
    at_studio(document, byte, lines)
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
        text.push_str(&spelled.origin.path.join(" ▸ "));
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
