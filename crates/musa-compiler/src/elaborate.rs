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
use crate::lower::{self, ExpandCx, GroupInfo, GroupKind, Lowering};
use crate::origin::{ExpansionStep, Origin, SourceSpan};
use crate::pitch::WrittenPitch;
use crate::score::{
    ArticulationMark, ArticulationMarking, DynamicMark, DynamicMarking, HairpinSpan, HarmonyMark, NotatedDuration,
    Part, PartId, PhraseSpan, ScoreEvent, ScoreEventKind, ScoreSnapshot, SectionMark, SlurSpan, TempoChange,
    TupletSpan, Voice, VoiceId,
};
use crate::time::MusicalTime;
use musa_kernel::{Beat, Occurrence, Span, Timeline, overlay, sequence, timeline};
use musa_language::SyntaxNode;
use musa_language::ast::{AstNode as _, PieceDecl, VoiceItem};
use num_rational::Ratio;

/// What is written *about* an occurrence rather than in it: the marks that
/// become annotations once the events they belong to have identities
/// (roadmap §6.3).
#[derive(Clone, Debug, Default)]
struct Marks {
    /// This event is tied to the one that follows it.
    tie: bool,
    /// Articulations written on the event, in source order.
    articulations: Vec<ArticulationMark>,
    /// A dynamic marking that takes effect at this event.
    dynamic: Option<(DynamicMark, Origin)>,
    /// The slur and tuplet blocks enclosing it, outermost first.
    groups: Vec<u32>,
}

impl Marks {
    fn is_empty(&self) -> bool {
        !self.tie && self.articulations.is_empty() && self.dynamic.is_none() && self.groups.is_empty()
    }

    /// A deterministic, injective rendering for the canonical key. Empty
    /// marks contribute nothing, so a piece without them keeps the key it
    /// had before phase 2.
    fn canonical_key(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let articulations: Vec<&str> = self.articulations.iter().map(|mark| mark.name()).collect();
        let groups: Vec<String> = self.groups.iter().map(u32::to_string).collect();
        format!(
            "|tie:{}|artic:{}|dyn:{}|groups:{}",
            self.tie,
            articulations.join(","),
            self.dynamic.as_ref().map_or("-", |(mark, _)| mark.name()),
            groups.join(","),
        )
    }
}

/// A dynamic marking waiting for the event it applies from.
struct PendingDynamic {
    mark: DynamicMark,
    origin: Origin,
    span: SourceSpan,
}

/// The elaborated fact of one voice item: notation intent plus provenance,
/// opaque to the kernel (docs/kernel/06).
#[derive(Clone, Debug)]
pub(crate) struct VoicePayload {
    part: u32,
    voice: u32,
    kind: PayloadKind,
    origin: Origin,
    /// The written duration, kept whole: the kernel span says how long the
    /// occurrence is, but only this says how many noteheads spell it.
    duration: NotatedDuration,
    marks: Marks,
}

/// What the occurrence states: a sounding note, or a notated rest (typed
/// intent — never a silence object).
#[derive(Clone, Debug)]
enum PayloadKind {
    Note { pitch: WrittenPitch },
    Rest,
}

impl VoicePayload {
    fn note(
        part: u32,
        voice: u32,
        pitch: WrittenPitch,
        origin: Origin,
        duration: NotatedDuration,
        marks: Marks,
    ) -> Self {
        Self {
            part,
            voice,
            kind: PayloadKind::Note { pitch },
            origin,
            duration,
            marks,
        }
    }

    /// The same fact sounding and notated `factor` times as long.
    fn stretched(&self, factor: Ratio<i64>) -> Self {
        let mut stretched = self.clone();
        stretched.duration = self.duration.stretched(factor);
        stretched
    }

    /// The same fact with its pitch mirrored about `axis`, or `None` when
    /// the mirror image is not spellable (roadmap §5.4's meaningful failure).
    fn inverted(&self, axis: WrittenPitch) -> Option<Self> {
        let PayloadKind::Note { pitch } = self.kind else {
            return Some(self.clone());
        };
        let mut inverted = self.clone();
        inverted.kind = PayloadKind::Note {
            pitch: pitch.invert(axis)?,
        };
        Some(inverted)
    }

    fn rest(part: u32, voice: u32, origin: Origin, duration: NotatedDuration, marks: Marks) -> Self {
        Self {
            part,
            voice,
            kind: PayloadKind::Rest,
            origin,
            duration,
            marks,
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
            "{}|{}|{}|{}|{}|{}|{:?}{}",
            self.part,
            self.voice,
            kind,
            self.duration.spelling,
            self.origin.source_span.start,
            self.origin.source_span.end,
            self.origin.expansion_path,
            self.marks.canonical_key(),
        )
    }
}

/// Elaborate `source` through the temporal kernel and adapt the result into
/// a `ScoreSnapshot` (docs/kernel/06, prompt 11).
pub(crate) fn elaborate(source: &SourceDocument, options: &crate::CompileOptions) -> Compilation {
    let document = musa_language::parse(source.text());
    let mut lowering = Lowering::new();
    elaborate_parsed(&document, source.name(), options, &mut lowering)
}

/// One voice's elaborated timeline, before the snapshot adapter sees it.
pub(crate) type VoiceTimeline = Timeline<VoicePayload>;

/// Everything after parsing (docs/kernel/06): elaborate, adapt, check.
///
/// Split out of [`elaborate`] so the parse and the semantic work can be
/// measured apart; `lowering` arrives from the caller for the same reason
/// (see `crate::bench`). The production path passes a fresh one.
pub(crate) fn elaborate_parsed(
    document: &musa_language::ParsedDocument,
    name: &str,
    options: &crate::CompileOptions,
    lowering: &mut Lowering,
) -> Compilation {
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
        return Compilation::new(None, std::mem::take(&mut lowering.diagnostics));
    }
    let Some(piece) = PieceDecl::from_root(&document.syntax()) else {
        lowering.error("no `piece` declaration", SourceSpan::new(0, 0));
        return Compilation::new(None, std::mem::take(&mut lowering.diagnostics));
    };

    let mut snapshot = ScoreSnapshot::default();
    let libraries = crate::imports::load(lowering, name, &piece, &options.imports);
    elaborate_libraries(lowering, &libraries, &mut snapshot);
    lower::lower_header(lowering, &piece, &mut snapshot);
    if let Some(score) = piece.score() {
        elaborate_score(lowering, &score, &mut snapshot);
        elaborate_annotations(lowering, &score, &snapshot);
        elaborate_tempo_changes(lowering, &piece, &mut snapshot);
    }
    snapshot.annotations = std::mem::take(&mut lowering.annotations);
    lower::check_measure_sanity(lowering, &snapshot);
    check_tuplets(lowering, &snapshot);
    if lowering
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, std::mem::take(&mut lowering.diagnostics));
    }
    let imported_studios: Vec<musa_language::ast::StudioDecl> =
        libraries.each().filter_map(|(_, library)| library.studio()).collect();
    let studio = lower::lower_studio(lowering, &piece, &snapshot, &imported_studios);
    if lowering
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, std::mem::take(&mut lowering.diagnostics));
    }
    Compilation::new(Some(snapshot), std::mem::take(&mut lowering.diagnostics)).with_studio(studio)
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

        let (clef, profile) = lower::part_metadata(lowering, &part, &snapshot.profiles);
        if let Some(profile) = profile {
            snapshot.profiles.assign(&name, profile);
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
            if let Some(sink) = &mut lowering.timeline_sink {
                sink.push(timeline);
            }
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

/// Record the score's form markers and chord symbols (roadmap §8.2).
///
/// These are the annotations written at a *position* rather than on a note,
/// so they are resolved here, after the parts are elaborated and the piece
/// has a length to be inside of. Nothing interprets them: a chord symbol is
/// parsed so a later library can read it, and that is the end of the core's
/// involvement.
fn elaborate_annotations(lowering: &mut Lowering, score: &musa_language::ast::ScoreDecl, snapshot: &ScoreSnapshot) {
    let declaration = crate::origin::DeclarationId::default();
    let extent = piece_extent(snapshot);
    for section in score.sections() {
        let span = lower::trimmed_span(section.syntax());
        let Some(at) = resolve_position(lowering, section.position().as_ref(), span, snapshot, extent) else {
            continue;
        };
        lowering.annotations.push_section(SectionMark {
            name: section.name().unwrap_or_default(),
            at,
            origin: Origin {
                source_span: span,
                definition_span: span,
                declaration,
                expansion_path: Vec::new(),
            },
        });
    }
    let lanes = score.harmonies();
    for extra in lanes.iter().skip(1) {
        lowering.error(
            "one `harmony` lane per score; write every chord in the first one",
            lower::trimmed_span(extra.syntax()),
        );
    }
    for chord in lanes.iter().flat_map(musa_language::ast::HarmonyDecl::chords) {
        let span = lower::trimmed_span(chord.syntax());
        let Some(at) = resolve_position(lowering, chord.position().as_ref(), span, snapshot, extent) else {
            continue;
        };
        let Some(written) = chord.symbol() else {
            continue;
        };
        if !written.is_one_word() {
            lowering.error("a chord symbol is one word, such as `am` or `fmaj7`", span);
            continue;
        }
        let text = written.text();
        let Some(symbol) = crate::harmony::ChordSymbol::parse(&text) else {
            lowering.error(format!("`{text}` is not a chord symbol musa can read"), span);
            continue;
        };
        lowering.annotations.push_harmony(HarmonyMark {
            symbol,
            at,
            origin: Origin {
                source_span: span,
                definition_span: span,
                declaration,
                expansion_path: Vec::new(),
            },
        });
    }
}

/// Register everything the imported libraries declare, before the piece's
/// own declarations, so a collision is reported against the library that
/// caused it (roadmap §16).
fn elaborate_libraries(lowering: &mut Lowering, libraries: &crate::imports::Libraries, snapshot: &mut ScoreSnapshot) {
    for (path, library) in libraries.each() {
        if let Some(performance) = library.performance() {
            let profiles = lower::parse_profiles(lowering, &performance);
            lower::merge_profiles(
                lowering,
                snapshot,
                &profiles,
                lower::span_of(performance.syntax()),
                Some(path),
            );
        }
        lower::register_motifs(lowering, snapshot, &library.motifs(), Some(path));
    }
}

/// Resolve the piece's tempo changes against the meter (roadmap §6.3).
///
/// A tempo change is written where a form marker is written — `at 9:1` — and
/// for the same reason: a tempo belongs to a place in the piece, not to a
/// note. Positions are resolved after the parts exist so that a tempo nobody
/// ever reaches is an error rather than a silent segment.
fn elaborate_tempo_changes(
    lowering: &mut Lowering,
    piece: &musa_language::ast::PieceDecl,
    snapshot: &mut ScoreSnapshot,
) {
    let declaration = crate::origin::DeclarationId::default();
    let extent = piece_extent(snapshot);
    let mut changes: Vec<TempoChange> = Vec::new();
    for tempo in piece.tempos().iter().filter(|tempo| tempo.position().is_some()) {
        let span = lower::trimmed_span(tempo.syntax());
        let Some(at) = resolve_position(lowering, tempo.position().as_ref(), span, snapshot, extent) else {
            continue;
        };
        if at == MusicalTime::ZERO {
            lowering.error("the tempo at `1:1` is the piece's tempo; write it without `at`", span);
            continue;
        }
        if changes.iter().any(|existing| existing.at == at) {
            lowering.error("two tempos at the same place", span);
            continue;
        }
        let (beat, bpm) = lower::tempo_reading(lowering, tempo);
        changes.push(TempoChange {
            at,
            beat,
            bpm,
            origin: Origin {
                source_span: span,
                definition_span: span,
                declaration,
                expansion_path: Vec::new(),
            },
        });
    }
    changes.sort_by_key(|change| change.at);
    snapshot.tempo_map.changes = changes;
}

/// How long the piece is: where its last event ends.
fn piece_extent(snapshot: &ScoreSnapshot) -> MusicalTime {
    snapshot
        .parts
        .iter()
        .flat_map(|(_, part)| part.voices.values())
        .flat_map(|voice| voice.events.iter())
        .map(|event| event.onset + event.notated_duration.value)
        .max()
        .unwrap_or_default()
}

/// Turn a `measure:beat` coordinate into time, or report why it is not one.
///
/// Measures and beats count from one, the way a composer reads them off the
/// page, and a position past the end of the piece is an error: a chord symbol
/// nobody will ever reach is a mistake, not a comment.
fn resolve_position(
    lowering: &mut Lowering,
    position: Option<&musa_language::ast::Position>,
    span: SourceSpan,
    snapshot: &ScoreSnapshot,
    extent: MusicalTime,
) -> Option<MusicalTime> {
    let position = position?;
    let measure: i64 = position.measure()?.parse().ok()?;
    let beat_text = position.beat()?;
    let beat = lower::parse_ratio(&beat_text).or_else(|| beat_text.parse::<i64>().ok().map(Ratio::from_integer))?;
    if measure < 1 || beat < Ratio::ONE {
        lowering.error("measures and beats count from `1:1`", span);
        return None;
    }
    let measure_len = snapshot.meter_map.measure_len().as_ratio();
    let beat_len = Ratio::new(1, i64::from(snapshot.meter_map.denominator.max(1)));
    let at = MusicalTime::new(measure_len * (measure - 1) + beat_len * (beat - Ratio::ONE));
    if at >= extent && extent > MusicalTime::default() {
        lowering.error(format!("the piece ends before `{measure}:{beat_text}`"), span);
        return None;
    }
    Some(at)
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
        scale: Ratio::ONE,
    };
    let mut pending = None;
    let timeline = elaborate_items(lowering, &voice.items(), &cx, part, voice_id, &[], &mut pending);
    if let Some(pending) = pending {
        lowering.error("this dynamic marking has no note after it", pending.span);
    }
    timeline
}

/// Elaborate voice items into a kernel timeline (sequence of item segments).
///
/// `groups` are the slur and tuplet blocks enclosing these items, outermost
/// first; `pending` carries a dynamic marking forward to the first event
/// written after it, however deeply nested that event turns out to be.
fn elaborate_items(
    lowering: &mut Lowering,
    items: &[VoiceItem],
    cx: &ExpandCx,
    part: u32,
    voice: u32,
    groups: &[u32],
    pending: &mut Option<PendingDynamic>,
) -> Timeline<VoicePayload> {
    let mut segments = Vec::new();
    for item in items {
        segments.push(elaborate_item(lowering, item, cx, part, voice, groups, pending));
    }
    sequence(segments)
}

/// The marks a note or chord statement carries, with the pending dynamic
/// consumed if there is one.
fn marks_for(
    lowering: &mut Lowering,
    names: &[String],
    tied: bool,
    groups: &[u32],
    span: SourceSpan,
    pending: &mut Option<PendingDynamic>,
) -> Marks {
    let mut articulations = Vec::new();
    for name in names {
        match ArticulationMark::parse(name) {
            Some(mark) => articulations.push(mark),
            None => lowering.error(format!("unknown articulation `{name}`"), span),
        }
    }
    Marks {
        tie: tied,
        articulations,
        dynamic: pending.take().map(|pending| (pending.mark, pending.origin)),
        groups: groups.to_vec(),
    }
}

/// Elaborate one item; malformed items elaborate to the empty segment
/// `(0, ∅)` — the direct lowerer's `continue` (diagnostic already emitted).
fn elaborate_item(
    lowering: &mut Lowering,
    item: &VoiceItem,
    cx: &ExpandCx,
    part: u32,
    voice: u32,
    groups: &[u32],
    pending: &mut Option<PendingDynamic>,
) -> Timeline<VoicePayload> {
    match item {
        VoiceItem::Note(note) => {
            let Some(duration) = resolve_scaled_duration(lowering, note.syntax(), cx) else {
                return empty_segment();
            };
            let pitch_text = note.pitch().unwrap_or_default();
            let Some(pitch) = lower::resolve_pitch(lowering, &pitch_text, note.syntax(), cx) else {
                return empty_segment();
            };
            let span = lower::trimmed_span(note.syntax());
            let marks = marks_for(lowering, &note.articulations(), note.tied(), groups, span, pending);
            let origin = origin_of(cx, span);
            single(
                &duration,
                VoicePayload::note(part, voice, pitch, origin, duration.clone(), marks),
            )
        }
        VoiceItem::Rest(rest) => {
            let Some(duration) = resolve_scaled_duration(lowering, rest.syntax(), cx) else {
                return empty_segment();
            };
            let span = lower::trimmed_span(rest.syntax());
            let marks = marks_for(lowering, &[], false, groups, span, pending);
            let origin = origin_of(cx, span);
            single(
                &duration,
                VoicePayload::rest(part, voice, origin, duration.clone(), marks),
            )
        }
        VoiceItem::Chord(chord) => {
            let Some(duration) = resolve_scaled_duration(lowering, chord.syntax(), cx) else {
                return empty_segment();
            };
            let node_span = lower::trimmed_span(chord.syntax());
            let marks = marks_for(
                lowering,
                &chord.articulations(),
                chord.tied(),
                groups,
                node_span,
                pending,
            );
            let mut pitches = Vec::new();
            for text in chord.pitches() {
                match WrittenPitch::parse(&text).map(|pitch| apply_intervals(lowering, pitch, cx, chord.syntax())) {
                    Some(Some(pitch)) => {
                        let origin = origin_of(cx, node_span);
                        pitches.push(VoicePayload::note(
                            part,
                            voice,
                            pitch,
                            origin,
                            duration.clone(),
                            marks.clone(),
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
        VoiceItem::Use(call) => elaborate_use(lowering, call, cx, part, voice, groups, pending),
        VoiceItem::Transpose(transpose) => {
            let text = transpose.interval().unwrap_or_default();
            let Some(interval) = crate::origin::Interval::parse(&text, transpose.is_down()) else {
                lowering.error(format!("unknown interval `{text}`"), lower::span_of(transpose.syntax()));
                return empty_segment();
            };
            let mut inner = cx.clone();
            inner.intervals.push(interval);
            inner.path.push(ExpansionStep::Transposition(interval));
            elaborate_items(lowering, &transpose.items(), &inner, part, voice, groups, pending)
        }
        VoiceItem::Repeat(repeat) => {
            let count: u32 = repeat.count().and_then(|text| text.parse().ok()).unwrap_or(0);
            let mut segments = Vec::new();
            for iteration in 0..count {
                let mut inner = cx.clone();
                inner.path.push(ExpansionStep::RepeatIteration(iteration));
                segments.push(elaborate_items(
                    lowering,
                    &repeat.items(),
                    &inner,
                    part,
                    voice,
                    groups,
                    pending,
                ));
            }
            sequence(segments)
        }
        VoiceItem::Slur(slur) => {
            let origin = origin_of(cx, lower::trimmed_span(slur.syntax()));
            let id = lowering.group(GroupInfo {
                kind: GroupKind::Slur,
                origin,
            });
            let inner: Vec<u32> = groups.iter().copied().chain(std::iter::once(id)).collect();
            elaborate_items(lowering, &slur.items(), cx, part, voice, &inner, pending)
        }
        VoiceItem::Phrase(phrase) => {
            let origin = origin_of(cx, lower::trimmed_span(phrase.syntax()));
            let id = lowering.group(GroupInfo {
                kind: GroupKind::Phrase {
                    name: phrase.name().unwrap_or_default(),
                },
                origin,
            });
            let inner: Vec<u32> = groups.iter().copied().chain(std::iter::once(id)).collect();
            elaborate_items(lowering, &phrase.items(), cx, part, voice, &inner, pending)
        }
        VoiceItem::Hairpin(hairpin) => {
            let span = lower::trimmed_span(hairpin.syntax());
            let text = hairpin.target().unwrap_or_default();
            let Some(target) = DynamicMark::parse(&text) else {
                lowering.error(format!("unknown dynamic marking `{text}`"), span);
                return empty_segment();
            };
            let id = lowering.group(GroupInfo {
                kind: GroupKind::Hairpin {
                    grows: hairpin.grows(),
                    target,
                },
                origin: origin_of(cx, span),
            });
            let inner: Vec<u32> = groups.iter().copied().chain(std::iter::once(id)).collect();
            elaborate_items(lowering, &hairpin.items(), cx, part, voice, &inner, pending)
        }
        VoiceItem::Tuplet(tuplet) => {
            let text = tuplet.ratio().unwrap_or_default();
            let span = lower::trimmed_span(tuplet.syntax());
            let Some((num, den)) = parse_tuplet_ratio(&text) else {
                lowering.error(format!("`{text}` is not a tuplet ratio such as `3/2`"), span);
                return empty_segment();
            };
            let origin = origin_of(cx, span);
            let id = lowering.group(GroupInfo {
                kind: GroupKind::Tuplet { num, den },
                origin,
            });
            let inner_groups: Vec<u32> = groups.iter().copied().chain(std::iter::once(id)).collect();
            let mut inner = cx.clone();
            inner.scale = cx.scale * Ratio::new(i64::from(den), i64::from(num));
            elaborate_items(lowering, &tuplet.items(), &inner, part, voice, &inner_groups, pending)
        }
        VoiceItem::Stretch(stretch) => {
            let text = stretch.factor().unwrap_or_default();
            let span = lower::trimmed_span(stretch.syntax());
            let factor = lower::parse_ratio(&text).or_else(|| text.parse::<i64>().ok().map(Ratio::from_integer));
            let Some(factor) = factor.filter(|factor| *factor > Ratio::ZERO) else {
                lowering.error(format!("`{text}` is not a positive stretch factor such as `3/2`"), span);
                return empty_segment();
            };
            let mut inner = cx.clone();
            inner.path.push(ExpansionStep::Stretch(factor));
            let elaborated = elaborate_items(lowering, &stretch.items(), &inner, part, voice, groups, pending);
            // The kernel's time-scaling action (course correction §14) plus
            // the matching renotation: a stretched quarter is *written* as a
            // half, not as a quarter that lasts twice as long.
            elaborated
                .map_payload(|payload| payload.stretched(factor))
                .scale(factor)
                .unwrap_or_else(|_| empty_segment())
        }
        VoiceItem::Retrograde(retrograde) => {
            let mut inner = cx.clone();
            inner.path.push(ExpansionStep::Retrograde);
            let elaborated = elaborate_items(lowering, &retrograde.items(), &inner, part, voice, groups, pending);
            reverse(&elaborated)
        }
        VoiceItem::Invert(invert) => {
            let text = invert.axis().unwrap_or_default();
            let span = lower::trimmed_span(invert.syntax());
            let Some(axis) = WrittenPitch::parse(&text) else {
                lowering.error(format!("`{text}` is not a pitch to invert around"), span);
                return empty_segment();
            };
            let mut inner = cx.clone();
            inner.path.push(ExpansionStep::Inversion { axis: text.clone() });
            let elaborated = elaborate_items(lowering, &invert.items(), &inner, part, voice, groups, pending);
            // Inversion is a payload map (course correction §13). A note
            // whose mirror image is unspellable is reported where it is
            // written and left alone, so one impossible note does not take
            // the rest of the phrase with it.
            let refused = std::cell::RefCell::new(Vec::new());
            let inverted = elaborated.map_payload(|payload| {
                payload.inverted(axis).unwrap_or_else(|| {
                    if let PayloadKind::Note { pitch } = payload.kind {
                        refused.borrow_mut().push((pitch, payload.origin.definition_span));
                    }
                    payload.clone()
                })
            });
            for (pitch, at) in refused.into_inner() {
                lowering.error(
                    format!("`{pitch}` inverted around `{text}` needs more than a double accidental"),
                    at,
                );
            }
            inverted
        }
        VoiceItem::Dynamic(dynamic) => {
            let text = dynamic.mark().unwrap_or_default();
            let span = lower::trimmed_span(dynamic.syntax());
            match DynamicMark::parse(&text) {
                Some(mark) => {
                    *pending = Some(PendingDynamic {
                        mark,
                        origin: origin_of(cx, span),
                        span,
                    });
                }
                None => lowering.error(format!("unknown dynamic marking `{text}`"), span),
            }
            empty_segment()
        }
    }
}

/// A tuplet ratio `n/d`, read without reducing: `4/4` is four in the time of
/// four, not one in the time of one.
fn parse_tuplet_ratio(text: &str) -> Option<(u32, u32)> {
    let (num, den) = text.split_once('/')?;
    let num: u32 = num.parse().ok()?;
    let den: u32 = den.parse().ok()?;
    (num > 0 && den > 0).then_some((num, den))
}

/// The written duration of a statement, scaled by the enclosing tuplets.
fn resolve_scaled_duration(lowering: &mut Lowering, node: &SyntaxNode, cx: &ExpandCx) -> Option<NotatedDuration> {
    let duration = lower::resolve_duration(lowering, node, cx)?;
    Some(if cx.scale == Ratio::ONE {
        duration
    } else {
        duration.scaled(cx.scale)
    })
}

/// Expand a `use` statement, mirroring the direct lowerer's binding rules.
fn elaborate_use(
    lowering: &mut Lowering,
    call: &musa_language::ast::UseStmt,
    cx: &ExpandCx,
    part: u32,
    voice: u32,
    groups: &[u32],
    pending: &mut Option<PendingDynamic>,
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
        scale: cx.scale,
    };
    let elaborated = elaborate_items(lowering, &body, &inner, part, voice, groups, pending);
    specialize(lowering, call, &elaborated)
}

/// Apply an occurrence's `with { note n = <pitch>; }` overrides (roadmap §9).
///
/// The ordinal counts the occurrence's *events* in time order, from one: a
/// chord is one position rather than one per pitch, and a rest takes a
/// position it cannot be given a pitch at. That is the same count the score
/// inspector shows as `▸ note 3` (`musa-project`'s `OriginFacts::note_index`),
/// and the two must agree — a composer reading a position off the score and
/// typing it into a `with` clause is naming the note they are looking at.
///
/// Overrides apply after the body elaborates, so what they respell is what
/// this call actually produced — including notes a transposition or an
/// inversion around it already moved.
///
/// Every override that lands records itself in the note's provenance, so the
/// score can still say both "this came from the motif" and "and this call
/// changed it" (roadmap §8.3).
fn specialize(
    lowering: &mut Lowering,
    call: &musa_language::ast::UseStmt,
    elaborated: &Timeline<VoicePayload>,
) -> Timeline<VoicePayload> {
    let overrides = call.overrides();
    if overrides.is_empty() {
        return elaborated.clone();
    }
    // The positions of this occurrence: one entry per group of occurrences
    // sharing a span, which is how a chord's pitches become one position.
    let mut positions: Vec<(usize, usize)> = Vec::new();
    for (index, occurrence) in elaborated.occurrences().iter().enumerate() {
        match positions.last_mut() {
            Some(&mut (start, ref mut end))
                if elaborated
                    .occurrences()
                    .get(start)
                    .is_some_and(|first| first.span() == occurrence.span()) =>
            {
                *end = index.saturating_add(1);
            }
            _ => positions.push((index, index.saturating_add(1))),
        }
    }

    let mut replacements: indexmap::IndexMap<usize, (WrittenPitch, SourceSpan)> = indexmap::IndexMap::new();
    for each in &overrides {
        let at = lower::trimmed_span(each.syntax());
        let Some(position) = each
            .position()
            .and_then(|text| text.parse::<usize>().ok())
            .filter(|n| *n > 0)
        else {
            lowering.error("a note override counts from `note 1`", at);
            continue;
        };
        let Some(pitch) = each.pitch().as_deref().and_then(WrittenPitch::parse) else {
            lowering.error("this override does not name a pitch", at);
            continue;
        };
        let Some(&(start, end)) = positions.get(position.saturating_sub(1)) else {
            lowering.error(
                format!(
                    "this occurrence has {} note{}, so there is no `note {position}`",
                    positions.len(),
                    if positions.len() == 1 { "" } else { "s" }
                ),
                at,
            );
            continue;
        };
        if end.saturating_sub(start) > 1 {
            lowering.error(
                format!("`note {position}` is a chord; an override respells one note"),
                at,
            );
            continue;
        }
        if !matches!(
            elaborated.occurrences().get(start).map(|it| &it.payload().kind),
            Some(&PayloadKind::Note { .. })
        ) {
            lowering.error(format!("`note {position}` is a rest; an override respells a note"), at);
            continue;
        }
        if replacements.insert(start, (pitch, at)).is_some() {
            lowering.error(format!("`note {position}` is overridden twice"), at);
        }
    }
    if replacements.is_empty() {
        return elaborated.clone();
    }

    let occurrences: Vec<Occurrence<VoicePayload>> = elaborated
        .occurrences()
        .iter()
        .enumerate()
        .map(|(index, occurrence)| {
            let Some(&(pitch, at)) = replacements.get(&index) else {
                return occurrence.clone();
            };
            let mut payload = occurrence.payload().clone();
            payload.kind = PayloadKind::Note { pitch };
            payload
                .origin
                .expansion_path
                .push(ExpansionStep::Specialization { override_site: at });
            Occurrence::new(occurrence.span(), payload)
        })
        .collect();
    timeline_or_empty(elaborated.extent(), occurrences)
}

/// The derived time reversal (roadmap §5.4, prompt 34): `(d, E)` becomes
/// `(d, {(d−e, d−s, a)})`.
///
/// A plain function over an elaborated timeline, deliberately: the kernel
/// needs no reversal primitive to express it, which is the evidence
/// `docs/kernel/08-open-questions.md` records for §34's smallest complete
/// basis.
///
/// Every mark stays with the note that carries it — a staccato is written on
/// a note, and reversing time does not move it. The tie is the exception,
/// because it is a relation *between* two notes rather than a property of
/// one: reversed, the tie belongs to what is now the earlier of the pair.
fn reverse(timeline: &Timeline<VoicePayload>) -> Timeline<VoicePayload> {
    let extent = timeline.extent();
    let mut mirrored: Vec<Occurrence<VoicePayload>> = timeline
        .occurrences()
        .iter()
        .map(|occurrence| {
            let span = occurrence.span();
            let start = Beat::new(extent.as_ratio() - span.end().as_ratio());
            let end = Beat::new(extent.as_ratio() - span.start().as_ratio());
            let mirrored = Span::new(start, end).unwrap_or(Span::ZERO);
            Occurrence::new(mirrored, occurrence.payload().clone())
        })
        .collect();
    // Stable by start, so the members of a chord stay adjacent and in the
    // order the adapter expects.
    mirrored.sort_by_key(|occurrence| (occurrence.span().start(), occurrence.span().end()));
    timeline_or_empty(extent, retie(&mirrored))
}

/// Move tie marks back one sounding position, in the reversed order.
///
/// A tie says "and the next one continues this". After reversal the pair
/// still sounds together, in the other order, so the mark moves from the
/// note that had it to the note it pointed at.
fn retie(occurrences: &[Occurrence<VoicePayload>]) -> Vec<Occurrence<VoicePayload>> {
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for (index, occurrence) in occurrences.iter().enumerate() {
        let span = occurrence.span();
        match groups.last_mut() {
            Some(&mut (start, ref mut end)) if occurrences.get(start).is_some_and(|first| first.span() == span) => {
                *end = index.saturating_add(1);
            }
            _ => groups.push((index, index.saturating_add(1))),
        }
    }
    let ties: Vec<bool> = groups
        .iter()
        .map(|&(start, _)| occurrences.get(start).is_some_and(|first| first.payload().marks.tie))
        .collect();
    let mut retied = Vec::with_capacity(occurrences.len());
    for (position, &(start, end)) in groups.iter().enumerate() {
        let tie = ties.get(position.saturating_add(1)).copied().unwrap_or(false);
        for occurrence in occurrences.get(start..end).unwrap_or_default() {
            let mut payload = occurrence.payload().clone();
            payload.marks.tie = tie;
            retied.push(Occurrence::new(occurrence.span(), payload));
        }
    }
    retied
}

/// A timeline over `extent`, or the empty segment when the occurrences do
/// not fit it (unreachable for elaborated music; never a panic).
fn timeline_or_empty(extent: Beat, occurrences: Vec<Occurrence<VoicePayload>>) -> Timeline<VoicePayload> {
    timeline(extent, occurrences).unwrap_or_else(|_| musa_kernel::zero())
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
        definition_span: span,
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
pub(crate) fn adapt_voice(lowering: &mut Lowering, timeline: &Timeline<VoicePayload>) -> Voice {
    let mut adapted = Vec::new();
    let mut index = 0;
    let occurrences = timeline.occurrences();
    while index < occurrences.len() {
        let Some(first) = occurrences.get(index) else { break };
        let span = first.span();
        let payload = first.payload();
        let onset = MusicalTime::new(span.start().as_ratio());
        match &payload.kind {
            PayloadKind::Rest => {
                adapted.push(Adapted {
                    onset,
                    duration: payload.duration.clone(),
                    kind: ScoreEventKind::Rest,
                    origin: payload.origin.clone(),
                    marks: payload.marks.clone(),
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
                adapted.push(Adapted {
                    onset,
                    duration: payload.duration.clone(),
                    kind,
                    origin: payload.origin.clone(),
                    marks: payload.marks.clone(),
                });
                index = index.saturating_add(consumed);
            }
        }
    }
    let adapted = merge_ties(lowering, adapted);
    identify(lowering, adapted)
}

/// One adapted occurrence, before it has an identity: ties still have to
/// merge, and merging changes how many events there are.
struct Adapted {
    onset: MusicalTime,
    duration: NotatedDuration,
    kind: ScoreEventKind,
    origin: Origin,
    marks: Marks,
}

/// Join tied runs into single sounding events (roadmap §6.3: a tie is
/// duration structure, not an annotation). The merged event keeps the first
/// piece's onset, provenance, and marks, and its written pieces are the
/// noteheads the composer asked for.
fn merge_ties(lowering: &mut Lowering, adapted: Vec<Adapted>) -> Vec<Adapted> {
    let mut merged: Vec<Adapted> = Vec::with_capacity(adapted.len());
    for item in adapted {
        let joins = merged.last().is_some_and(|previous| previous.marks.tie);
        if !joins {
            merged.push(item);
            continue;
        }
        let Some(previous) = merged.last_mut() else {
            merged.push(item);
            continue;
        };
        if previous.kind != item.kind || matches!(item.kind, ScoreEventKind::Rest) {
            lowering.error(
                "a tie must be followed by the same pitch or chord",
                item.origin.definition_span,
            );
            previous.marks.tie = false;
            merged.push(item);
            continue;
        }
        previous.duration = previous.duration.tied_to(&item.duration);
        previous.marks.tie = item.marks.tie;
        previous.marks.articulations.extend(item.marks.articulations);
    }
    if let Some(last) = merged.last()
        && last.marks.tie
    {
        lowering.error("this tie has no note after it", last.origin.definition_span);
    }
    merged
}

/// Give the voice's events their identities, and turn the marks they carry
/// into annotations now that there is something to anchor to.
fn identify(lowering: &mut Lowering, adapted: Vec<Adapted>) -> Voice {
    let mut events = Vec::with_capacity(adapted.len());
    // `IndexMap` rather than a hash map: the annotation order is part of the
    // snapshot, and it follows the order the blocks were entered.
    let mut ranges: indexmap::IndexMap<u32, (crate::score::EventId, crate::score::EventId)> = indexmap::IndexMap::new();
    for item in adapted {
        let id = lowering.event_id();
        for group in &item.marks.groups {
            ranges
                .entry(*group)
                .and_modify(|range| range.1 = id)
                .or_insert((id, id));
        }
        for mark in item.marks.articulations {
            lowering.annotations.push_articulation(ArticulationMarking {
                at: id,
                mark,
                origin: item.origin.clone(),
            });
        }
        if let Some((mark, origin)) = item.marks.dynamic {
            lowering
                .annotations
                .push_dynamic(DynamicMarking { at: id, mark, origin });
        }
        events.push(ScoreEvent {
            id,
            origin: item.origin,
            onset: item.onset,
            notated_duration: item.duration,
            kind: item.kind,
        });
    }
    ranges.sort_keys();
    for (group, (from, to)) in ranges {
        let Some(info) = lowering.groups.get(&group) else {
            continue;
        };
        let origin = info.origin.clone();
        match info.kind {
            GroupKind::Slur => lowering.annotations.push_slur(SlurSpan { from, to, origin }),
            GroupKind::Phrase { ref name } => lowering.annotations.push_phrase(PhraseSpan {
                name: name.clone(),
                from,
                to,
                origin,
            }),
            GroupKind::Hairpin { grows, target } => lowering.annotations.push_hairpin(HairpinSpan {
                from,
                to,
                grows,
                target,
                origin,
            }),
            GroupKind::Tuplet { num, den } => lowering.annotations.push_tuplet(TupletSpan {
                from,
                to,
                num,
                den,
                origin,
            }),
        }
    }
    Voice { events }
}

/// A tuplet has to be spellable, and a group split across a barline is not:
/// the notes on either side would need their own bracket and their own
/// ratio, which is a different piece of music from the one that was written.
fn check_tuplets(lowering: &mut Lowering, snapshot: &ScoreSnapshot) {
    let measure = snapshot.meter_map.measure_len().as_ratio();
    if measure == Ratio::ZERO {
        return;
    }
    let mut offenders = Vec::new();
    for tuplet in snapshot.annotations.tuplets() {
        let mut start = None;
        let mut end = None;
        for (_, part) in snapshot.parts.iter() {
            for voice in part.voices.values() {
                for event in &voice.events {
                    if event.id < tuplet.from || event.id > tuplet.to {
                        continue;
                    }
                    let event_end = event.onset + event.notated_duration.value;
                    start = Some(start.map_or(event.onset, |current: MusicalTime| current.min(event.onset)));
                    end = Some(end.map_or(event_end, |current: MusicalTime| current.max(event_end)));
                }
            }
        }
        let (Some(start), Some(end)) = (start, end) else {
            continue;
        };
        let first_bar = (start.as_ratio() / measure).floor();
        let last_bar = ((end.as_ratio() - Ratio::new(1, 1_000_000)) / measure).floor();
        if first_bar != last_bar {
            offenders.push(tuplet.origin.definition_span);
        }
    }
    for span in offenders {
        lowering.error("a tuplet must fit inside one measure", span);
    }
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
