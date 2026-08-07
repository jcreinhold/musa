//! The direct resolver pass: CST → resolution + unit checks → `ScoreSnapshot`.
//! Private to the crate (roadmap §10.6: pass types never cross the
//! boundary).
//!
//! **Frozen oracle (prompt 12):** the canonical semantic path is
//! `elaborate.rs` through the temporal kernel (course correction §30 Step 6).
//! This pass stays compiled in and runnable — it is the differential
//! regression oracle (tests/elaboration.rs). It is deleted at prompt 41,
//! once the kernel migration it guarded is finished; the vocabulary it shares
//! with elaboration now lives in `resolve.rs`.
//!
//! Time accumulation here uses the `MusicalTime`/`MusicalDuration` operators,
//! which are total for musa's magnitudes (see `time.rs`); the workspace
//! arithmetic lint is allowed at module scope for that reason.
#![allow(clippy::arithmetic_side_effects)]

use indexmap::IndexMap;
use musa_language::SyntaxNode;
use musa_language::ast::{AstNode as _, PieceDecl, VoiceItem};
use num_rational::Ratio;

use crate::compile::{Compilation, SourceDocument};
use crate::origin::{DeclarationId, ExpansionStep, Interval, Origin, SourceSpan};
use crate::pitch::WrittenPitch;
use crate::resolve::{
    DeclInfo, ExpandCx, Resolver, apply_intervals, bind_argument, check_measure_sanity, lower_header, lower_studio,
    ordinal, part_metadata, resolve_duration, resolve_pitch, span_of, trimmed_span,
};
use crate::score::{NotatedDuration, Part, PartId, ScoreEvent, ScoreEventKind, ScoreSnapshot, Voice, VoiceId};
use crate::time::MusicalTime;

/// Lower a document into a compilation.
pub(crate) fn lower(source: &SourceDocument) -> Compilation {
    let document = musa_language::parse(source.text());
    let mut resolver = Resolver::new();
    for error in document.errors() {
        let range = error.range();
        resolver.error(
            format!("syntax: {}", error.message()),
            SourceSpan::new(u32::from(range.start()), u32::from(range.end())),
        );
    }
    if resolver
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, resolver.diagnostics);
    }
    let Some(piece) = PieceDecl::from_root(&document.syntax()) else {
        resolver.error("no `piece` declaration", SourceSpan::new(0, 0));
        return Compilation::new(None, resolver.diagnostics);
    };

    let mut snapshot = ScoreSnapshot::default();
    lower_header(&mut resolver, &piece, &mut snapshot);
    for import in piece.imports() {
        reject(&mut resolver, "use", import.syntax());
    }
    for tempo in piece.tempos().iter().filter(|tempo| tempo.position().is_some()) {
        reject(&mut resolver, "tempo change", tempo.syntax());
    }
    if let Some(score) = piece.score() {
        lower_score(&mut resolver, &score, &mut snapshot);
    }
    check_measure_sanity(&mut resolver, &snapshot);
    if resolver
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, resolver.diagnostics);
    }
    let studio = lower_studio(&mut resolver, &piece, &snapshot, &[]);
    if resolver
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, resolver.diagnostics);
    }
    Compilation::new(Some(snapshot), resolver.diagnostics).with_studio(studio)
}

fn lower_score(resolver: &mut Resolver, score: &musa_language::ast::ScoreDecl, snapshot: &mut ScoreSnapshot) {
    // The oracle is frozen at phase 1 (prompt 11): every construct added after
    // it says so rather than compiling to something quietly smaller.
    for section in score.sections() {
        reject(resolver, "section", section.syntax());
    }
    for lane in score.harmonies() {
        reject(resolver, "harmony", lane.syntax());
    }
    for part in score.parts() {
        let name = part.name().unwrap_or_default();
        let part_key = resolver.declare(DeclInfo::Part);
        let _ = ordinal(resolver, part_key);
        if snapshot.parts.iter().any(|(_, existing)| existing.name == name) {
            resolver.error(format!("duplicate part `{name}`"), span_of(part.syntax()));
            continue;
        }
        let id = PartId(resolver.next_part);
        resolver.next_part = resolver.next_part.saturating_add(1);

        let (clef, profile) = part_metadata(resolver, &part, &snapshot.profiles);
        if let Some(profile) = profile {
            snapshot.profiles.assign(&name, profile);
        }

        let mut voices = IndexMap::new();
        let mut voice_names = IndexMap::new();
        for (index, voice) in part.voices().iter().enumerate() {
            let voice_name = voice.name().unwrap_or_default();
            let voice_key = resolver.declare(DeclInfo::Voice);
            let declaration = ordinal(resolver, voice_key);
            if voice_names.values().any(|existing| *existing == voice_name) {
                resolver.error(
                    format!("duplicate voice `{voice_name}` in part `{name}`"),
                    span_of(voice.syntax()),
                );
                continue;
            }
            let voice_id = VoiceId(u32::try_from(index).unwrap_or(u32::MAX));
            let lowered = lower_voice(resolver, voice, declaration);
            voices.insert(voice_id, lowered);
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

/// Lower one voice: sequential accumulation of onsets (roadmap §5.2's span
/// law by construction).
/// Lower one voice from a root expansion context: sequential accumulation
/// of onsets (roadmap §5.2's span law by construction).
fn lower_voice(resolver: &mut Resolver, voice: &musa_language::ast::VoiceDecl, declaration: DeclarationId) -> Voice {
    let mut events = Vec::new();
    let mut onset = MusicalTime::ZERO;
    let cx = ExpandCx {
        params: IndexMap::new(),
        intervals: Vec::new(),
        path: Vec::new(),
        declaration,
        origin_span: None,
        max_motif: usize::MAX,
        scale: Ratio::ONE,
    };
    lower_items(resolver, &voice.items(), &cx, &mut events, &mut onset);
    Voice { events }
}

/// Lower a sequence of voice items under an expansion context.
fn lower_items(
    resolver: &mut Resolver,
    items: &[VoiceItem],
    cx: &ExpandCx,
    events: &mut Vec<ScoreEvent>,
    onset: &mut MusicalTime,
) {
    for item in items {
        match item {
            VoiceItem::Note(note) => {
                let Some(duration) = resolve_duration(resolver, note.syntax(), cx) else {
                    continue;
                };
                let pitch_text = note.pitch().unwrap_or_default();
                let Some(pitch) = resolve_pitch(resolver, &pitch_text, note.syntax(), cx) else {
                    continue;
                };
                push_event(
                    resolver,
                    events,
                    onset,
                    cx,
                    trimmed_span(note.syntax()),
                    duration,
                    ScoreEventKind::Note { pitch },
                );
            }
            VoiceItem::Rest(rest) => {
                let Some(duration) = resolve_duration(resolver, rest.syntax(), cx) else {
                    continue;
                };
                push_event(
                    resolver,
                    events,
                    onset,
                    cx,
                    trimmed_span(rest.syntax()),
                    duration,
                    ScoreEventKind::Rest,
                );
            }
            VoiceItem::Chord(chord) => {
                let Some(duration) = resolve_duration(resolver, chord.syntax(), cx) else {
                    continue;
                };
                let mut pitches = Vec::new();
                for text in chord.pitches() {
                    match WrittenPitch::parse(&text).map(|pitch| apply_intervals(resolver, pitch, cx, chord.syntax())) {
                        Some(Some(pitch)) => pitches.push(pitch),
                        Some(None) => break,
                        None => {
                            resolver.error(format!("invalid chord pitch `{text}`"), span_of(chord.syntax()));
                        }
                    }
                }
                push_event(
                    resolver,
                    events,
                    onset,
                    cx,
                    trimmed_span(chord.syntax()),
                    duration,
                    ScoreEventKind::Chord { pitches },
                );
            }
            VoiceItem::Use(call) => lower_use(resolver, call, cx, events, onset),
            VoiceItem::Transpose(transpose) => {
                let text = transpose.interval().unwrap_or_default();
                let Some(interval) = Interval::parse(&text, transpose.is_down()) else {
                    resolver.error(format!("unknown interval `{text}`"), span_of(transpose.syntax()));
                    continue;
                };
                let mut inner = cx.clone();
                inner.intervals.push(interval);
                inner.path.push(ExpansionStep::Transposition(interval));
                lower_items(resolver, &transpose.items(), &inner, events, onset);
            }
            VoiceItem::Repeat(repeat) => {
                let count: u32 = repeat.count().and_then(|text| text.parse().ok()).unwrap_or(0);
                for iteration in 0..count {
                    let mut inner = cx.clone();
                    inner.path.push(ExpansionStep::RepeatIteration(iteration));
                    lower_items(resolver, &repeat.items(), &inner, events, onset);
                }
            }
            // The frozen oracle predates phase 2 and does not grow with it
            // (module docs above). Saying so is the honest answer: silently
            // dropping a tuplet would make this pass disagree with the
            // canonical one about how long a bar is.
            VoiceItem::Slur(item) => reject(resolver, "slur", item.syntax()),
            VoiceItem::Dynamic(item) => reject(resolver, "dynamic", item.syntax()),
            VoiceItem::Tuplet(item) => reject(resolver, "tuplet", item.syntax()),
            VoiceItem::Stretch(item) => reject(resolver, "stretch", item.syntax()),
            VoiceItem::Retrograde(item) => reject(resolver, "retrograde", item.syntax()),
            VoiceItem::Invert(item) => reject(resolver, "invert", item.syntax()),
            VoiceItem::Phrase(item) => reject(resolver, "phrase", item.syntax()),
            VoiceItem::Hairpin(item) => reject(resolver, "crescendo", item.syntax()),
        }
    }
}

/// Refuse a construct this pass was frozen before.
fn reject(resolver: &mut Resolver, what: &str, node: &SyntaxNode) {
    resolver.error(
        format!("`{what}` needs the kernel elaboration path"),
        trimmed_span(node),
    );
}

/// Append an event and advance the onset.
fn push_event(
    resolver: &mut Resolver,
    events: &mut Vec<ScoreEvent>,
    onset: &mut MusicalTime,
    cx: &ExpandCx,
    span: SourceSpan,
    duration: NotatedDuration,
    kind: ScoreEventKind,
) {
    let origin = Origin {
        source_span: cx.origin_span.unwrap_or(span),
        definition_span: span,
        declaration: cx.declaration,
        expansion_path: cx.path.clone(),
    };
    let value = duration.value;
    events.push(ScoreEvent {
        id: resolver.event_id(),
        origin,
        onset: *onset,
        notated_duration: duration,
        kind,
    });
    *onset = *onset + value;
}

/// Expand a `use` statement: bind arguments, then lower the motif body.
fn lower_use(
    resolver: &mut Resolver,
    call: &musa_language::ast::UseStmt,
    cx: &ExpandCx,
    events: &mut Vec<ScoreEvent>,
    onset: &mut MusicalTime,
) {
    let name = call.motif().unwrap_or_default();
    let found = resolver
        .motifs
        .get_full(&name)
        .map(|(index, _, motif)| (index, motif.params.clone(), motif.body.clone(), motif.declaration));
    let Some((index, motif_params, body, declaration)) = found else {
        resolver.error(format!("unknown motif `{name}`"), span_of(call.syntax()));
        return;
    };
    if index >= cx.max_motif {
        resolver.error(
            format!("motif `{name}` can only reference motifs declared before it"),
            span_of(call.syntax()),
        );
        return;
    }
    let args = call.args();
    if args.len() > motif_params.len() {
        resolver.error(
            format!(
                "motif `{name}` takes {} arguments, got {}",
                motif_params.len(),
                args.len()
            ),
            span_of(call.syntax()),
        );
        return;
    }
    let mut params = IndexMap::new();
    for (position, param) in motif_params.iter().enumerate() {
        let text = args.get(position).cloned().or_else(|| param.default.clone());
        let Some(text) = text else {
            resolver.error(
                format!("motif `{name}`: missing argument `{}`", param.name),
                span_of(call.syntax()),
            );
            return;
        };
        let Some(value) = bind_argument(resolver, &name, param, &text, cx, call.syntax()) else {
            return;
        };
        params.insert(param.name.clone(), value);
    }
    let call_span = trimmed_span(call.syntax());
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
        scale: cx.scale,
    };
    lower_items(resolver, &body, &inner, events, onset);
}
