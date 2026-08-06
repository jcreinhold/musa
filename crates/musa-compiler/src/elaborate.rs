//! Elaboration of the surface language through the temporal kernel
//! (docs/kernel/06, course correction §19–20, §30 Step 4).
//!
//! This is the second semantic path: the same CST, the same declaration
//! table, motif registration, and unit checks as the direct lowerer (shared
//! helpers from `lower.rs`), but voice content elaborates into
//! `Timeline<VoicePayload>` values built from kernel `sequence`/`overlay`,
//! then adapts back into the existing `ScoreSnapshot` (§27). The direct
//! lowerer remains the regression oracle until prompt 12.
//!
//! Design decisions recorded in docs/kernel/06 and 08:
//! - a `rest` statement elaborates to a `Rest` payload occurrence — notation
//!   intent, a typed fact; the kernel has no silence object (§2);
//! - transposition applies eagerly during elaboration via the shared
//!   interval stack (§19 evaluation strategy); semantically it is a payload
//!   map (§13) and composition is commutative, so eager application is
//!   observably equal (prompt 06's composition law);
//! - voice/part identity rides in the payload (§32 Q3 working stance);
//! - provenance (`Origin`) rides in the payload, above the kernel (§20).
//!
//! In the score adapter the kernel's unit is the whole note (1 = semibreve),
//! matching `MusicalTime`.
//!
//! Rational arithmetic here is exact and total for musa's magnitudes (see
//! `time.rs`); the workspace arithmetic lint is allowed at module scope.
#![allow(clippy::arithmetic_side_effects)]

use crate::compile::{Compilation, SourceDocument};
use crate::lower::{self, ExpandCx, Lowering};
use crate::origin::{ExpansionStep, Origin, SourceSpan};
use crate::pitch::WrittenPitch;
use crate::score::{Clef, NotatedDuration, Part, PartId, ScoreEvent, ScoreEventKind, ScoreSnapshot, Voice, VoiceId};
use crate::time::{MusicalDuration, MusicalTime};
use musa_kernel::{Beat, Occurrence, Span, Timeline, overlay, sequence, timeline};
use musa_language::ast::{AstNode as _, PieceDecl, VoiceItem};
use musa_language::{SyntaxKind, SyntaxNode};

/// The elaborated fact of one voice item: notation intent plus provenance,
/// opaque to the kernel (docs/kernel/06).
#[derive(Clone, Debug)]
pub(crate) struct VoicePayload {
    part: u32,
    voice: u32,
    kind: PayloadKind,
    origin: Origin,
    spelling: String,
}

/// What the occurrence states: a sounding note, or a notated rest (typed
/// intent — never a silence object).
#[derive(Clone, Debug)]
enum PayloadKind {
    Note { pitch: WrittenPitch },
    Rest,
}

impl VoicePayload {
    fn note(part: u32, voice: u32, pitch: WrittenPitch, origin: Origin, spelling: String) -> Self {
        Self {
            part,
            voice,
            kind: PayloadKind::Note { pitch },
            origin,
            spelling,
        }
    }

    fn rest(part: u32, voice: u32, origin: Origin, spelling: String) -> Self {
        Self {
            part,
            voice,
            kind: PayloadKind::Rest,
            origin,
            spelling,
        }
    }
}

impl musa_kernel::Canonical for VoicePayload {
    /// Deterministic, injective key for canonical ordering and semantic
    /// equality (docs/kernel/05 N3): identity, kind, and full provenance.
    fn canonical_key(&self) -> String {
        let kind = match &self.kind {
            PayloadKind::Note { pitch } => format!("note:{pitch}"),
            PayloadKind::Rest => "rest".to_string(),
        };
        format!(
            "{}|{}|{}|{}|{}|{}|{:?}",
            self.part,
            self.voice,
            kind,
            self.spelling,
            self.origin.source_span.start,
            self.origin.source_span.end,
            self.origin.expansion_path
        )
    }
}

/// Elaborate `source` through the temporal kernel and adapt the result into
/// a `ScoreSnapshot` (docs/kernel/06, prompt 11).
pub(crate) fn elaborate(source: &SourceDocument) -> Compilation {
    let document = musa_language::parse(source.text());
    let mut lowering = Lowering::new();
    for error in document.errors() {
        let range = error.range();
        lowering.error(
            format!("syntax: {}", error.message()),
            SourceSpan::new(u32::from(range.start()), u32::from(range.end())),
        );
    }
    if lowering
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, lowering.diagnostics);
    }
    let Some(piece) = PieceDecl::from_root(&document.syntax()) else {
        lowering.error("no `piece` declaration", SourceSpan::new(0, 0));
        return Compilation::new(None, lowering.diagnostics);
    };

    let mut snapshot = ScoreSnapshot::default();
    lower::lower_header(&mut lowering, &piece, &mut snapshot);
    if let Some(score) = piece.score() {
        elaborate_score(&mut lowering, &score, &mut snapshot);
    }
    lower::check_measure_sanity(&mut lowering, &snapshot);
    if lowering
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, lowering.diagnostics);
    }
    Compilation::new(Some(snapshot), lowering.diagnostics)
}

/// Walk parts and voices exactly as the direct lowerer does, but elaborate
/// each voice into a kernel timeline and adapt it back to events.
fn elaborate_score(lowering: &mut Lowering, score: &musa_language::ast::ScoreDecl, snapshot: &mut ScoreSnapshot) {
    for part in score.parts() {
        let name = part.name().unwrap_or_default();
        let part_key = lowering.declare(crate::lower::DeclInfo::Part);
        let _ = lower::ordinal(lowering, part_key);
        if snapshot.parts.iter().any(|(_, existing)| existing.name == name) {
            lowering.error(format!("duplicate part `{name}`"), lower::span_of(part.syntax()));
            continue;
        }
        let id = PartId(lowering.next_part);
        lowering.next_part = lowering.next_part.saturating_add(1);

        let mut clef = None;
        for node in part.syntax().children() {
            if node.kind() != SyntaxKind::ClefStmt {
                continue;
            }
            match lower::token_text(&node, SyntaxKind::Identifier).and_then(|text| Clef::parse(&text)) {
                Some(parsed) => clef = Some(parsed),
                None => lowering.error("unknown clef", lower::span_of(&node)),
            }
        }

        let mut voices = indexmap::IndexMap::new();
        let mut voice_names = indexmap::IndexMap::new();
        for (index, voice) in part.voices().iter().enumerate() {
            let voice_name = voice.name().unwrap_or_default();
            let voice_key = lowering.declare(crate::lower::DeclInfo::Voice);
            let declaration = lower::ordinal(lowering, voice_key);
            if voice_names.values().any(|existing| *existing == voice_name) {
                lowering.error(
                    format!("duplicate voice `{voice_name}` in part `{name}`"),
                    lower::span_of(voice.syntax()),
                );
                continue;
            }
            let voice_id = VoiceId(u32::try_from(index).unwrap_or(u32::MAX));
            let timeline = elaborate_voice(lowering, voice, declaration, id.0, voice_id.0);
            let adapted = adapt_voice(lowering, &timeline);
            voices.insert(voice_id, adapted);
            voice_names.insert(voice_id, voice_name);
        }

        snapshot.parts.insert(
            id,
            Part {
                id,
                name,
                clef,
                voices,
                voice_names,
            },
        );
    }
}

/// Elaborate one voice: `sequence` of its items (docs/kernel/06).
fn elaborate_voice(
    lowering: &mut Lowering,
    voice: &musa_language::ast::VoiceDecl,
    declaration: crate::origin::DeclarationId,
    part: u32,
    voice_id: u32,
) -> Timeline<VoicePayload> {
    let cx = ExpandCx {
        params: indexmap::IndexMap::new(),
        intervals: Vec::new(),
        path: Vec::new(),
        declaration,
        origin_span: None,
        max_motif: usize::MAX,
    };
    elaborate_items(lowering, &voice.items(), &cx, part, voice_id)
}

/// Elaborate voice items into a kernel timeline (sequence of item segments).
fn elaborate_items(
    lowering: &mut Lowering,
    items: &[VoiceItem],
    cx: &ExpandCx,
    part: u32,
    voice: u32,
) -> Timeline<VoicePayload> {
    let mut segments = Vec::new();
    for item in items {
        segments.push(elaborate_item(lowering, item, cx, part, voice));
    }
    sequence(segments)
}

/// Elaborate one item; malformed items elaborate to the empty segment
/// `(0, ∅)` — the direct lowerer's `continue` (diagnostic already emitted).
fn elaborate_item(
    lowering: &mut Lowering,
    item: &VoiceItem,
    cx: &ExpandCx,
    part: u32,
    voice: u32,
) -> Timeline<VoicePayload> {
    match item {
        VoiceItem::Note(note) => {
            let Some(duration) = lower::resolve_duration(lowering, note.syntax(), cx) else {
                return empty_segment();
            };
            let pitch_text = note.pitch().unwrap_or_default();
            let Some(pitch) = lower::resolve_pitch(lowering, &pitch_text, note.syntax(), cx) else {
                return empty_segment();
            };
            let origin = origin_of(cx, lower::trimmed_span(note.syntax()));
            single(
                &duration,
                VoicePayload::note(part, voice, pitch, origin, duration.spelling.clone()),
            )
        }
        VoiceItem::Rest(rest) => {
            let Some(duration) = lower::resolve_duration(lowering, rest.syntax(), cx) else {
                return empty_segment();
            };
            let origin = origin_of(cx, lower::trimmed_span(rest.syntax()));
            single(
                &duration,
                VoicePayload::rest(part, voice, origin, duration.spelling.clone()),
            )
        }
        VoiceItem::Chord(chord) => {
            let Some(duration) = lower::resolve_duration(lowering, chord.syntax(), cx) else {
                return empty_segment();
            };
            let mut pitches = Vec::new();
            for text in chord.pitches() {
                match WrittenPitch::parse(&text).map(|pitch| apply_intervals(lowering, pitch, cx, chord.syntax())) {
                    Some(Some(pitch)) => {
                        let origin = origin_of(cx, lower::trimmed_span(chord.syntax()));
                        pitches.push(VoicePayload::note(
                            part,
                            voice,
                            pitch,
                            origin,
                            duration.spelling.clone(),
                        ));
                    }
                    Some(None) => break,
                    None => {
                        lowering.error(format!("invalid chord pitch `{text}`"), lower::span_of(chord.syntax()));
                    }
                }
            }
            let span = span_of_duration(&duration);
            timeline(
                span.end(),
                pitches
                    .into_iter()
                    .map(|payload| Occurrence::new(span, payload))
                    .collect(),
            )
            .unwrap_or_else(|_| musa_kernel::zero())
        }
        VoiceItem::Use(call) => elaborate_use(lowering, call, cx, part, voice),
        VoiceItem::Transpose(transpose) => {
            let text = transpose.interval().unwrap_or_default();
            let Some(interval) = crate::origin::Interval::parse(&text, transpose.is_down()) else {
                lowering.error(format!("unknown interval `{text}`"), lower::span_of(transpose.syntax()));
                return empty_segment();
            };
            let mut inner = cx.clone();
            inner.intervals.push(interval);
            inner.path.push(ExpansionStep::Transposition(interval));
            elaborate_items(lowering, &transpose.items(), &inner, part, voice)
        }
        VoiceItem::Repeat(repeat) => {
            let count: u32 = repeat.count().and_then(|text| text.parse().ok()).unwrap_or(0);
            let mut segments = Vec::new();
            for iteration in 0..count {
                let mut inner = cx.clone();
                inner.path.push(ExpansionStep::RepeatIteration(iteration));
                segments.push(elaborate_items(lowering, &repeat.items(), &inner, part, voice));
            }
            sequence(segments)
        }
    }
}

/// Expand a `use` statement, mirroring the direct lowerer's binding rules.
fn elaborate_use(
    lowering: &mut Lowering,
    call: &musa_language::ast::UseStmt,
    cx: &ExpandCx,
    part: u32,
    voice: u32,
) -> Timeline<VoicePayload> {
    let name = call.motif().unwrap_or_default();
    let found = lowering
        .motifs
        .get_full(&name)
        .map(|(index, _, motif)| (index, motif.params.clone(), motif.body.clone(), motif.declaration));
    let Some((index, motif_params, body, declaration)) = found else {
        lowering.error(format!("unknown motif `{name}`"), lower::span_of(call.syntax()));
        return empty_segment();
    };
    if index >= cx.max_motif {
        lowering.error(
            format!("motif `{name}` can only reference motifs declared before it"),
            lower::span_of(call.syntax()),
        );
        return empty_segment();
    }
    let args = call.args();
    if args.len() > motif_params.len() {
        lowering.error(
            format!(
                "motif `{name}` takes {} arguments, got {}",
                motif_params.len(),
                args.len()
            ),
            lower::span_of(call.syntax()),
        );
        return empty_segment();
    }
    let mut params = indexmap::IndexMap::new();
    for (position, param) in motif_params.iter().enumerate() {
        let text = args.get(position).cloned().or_else(|| param.default.clone());
        let Some(text) = text else {
            lowering.error(
                format!("motif `{name}`: missing argument `{}`", param.name),
                lower::span_of(call.syntax()),
            );
            return empty_segment();
        };
        let Some(value) = lower::bind_argument(lowering, &name, param, &text, cx, call.syntax()) else {
            return empty_segment();
        };
        params.insert(param.name.clone(), value);
    }
    let call_span = lower::trimmed_span(call.syntax());
    let inner = ExpandCx {
        params,
        intervals: cx.intervals.clone(),
        path: cx
            .path
            .iter()
            .cloned()
            .chain(std::iter::once(ExpansionStep::MotifApplication {
                call_site: call_span,
            }))
            .collect(),
        declaration,
        origin_span: Some(call_span),
        max_motif: index,
    };
    elaborate_items(lowering, &body, &inner, part, voice)
}

/// The empty segment `(0, ∅)` — contributes nothing to the sequence.
fn empty_segment() -> Timeline<VoicePayload> {
    musa_kernel::zero()
}

/// The span `[0, d)` for a notated duration. Non-negative by construction
/// (durations parse from positive literals); `Span::ZERO` is the dead
/// fallback.
fn span_of_duration(duration: &NotatedDuration) -> Span {
    Span::new(Beat::ZERO, Beat::new(duration.value.as_ratio())).unwrap_or(Span::ZERO)
}

/// A segment holding one occurrence over `[0, d)`. Bounds hold by
/// construction; the empty segment is the dead fallback.
fn single(duration: &NotatedDuration, payload: VoicePayload) -> Timeline<VoicePayload> {
    let span = span_of_duration(duration);
    timeline(span.end(), vec![Occurrence::new(span, payload)]).unwrap_or_else(|_| musa_kernel::zero())
}

/// The origin for an event under this expansion context (mirrors the direct
/// lowerer: motif applications point at the call site).
fn origin_of(cx: &ExpandCx, span: SourceSpan) -> Origin {
    Origin {
        source_span: cx.origin_span.unwrap_or(span),
        declaration: cx.declaration,
        expansion_path: cx.path.clone(),
    }
}

/// Apply the transposition stack (shared semantics with the direct lowerer).
fn apply_intervals(
    lowering: &mut Lowering,
    pitch: WrittenPitch,
    cx: &ExpandCx,
    node: &SyntaxNode,
) -> Option<WrittenPitch> {
    let mut current = pitch;
    for interval in &cx.intervals {
        let Some(next) = current.transpose(*interval) else {
            lowering.error(
                format!("transposition of `{current}` needs more than a double accidental"),
                lower::span_of(node),
            );
            return None;
        };
        current = next;
    }
    Some(current)
}

/// Adapt a voice's kernel timeline back into snapshot events
/// (docs/kernel/06 adapter contract): per-pitch occurrences sharing span,
/// voice, and origin regroup into chords; a `Rest` payload becomes a rest
/// event. Event ids are assigned in traversal order, matching the oracle.
fn adapt_voice(lowering: &mut Lowering, timeline: &Timeline<VoicePayload>) -> Voice {
    let mut events = Vec::new();
    let mut index = 0;
    let occurrences = timeline.occurrences();
    while index < occurrences.len() {
        let Some(first) = occurrences.get(index) else { break };
        let span = first.span();
        let payload = first.payload();
        let onset = MusicalTime::new(span.start().as_ratio());
        let duration = NotatedDuration {
            value: MusicalDuration::new(span.duration()),
            spelling: payload.spelling.clone(),
        };
        match &payload.kind {
            PayloadKind::Rest => {
                events.push(ScoreEvent {
                    id: lowering.event_id(),
                    origin: payload.origin.clone(),
                    onset,
                    notated_duration: duration,
                    kind: ScoreEventKind::Rest,
                });
                index = index.saturating_add(1);
            }
            PayloadKind::Note { pitch } => {
                let mut pitches = vec![*pitch];
                let mut consumed = 1;
                while let Some(next) = occurrences.get(index.saturating_add(consumed)) {
                    let same_group = next.span() == span
                        && next.payload().origin == payload.origin
                        && matches!(next.payload().kind, PayloadKind::Note { .. });
                    if !same_group {
                        break;
                    }
                    if let PayloadKind::Note { pitch } = &next.payload().kind {
                        pitches.push(*pitch);
                    }
                    consumed = consumed.saturating_add(1);
                }
                let kind = if pitches.len() == 1 {
                    ScoreEventKind::Note {
                        pitch: pitches.first().copied().unwrap_or(*pitch),
                    }
                } else {
                    ScoreEventKind::Chord { pitches }
                };
                events.push(ScoreEvent {
                    id: lowering.event_id(),
                    origin: payload.origin.clone(),
                    onset,
                    notated_duration: duration,
                    kind,
                });
                index = index.saturating_add(consumed);
            }
        }
    }
    Voice { events }
}

/// The normalized kernel text of a source's part timelines, for golden
/// snapshots and semantic hashing (docs/kernel/05 N5–N6). `None` when the
/// source does not elaborate cleanly.
#[doc(hidden)]
pub fn kernel_normal_form(source: &SourceDocument) -> Option<String> {
    let document = musa_language::parse(source.text());
    if !document.errors().is_empty() {
        return None;
    }
    let piece = PieceDecl::from_root(&document.syntax())?;
    let mut lowering = Lowering::new();
    let mut snapshot = ScoreSnapshot::default();
    lower::lower_header(&mut lowering, &piece, &mut snapshot);
    let mut out = String::new();
    for (part_index, part) in piece.score()?.parts().iter().enumerate() {
        let part_id = u32::try_from(part_index).unwrap_or(u32::MAX);
        let mut lanes = Vec::new();
        for (index, voice) in part.voices().iter().enumerate() {
            let voice_key = lowering.declare(crate::lower::DeclInfo::Voice);
            let declaration = lower::ordinal(&lowering, voice_key);
            lanes.push(elaborate_voice(
                &mut lowering,
                voice,
                declaration,
                part_id,
                u32::try_from(index).unwrap_or(u32::MAX),
            ));
        }
        out.push_str(&overlay(lanes).to_string());
    }
    Some(out)
}
