//! Elaboration of the surface language through the temporal kernel
//! (docs/kernel/06, course correction §19–20, §30 Step 4).
//!
//! This is *the* semantic path: name resolution, motif registration and unit
//! checks come from `resolve.rs`, voice content elaborates into
//! `Timeline<ScoreFact>` values built from kernel `sequence`/`overlay`, and
//! `project.rs` reads a `ScoreSnapshot` back out of the result (§27). It was
//! the second path until prompt 41 deleted the direct lowerer it was
//! validated against.
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
use crate::origin::{ExpansionStep, Origin, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::resolve::{self, ExpandCx, Resolver};
use crate::score::{
    ArticulationMark, DynamicMark, MeterMap, Mode, NotatedDuration, Part, PartId, ScoreSnapshot, TempoChange, Voice,
    VoiceId,
};
use crate::time::MusicalTime;
use musa_kernel::{Beat, Occurrence, Span, Timeline, overlay, sequence, timeline};
use musa_language::SyntaxNode;
use musa_language::ast::{AstNode as _, PieceDecl, VoiceItem};
use num_rational::Ratio;
use std::fmt::Write as _;

/// Where a fact sits in the score's *structure*.
///
/// Never where it sits in time — that is the occurrence's span, and keeping
/// the two apart is the point (docs/kernel/03). A slur moves in time without
/// changing voice; a voice is renamed without moving anything.
///
/// A part scope arrives when a part-wide fact does; a variant with no
/// producer would be a public item with no caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// The piece as a whole: key, meter, form markers, chord symbols.
    Piece,
    /// One voice of one part, by the ids the snapshot uses.
    Voice { part: u32, voice: u32 },
}

impl Scope {
    /// The (part, voice) pair, for bucketing during projection, or `None`
    /// when the fact belongs to the piece rather than to a voice.
    pub(crate) fn voice(self) -> Option<(u32, u32)> {
        match self {
            Self::Piece => None,
            Self::Voice { part, voice } => Some((part, voice)),
        }
    }
}

/// What a fact *states*. Where it is in time is the span; where it is in the
/// score is the scope; why it exists is the origin.
///
/// Articulations are a field of `Note`/`Rest` rather than facts of their own,
/// by the rule that decides the question: does it have an extent and an
/// identity? A staccato dot has neither — no span but its note's, unmovable
/// without moving the note — so making it an occurrence would only force the
/// projection to re-join it by span, which is the information loss this
/// design exists to delete, inverted. A slur has both.
#[derive(Clone, Debug)]
pub(crate) enum FactKind {
    /// A sounding note. `duration` is notation intent: how many noteheads
    /// spell the span (roadmap §2 — notated ≠ performed).
    Note {
        pitch: WrittenPitch,
        duration: NotatedDuration,
        articulations: Vec<ArticulationMark>,
    },
    /// A written rest — notation intent, not a silence object (§2).
    Rest {
        duration: NotatedDuration,
        articulations: Vec<ArticulationMark>,
    },
    /// A slur over the region it spans.
    Slur,
    /// A named phrase over the region it spans.
    Phrase { name: String },
    /// A tuplet bracket, unreduced as the backends need it.
    Tuplet { num: u32, den: u32 },
    /// A dynamic marking: a point at the onset it applies from.
    Dynamic { mark: DynamicMark },
    /// A hairpin over the region it spans, and the mark it arrives at.
    Hairpin { grows: bool, target: DynamicMark },
    /// The key signature, over the region it governs — the whole piece
    /// while the grammar has no `modulate` (prompt 40).
    Key { tonic: PitchClass, mode: Mode },
    /// The meter, over the region it governs — likewise the whole piece.
    Meter { numerator: u32, denominator: u32 },
    /// A form marker at the place it names.
    Section { name: String },
    /// A chord symbol at the place it is written; a region once a chord's
    /// duration can be written.
    Harmony { symbol: crate::harmony::ChordSymbol },
}

impl FactKind {
    /// Whether this fact is a note or a rest — the facts that become events
    /// and are given identities.
    pub(crate) fn is_event(&self) -> bool {
        matches!(self, Self::Note { .. } | Self::Rest { .. })
    }

    /// The articulations written on this fact, if any.
    pub(crate) fn articulations_of(&self) -> &[ArticulationMark] {
        match self {
            Self::Note { articulations, .. } | Self::Rest { articulations, .. } => articulations,
            Self::Slur
            | Self::Phrase { .. }
            | Self::Tuplet { .. }
            | Self::Dynamic { .. }
            | Self::Hairpin { .. }
            | Self::Key { .. }
            | Self::Meter { .. }
            | Self::Section { .. }
            | Self::Harmony { .. } => &[],
        }
    }

    /// The written duration, for the facts that have one.
    pub(crate) fn duration_of(&self) -> Option<&NotatedDuration> {
        match self {
            Self::Note { duration, .. } | Self::Rest { duration, .. } => Some(duration),
            Self::Slur
            | Self::Phrase { .. }
            | Self::Tuplet { .. }
            | Self::Dynamic { .. }
            | Self::Hairpin { .. }
            | Self::Key { .. }
            | Self::Meter { .. }
            | Self::Section { .. }
            | Self::Harmony { .. } => None,
        }
    }
}

/// One elaborated fact of a score: what is stated, where in the score's
/// structure it belongs, and why it exists (docs/kernel/06).
#[derive(Clone, Debug)]
pub(crate) struct ScoreFact {
    pub(crate) scope: Scope,
    pub(crate) kind: FactKind,
    pub(crate) origin: Origin,
    /// Elaboration-only: this written notehead is tied to the next one.
    ///
    /// A tie is not a fact — it says two noteheads spell **one** occurrence —
    /// so it is resolved by merging during elaboration and is `false` on
    /// every fact that leaves [`elaborate_items`]. Nothing downstream reads
    /// it; the projection asserts as much in debug builds.
    pub(crate) tied: bool,
}

impl ScoreFact {
    fn new(scope: Scope, kind: FactKind, origin: Origin) -> Self {
        Self {
            scope,
            kind,
            origin,
            tied: false,
        }
    }

    /// The same fact sounding and notated `factor` times as long.
    fn stretched(&self, factor: Ratio<i64>) -> Self {
        let mut stretched = self.clone();
        match &mut stretched.kind {
            FactKind::Note { duration, .. } | FactKind::Rest { duration, .. } => {
                *duration = duration.stretched(factor);
            }
            FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. } => {}
        }
        stretched
    }

    /// The same fact with its pitch mirrored about `axis`, or `None` when
    /// the mirror image is not spellable (roadmap §5.4's meaningful failure).
    fn inverted(&self, axis: WrittenPitch) -> Option<Self> {
        let FactKind::Note { pitch, .. } = &self.kind else {
            return Some(self.clone());
        };
        let mirrored = pitch.invert(axis)?;
        let mut inverted = self.clone();
        if let FactKind::Note { pitch, .. } = &mut inverted.kind {
            *pitch = mirrored;
        }
        Some(inverted)
    }

    /// The pitch, for the facts that have one.
    pub(crate) fn pitch_of(&self) -> Option<WrittenPitch> {
        match &self.kind {
            FactKind::Note { pitch, .. } => Some(*pitch),
            FactKind::Rest { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. } => None,
        }
    }
}

impl musa_kernel::Canonical for ScoreFact {
    /// Deterministic, injective key for canonical ordering and semantic
    /// equality (docs/kernel/05 N3): identity, kind, and full provenance.
    ///
    /// A note or rest with nothing written on it keys exactly as it did
    /// before facts were heterogeneous, so a piece of plain notes has the
    /// normal form it has always had.
    fn canonical_key(&self) -> String {
        let articulations = |marks: &[ArticulationMark]| {
            if marks.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = marks.iter().map(|mark| mark.name()).collect();
                format!("|artic:{}", names.join(","))
            }
        };
        let kind = match &self.kind {
            FactKind::Note {
                pitch,
                duration,
                articulations: marks,
            } => format!("note:{pitch}|{}{}", duration.spelling, articulations(marks)),
            FactKind::Rest {
                duration,
                articulations: marks,
            } => format!("rest|{}{}", duration.spelling, articulations(marks)),
            FactKind::Slur => "slur|".to_owned(),
            FactKind::Phrase { name } => format!("phrase:{name}|"),
            FactKind::Tuplet { num, den } => format!("tuplet:{num}/{den}|"),
            FactKind::Dynamic { mark } => format!("dynamic:{}|", mark.name()),
            FactKind::Hairpin { grows, target } => {
                format!("hairpin:{}:{}|", if *grows { "cres" } else { "dim" }, target.name())
            }
            FactKind::Key { tonic, mode } => {
                let mode = match mode {
                    Mode::Major => "major",
                    Mode::Minor => "minor",
                };
                format!("key:{tonic}:{mode}|")
            }
            FactKind::Meter { numerator, denominator } => format!("meter:{numerator}/{denominator}|"),
            FactKind::Section { name } => format!("section:{name}|"),
            FactKind::Harmony { symbol } => format!("harmony:{}|", symbol.text),
        };
        // Written rather than `format!`ed so the scope costs no second
        // allocation: P4 walks every occurrence on every edit.
        let mut key = String::with_capacity(kind.len().saturating_add(32));
        match self.scope {
            // `*` sorts before any part number, so at one instant the context
            // a reader meets first is the context that prints first.
            Scope::Piece => key.push_str("*|*|"),
            Scope::Voice { part, voice } => {
                let _ = write!(key, "{part}|{voice}|");
            }
        }
        let _ = write!(
            key,
            "{}|{}|{}|{:?}",
            kind, self.origin.source_span.start, self.origin.source_span.end, self.origin.expansion_path,
        );
        key
    }
}

/// Elaborate `source` through the temporal kernel and adapt the result into
/// a `ScoreSnapshot` (docs/kernel/06, prompt 11).
pub(crate) fn elaborate(source: &SourceDocument, options: &crate::CompileOptions) -> Compilation {
    let document = musa_language::parse(source.text());
    let mut resolver = Resolver::new();
    elaborate_parsed(&document, source.name(), options, &mut resolver)
}

/// One voice's elaborated timeline, before the snapshot adapter sees it.
pub(crate) type VoiceTimeline = Timeline<ScoreFact>;

/// Everything after parsing (docs/kernel/06): elaborate, adapt, check.
///
/// Split out of [`elaborate`] so the parse and the semantic work can be
/// measured apart; `resolver` arrives from the caller for the same reason
/// (see `crate::bench`). The production path passes a fresh one.
pub(crate) fn elaborate_parsed(
    document: &musa_language::ParsedDocument,
    name: &str,
    options: &crate::CompileOptions,
    resolver: &mut Resolver,
) -> Compilation {
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
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }
    let Some(piece) = PieceDecl::from_root(&document.syntax()) else {
        resolver.error("no `piece` declaration", SourceSpan::new(0, 0));
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    };

    let mut snapshot = ScoreSnapshot::default();
    let libraries = crate::imports::load(resolver, name, &piece, &options.imports);
    elaborate_libraries(resolver, &libraries, &mut snapshot);
    resolve::lower_header(resolver, &piece, &mut snapshot);
    if let Some(score) = piece.score() {
        let context = elaborate_score(resolver, &piece, &score, &mut snapshot);
        elaborate_tempo_changes(resolver, &piece, &mut snapshot, &context);
    }
    snapshot.set_annotations(std::mem::take(&mut resolver.annotations));
    resolve::check_measure_sanity(resolver, &snapshot);
    check_tuplets(resolver, &snapshot);
    if resolver
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }
    let imported_studios: Vec<musa_language::ast::StudioDecl> =
        libraries.each().filter_map(|(_, library)| library.studio()).collect();
    let studio = resolve::lower_studio(resolver, &piece, &snapshot, &imported_studios);
    if resolver
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }
    Compilation::new(Some(snapshot), std::mem::take(&mut resolver.diagnostics)).with_studio(studio)
}

/// What the piece timeline says about the piece as a whole, for the callers
/// that need it after the projection has run.
///
/// Both fields are read *off the timeline* — the extent is the kernel's own,
/// and the meter is the meter occurrence's payload — which is why no function
/// in this module recomputes either from the snapshot.
pub(crate) struct PieceContext {
    /// Where the piece ends, in whole notes.
    extent: MusicalTime,
    /// The meter that governs it.
    meter: MeterMap,
}

/// Walk parts and voices exactly as the direct lowerer does, elaborating
/// each voice into kernel facts — and then overlay every one of them, plus
/// the piece's key, meter, form markers and chord symbols, into a single
/// `Timeline<ScoreFact>` for the whole piece, which is projected once.
///
/// One compilation, one temporal object (course correction §21). Part and
/// voice identity live in `Scope`, not in a timeline per voice, which is the
/// evidence Q3's working stance asked for.
fn elaborate_score(
    resolver: &mut Resolver,
    piece: &PieceDecl,
    score: &musa_language::ast::ScoreDecl,
    snapshot: &mut ScoreSnapshot,
) -> PieceContext {
    let mut voice_names: indexmap::IndexMap<PartId, indexmap::IndexMap<VoiceId, String>> = indexmap::IndexMap::new();
    let mut metadata: Vec<(PartId, String, Option<crate::score::Clef>)> = Vec::new();
    let mut lanes: Vec<Timeline<ScoreFact>> = Vec::new();
    for part in score.parts() {
        let name = part.name().unwrap_or_default();
        let part_key = resolver.declare(crate::resolve::DeclInfo::Part);
        let _ = resolve::ordinal(resolver, part_key);
        if metadata.iter().any(|(_, existing, _)| *existing == name) {
            resolver.error(format!("duplicate part `{name}`"), resolve::span_of(part.syntax()));
            continue;
        }
        let id = PartId(resolver.next_part);
        resolver.next_part = resolver.next_part.saturating_add(1);

        let (clef, profile) = resolve::part_metadata(resolver, &part, snapshot.profiles());
        if let Some(profile) = profile {
            snapshot.profiles_mut().assign(&name, profile);
        }

        let mut names = indexmap::IndexMap::new();
        for (index, voice) in part.voices().iter().enumerate() {
            let voice_name = voice.name().unwrap_or_default();
            let voice_key = resolver.declare(crate::resolve::DeclInfo::Voice);
            let declaration = resolve::ordinal(resolver, voice_key);
            if names.values().any(|existing| *existing == voice_name) {
                resolver.error(
                    format!("duplicate voice `{voice_name}` in part `{name}`"),
                    resolve::span_of(voice.syntax()),
                );
                continue;
            }
            let voice_id = VoiceId(u32::try_from(index).unwrap_or(u32::MAX));
            let timeline = elaborate_voice(resolver, voice, declaration, id.0, voice_id.0);
            if let Some(sink) = &mut resolver.timeline_sink {
                sink.push(timeline.clone());
            }
            lanes.push(timeline);
            names.insert(voice_id, voice_name);
        }
        voice_names.insert(id, names);
        metadata.push((id, name, clef));
    }

    let music = overlay(lanes);
    let extent = music.extent();
    let context = context_facts(resolver, piece, score, snapshot, extent);
    let whole = overlay(vec![music, context]);
    let projection = crate::project::project(resolver, &whole);
    snapshot.set_key(projection.key);
    snapshot.set_meter(projection.meter);
    let meter = projection.meter;
    let mut projected = projection.voices;
    for (id, name, clef) in metadata {
        let names = voice_names.swap_remove(&id).unwrap_or_default();
        let mut voices = indexmap::IndexMap::with_capacity(names.len());
        for voice_id in names.keys() {
            let voice = projected
                .swap_remove(&(id.0, voice_id.0))
                .unwrap_or_else(|| Voice::new(Vec::new()));
            voices.insert(*voice_id, voice);
        }
        snapshot
            .parts_mut()
            .insert(id, Part::new(id, name, clef, voices, names));
    }
    PieceContext {
        extent: MusicalTime::new(extent.as_ratio()),
        meter,
    }
}

/// The facts that are about the piece rather than about a voice: its key,
/// its meter, its form markers, and its chord symbols (roadmap §8.2).
///
/// Key and meter are *regions*. Today they cover `[0, d]`, because the
/// grammar has no `modulate` and no mid-piece `meter`; when it grows one the
/// change is more occurrences, not a second representation of the same
/// question (prompt 40). A region that happens to cover everything is not a
/// special case; a piece-wide scalar is.
///
/// Sections and chord symbols are *points*, resolved here because a position
/// is only meaningful once the piece has a length to be inside of. Nothing
/// interprets them: a chord symbol is parsed so a later library can read it,
/// and that is the end of the core's involvement.
fn context_facts(
    resolver: &mut Resolver,
    piece: &PieceDecl,
    score: &musa_language::ast::ScoreDecl,
    snapshot: &mut ScoreSnapshot,
    extent: Beat,
) -> Timeline<ScoreFact> {
    let declaration = crate::origin::DeclarationId::default();
    let at_span = |span: SourceSpan| Origin {
        source_span: span,
        definition_span: span,
        declaration,
        expansion_path: Vec::new(),
    };
    // `lower_header` parsed the key and the meter out of the header; take
    // them, so that the only thing which puts either back into the snapshot
    // is the projection of the timeline they are about to enter.
    let meter = snapshot.take_meter();
    let key = snapshot.take_key();

    let region = Span::new(Beat::ZERO, extent).unwrap_or(Span::ZERO);
    let mut occurrences = vec![Occurrence::new(
        region,
        ScoreFact::new(
            Scope::Piece,
            FactKind::Meter {
                numerator: meter.numerator(),
                denominator: meter.denominator(),
            },
            // An unwritten meter is still a meter — 4/4 governs the piece
            // whether or not anybody said so — so the fact exists either way
            // and points at the header when there is one to point at.
            at_span(
                piece
                    .meter()
                    .map_or_else(|| SourceSpan::new(0, 0), |node| resolve::span_of(node.syntax())),
            ),
        ),
    )];
    if let Some(key) = key {
        occurrences.push(Occurrence::new(
            region,
            ScoreFact::new(
                Scope::Piece,
                FactKind::Key {
                    tonic: key.tonic(),
                    mode: key.mode(),
                },
                at_span(
                    piece
                        .key()
                        .map_or_else(|| SourceSpan::new(0, 0), |node| resolve::span_of(node.syntax())),
                ),
            ),
        ));
    }

    let extent_time = MusicalTime::new(extent.as_ratio());
    for section in score.sections() {
        let span = resolve::trimmed_span(section.syntax());
        let Some(at) = resolve_position(resolver, section.position().as_ref(), span, meter, extent_time) else {
            continue;
        };
        occurrences.push(point_at(
            at,
            ScoreFact::new(
                Scope::Piece,
                FactKind::Section {
                    name: section.name().unwrap_or_default(),
                },
                at_span(span),
            ),
        ));
    }
    let lanes = score.harmonies();
    for extra in lanes.iter().skip(1) {
        resolver.error(
            "one `harmony` lane per score; write every chord in the first one",
            resolve::trimmed_span(extra.syntax()),
        );
    }
    for chord in lanes.iter().flat_map(musa_language::ast::HarmonyDecl::chords) {
        let span = resolve::trimmed_span(chord.syntax());
        let Some(at) = resolve_position(resolver, chord.position().as_ref(), span, meter, extent_time) else {
            continue;
        };
        let Some(written) = chord.symbol() else {
            continue;
        };
        if !written.is_one_word() {
            resolver.error("a chord symbol is one word, such as `am` or `fmaj7`", span);
            continue;
        }
        let text = written.text();
        let Some(symbol) = crate::harmony::ChordSymbol::parse(&text) else {
            resolver.error(format!("`{text}` is not a chord symbol musa can read"), span);
            continue;
        };
        occurrences.push(point_at(
            at,
            ScoreFact::new(Scope::Piece, FactKind::Harmony { symbol }, at_span(span)),
        ));
    }
    timeline_or_empty(extent, occurrences)
}

/// A point occurrence at an absolute time, for the facts that are placed by
/// coordinate rather than by where the cursor reached.
fn point_at(at: MusicalTime, fact: ScoreFact) -> Occurrence<ScoreFact> {
    let instant = Beat::new(at.as_ratio());
    let span = Span::new(instant, instant).unwrap_or(Span::ZERO);
    Occurrence::new(span, fact)
}

/// Register everything the imported libraries declare, before the piece's
/// own declarations, so a collision is reported against the library that
/// caused it (roadmap §16).
fn elaborate_libraries(resolver: &mut Resolver, libraries: &crate::imports::Libraries, snapshot: &mut ScoreSnapshot) {
    for (path, library) in libraries.each() {
        if let Some(performance) = library.performance() {
            let profiles = resolve::parse_profiles(resolver, &performance);
            resolve::merge_profiles(
                resolver,
                snapshot,
                &profiles,
                resolve::span_of(performance.syntax()),
                Some(path),
            );
        }
        resolve::register_motifs(resolver, snapshot, &library.motifs(), Some(path));
    }
}

/// Resolve the piece's tempo changes against the meter (roadmap §6.3).
///
/// A tempo change is written where a form marker is written — `at 9:1` — and
/// for the same reason: a tempo belongs to a place in the piece, not to a
/// note. Positions are resolved after the parts exist so that a tempo nobody
/// ever reaches is an error rather than a silent segment.
fn elaborate_tempo_changes(
    resolver: &mut Resolver,
    piece: &musa_language::ast::PieceDecl,
    snapshot: &mut ScoreSnapshot,
    context: &PieceContext,
) {
    let declaration = crate::origin::DeclarationId::default();
    let mut changes: Vec<TempoChange> = Vec::new();
    for tempo in piece.tempos().iter().filter(|tempo| tempo.position().is_some()) {
        let span = resolve::trimmed_span(tempo.syntax());
        let Some(at) = resolve_position(resolver, tempo.position().as_ref(), span, context.meter, context.extent)
        else {
            continue;
        };
        if at == MusicalTime::ZERO {
            resolver.error("the tempo at `1:1` is the piece's tempo; write it without `at`", span);
            continue;
        }
        if changes.iter().any(|existing| existing.at == at) {
            resolver.error("two tempos at the same place", span);
            continue;
        }
        let (beat, bpm) = resolve::tempo_reading(resolver, tempo);
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
    snapshot.tempo_mut().changes = changes;
}

/// Turn a `measure:beat` coordinate into time, or report why it is not one.
///
/// Measures and beats count from one, the way a composer reads them off the
/// page, and a position past the end of the piece is an error: a chord symbol
/// nobody will ever reach is a mistake, not a comment.
///
/// `meter` is the meter *occurrence*'s payload and `extent` is the timeline's
/// own extent — neither is recomputed from the snapshot, which is the point
/// of prompt 40. The extent is exact rather than a maximum over event ends,
/// and a piece that ends in a rest still ends where the rest ends, because a
/// rest is an occurrence.
fn resolve_position(
    resolver: &mut Resolver,
    position: Option<&musa_language::ast::Position>,
    span: SourceSpan,
    meter: MeterMap,
    extent: MusicalTime,
) -> Option<MusicalTime> {
    let position = position?;
    let measure: i64 = position.measure()?.parse().ok()?;
    let beat_text = position.beat()?;
    let beat = resolve::parse_ratio(&beat_text).or_else(|| beat_text.parse::<i64>().ok().map(Ratio::from_integer))?;
    if measure < 1 || beat < Ratio::ONE {
        resolver.error("measures and beats count from `1:1`", span);
        return None;
    }
    let measure_len = meter.measure_len().as_ratio();
    let beat_len = Ratio::new(1, i64::from(meter.denominator().max(1)));
    let at = MusicalTime::new(measure_len * (measure - 1) + beat_len * (beat - Ratio::ONE));
    if at >= extent && extent > MusicalTime::default() {
        resolver.error(format!("the piece ends before `{measure}:{beat_text}`"), span);
        return None;
    }
    Some(at)
}

/// Elaborate one voice: `sequence` of its items (docs/kernel/06).
fn elaborate_voice(
    resolver: &mut Resolver,
    voice: &musa_language::ast::VoiceDecl,
    declaration: crate::origin::DeclarationId,
    part: u32,
    voice_id: u32,
) -> Timeline<ScoreFact> {
    let cx = ExpandCx {
        params: indexmap::IndexMap::new(),
        intervals: Vec::new(),
        path: Vec::new(),
        declaration,
        origin_span: None,
        max_motif: usize::MAX,
        scale: Ratio::ONE,
    };
    let timeline = elaborate_items(resolver, &voice.items(), &cx, Scope::Voice { part, voice: voice_id });
    check_dangling_tie(resolver, timeline)
}

/// Elaborate voice items into a kernel timeline (sequence of item segments),
/// with tied noteheads merged.
///
/// Merging here rather than once per voice is what makes a tie invisible to
/// everything above: an inner block's ties are resolved before the block is
/// reversed or scaled, so `retrograde` mirrors ordinary occurrences and needs
/// no repair, and a tie that crosses a block boundary merges at the level
/// that contains both sides.
fn elaborate_items(resolver: &mut Resolver, items: &[VoiceItem], cx: &ExpandCx, scope: Scope) -> Timeline<ScoreFact> {
    let mut segments = Vec::new();
    for item in items {
        segments.push(elaborate_item(resolver, item, cx, scope));
    }
    merge_ties(resolver, sequence(segments))
}

/// The articulations written on a note or chord statement.
fn articulations_of(resolver: &mut Resolver, names: &[String], span: SourceSpan) -> Vec<ArticulationMark> {
    let mut articulations = Vec::new();
    for name in names {
        match ArticulationMark::parse(name) {
            Some(mark) => articulations.push(mark),
            None => resolver.error(format!("unknown articulation `{name}`"), span),
        }
    }
    articulations
}

/// Elaborate one item; malformed items elaborate to the empty segment
/// `(0, ∅)` — the direct lowerer's `continue` (diagnostic already emitted).
fn elaborate_item(resolver: &mut Resolver, item: &VoiceItem, cx: &ExpandCx, scope: Scope) -> Timeline<ScoreFact> {
    match item {
        VoiceItem::Note(note) => {
            let Some(duration) = resolve_scaled_duration(resolver, note.syntax(), cx) else {
                return empty_segment();
            };
            let pitch_text = note.pitch().unwrap_or_default();
            let Some(pitch) = resolve::resolve_pitch(resolver, &pitch_text, note.syntax(), cx) else {
                return empty_segment();
            };
            let span = resolve::trimmed_span(note.syntax());
            let articulations = articulations_of(resolver, &note.articulations(), span);
            let origin = origin_of(cx, span);
            let mut fact = ScoreFact::new(
                scope,
                FactKind::Note {
                    pitch,
                    duration: duration.clone(),
                    articulations,
                },
                origin,
            );
            fact.tied = note.tied();
            single(&duration, fact)
        }
        VoiceItem::Rest(rest) => {
            let Some(duration) = resolve_scaled_duration(resolver, rest.syntax(), cx) else {
                return empty_segment();
            };
            let span = resolve::trimmed_span(rest.syntax());
            let origin = origin_of(cx, span);
            single(
                &duration,
                ScoreFact::new(
                    scope,
                    FactKind::Rest {
                        duration: duration.clone(),
                        articulations: Vec::new(),
                    },
                    origin,
                ),
            )
        }
        VoiceItem::Chord(chord) => {
            let Some(duration) = resolve_scaled_duration(resolver, chord.syntax(), cx) else {
                return empty_segment();
            };
            let node_span = resolve::trimmed_span(chord.syntax());
            let articulations = articulations_of(resolver, &chord.articulations(), node_span);
            let mut pitches = Vec::new();
            for text in chord.pitches() {
                match WrittenPitch::parse(&text).map(|pitch| apply_intervals(resolver, pitch, cx, chord.syntax())) {
                    Some(Some(pitch)) => {
                        let origin = origin_of(cx, node_span);
                        let mut fact = ScoreFact::new(
                            scope,
                            FactKind::Note {
                                pitch,
                                duration: duration.clone(),
                                articulations: articulations.clone(),
                            },
                            origin,
                        );
                        fact.tied = chord.tied();
                        pitches.push(fact);
                    }
                    Some(None) => break,
                    None => {
                        resolver.error(
                            format!("invalid chord pitch `{text}`"),
                            resolve::span_of(chord.syntax()),
                        );
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
        VoiceItem::Use(call) => elaborate_use(resolver, call, cx, scope),
        VoiceItem::Transpose(transpose) => {
            let text = transpose.interval().unwrap_or_default();
            let Some(interval) = crate::origin::Interval::parse(&text, transpose.is_down()) else {
                resolver.error(
                    format!("unknown interval `{text}`"),
                    resolve::span_of(transpose.syntax()),
                );
                return empty_segment();
            };
            let mut inner = cx.clone();
            inner.intervals.push(interval);
            inner.path.push(ExpansionStep::Transposition(interval));
            elaborate_items(resolver, &transpose.items(), &inner, scope)
        }
        VoiceItem::Repeat(repeat) => {
            let count: u32 = repeat.count().and_then(|text| text.parse().ok()).unwrap_or(0);
            let mut segments = Vec::new();
            for iteration in 0..count {
                let mut inner = cx.clone();
                inner.path.push(ExpansionStep::RepeatIteration(iteration));
                segments.push(elaborate_items(resolver, &repeat.items(), &inner, scope));
            }
            sequence(segments)
        }
        VoiceItem::Slur(slur) => {
            let origin = origin_of(cx, resolve::trimmed_span(slur.syntax()));
            let body = elaborate_items(resolver, &slur.items(), cx, scope);
            over(body, ScoreFact::new(scope, FactKind::Slur, origin))
        }
        VoiceItem::Phrase(phrase) => {
            let origin = origin_of(cx, resolve::trimmed_span(phrase.syntax()));
            let name = phrase.name().unwrap_or_default();
            let body = elaborate_items(resolver, &phrase.items(), cx, scope);
            over(body, ScoreFact::new(scope, FactKind::Phrase { name }, origin))
        }
        VoiceItem::Hairpin(hairpin) => {
            let span = resolve::trimmed_span(hairpin.syntax());
            let text = hairpin.target().unwrap_or_default();
            let Some(target) = DynamicMark::parse(&text) else {
                resolver.error(format!("unknown dynamic marking `{text}`"), span);
                return empty_segment();
            };
            let origin = origin_of(cx, span);
            let grows = hairpin.grows();
            let body = elaborate_items(resolver, &hairpin.items(), cx, scope);
            over(body, ScoreFact::new(scope, FactKind::Hairpin { grows, target }, origin))
        }
        VoiceItem::Tuplet(tuplet) => {
            let text = tuplet.ratio().unwrap_or_default();
            let span = resolve::trimmed_span(tuplet.syntax());
            let Some((num, den)) = parse_tuplet_ratio(&text) else {
                resolver.error(format!("`{text}` is not a tuplet ratio such as `3/2`"), span);
                return empty_segment();
            };
            let origin = origin_of(cx, span);
            let mut inner = cx.clone();
            inner.scale = cx.scale * Ratio::new(i64::from(den), i64::from(num));
            let body = elaborate_items(resolver, &tuplet.items(), &inner, scope);
            over(body, ScoreFact::new(scope, FactKind::Tuplet { num, den }, origin))
        }
        VoiceItem::Stretch(stretch) => {
            let text = stretch.factor().unwrap_or_default();
            let span = resolve::trimmed_span(stretch.syntax());
            let factor = resolve::parse_ratio(&text).or_else(|| text.parse::<i64>().ok().map(Ratio::from_integer));
            let Some(factor) = factor.filter(|factor| *factor > Ratio::ZERO) else {
                resolver.error(format!("`{text}` is not a positive stretch factor such as `3/2`"), span);
                return empty_segment();
            };
            let mut inner = cx.clone();
            inner.path.push(ExpansionStep::Stretch(factor));
            let elaborated = elaborate_items(resolver, &stretch.items(), &inner, scope);
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
            let elaborated = elaborate_items(resolver, &retrograde.items(), &inner, scope);
            reverse(&elaborated)
        }
        VoiceItem::Invert(invert) => {
            let text = invert.axis().unwrap_or_default();
            let span = resolve::trimmed_span(invert.syntax());
            let Some(axis) = WrittenPitch::parse(&text) else {
                resolver.error(format!("`{text}` is not a pitch to invert around"), span);
                return empty_segment();
            };
            let mut inner = cx.clone();
            inner.path.push(ExpansionStep::Inversion { axis: text.clone() });
            let elaborated = elaborate_items(resolver, &invert.items(), &inner, scope);
            // Inversion is a payload map (course correction §13). A note
            // whose mirror image is unspellable is reported where it is
            // written and left alone, so one impossible note does not take
            // the rest of the phrase with it.
            let refused = std::cell::RefCell::new(Vec::new());
            let inverted = elaborated.map_payload(|payload| {
                payload.inverted(axis).unwrap_or_else(|| {
                    if let Some(pitch) = payload.pitch_of() {
                        refused.borrow_mut().push((pitch, payload.origin.definition_span));
                    }
                    payload.clone()
                })
            });
            for (pitch, at) in refused.into_inner() {
                resolver.error(
                    format!("`{pitch}` inverted around `{text}` needs more than a double accidental"),
                    at,
                );
            }
            inverted
        }
        VoiceItem::Dynamic(dynamic) => {
            let text = dynamic.mark().unwrap_or_default();
            let span = resolve::trimmed_span(dynamic.syntax());
            match DynamicMark::parse(&text) {
                Some(mark) => point(ScoreFact::new(scope, FactKind::Dynamic { mark }, origin_of(cx, span))),
                None => {
                    resolver.error(format!("unknown dynamic marking `{text}`"), span);
                    empty_segment()
                }
            }
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
fn resolve_scaled_duration(resolver: &mut Resolver, node: &SyntaxNode, cx: &ExpandCx) -> Option<NotatedDuration> {
    let duration = resolve::resolve_duration(resolver, node, cx)?;
    Some(if cx.scale == Ratio::ONE {
        duration
    } else {
        duration.scaled(cx.scale)
    })
}

/// Expand a `use` statement, mirroring the direct lowerer's binding rules.
fn elaborate_use(
    resolver: &mut Resolver,
    call: &musa_language::ast::UseStmt,
    cx: &ExpandCx,
    scope: Scope,
) -> Timeline<ScoreFact> {
    let name = call.motif().unwrap_or_default();
    let found = resolver
        .motifs
        .get_full(&name)
        .map(|(index, _, motif)| (index, motif.params.clone(), motif.body.clone(), motif.declaration));
    let Some((index, motif_params, body, declaration)) = found else {
        resolver.error(format!("unknown motif `{name}`"), resolve::span_of(call.syntax()));
        return empty_segment();
    };
    if index >= cx.max_motif {
        resolver.error(
            format!("motif `{name}` can only reference motifs declared before it"),
            resolve::span_of(call.syntax()),
        );
        return empty_segment();
    }
    let args = call.args();
    if args.len() > motif_params.len() {
        resolver.error(
            format!(
                "motif `{name}` takes {} arguments, got {}",
                motif_params.len(),
                args.len()
            ),
            resolve::span_of(call.syntax()),
        );
        return empty_segment();
    }
    let mut params = indexmap::IndexMap::new();
    for (position, param) in motif_params.iter().enumerate() {
        let text = args.get(position).cloned().or_else(|| param.default.clone());
        let Some(text) = text else {
            resolver.error(
                format!("motif `{name}`: missing argument `{}`", param.name),
                resolve::span_of(call.syntax()),
            );
            return empty_segment();
        };
        let Some(value) = resolve::bind_argument(resolver, &name, param, &text, cx, call.syntax()) else {
            return empty_segment();
        };
        params.insert(param.name.clone(), value);
    }
    let call_span = resolve::trimmed_span(call.syntax());
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
    let elaborated = elaborate_items(resolver, &body, &inner, scope);
    specialize(resolver, call, &elaborated)
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
    resolver: &mut Resolver,
    call: &musa_language::ast::UseStmt,
    elaborated: &Timeline<ScoreFact>,
) -> Timeline<ScoreFact> {
    let overrides = call.overrides();
    if overrides.is_empty() {
        return elaborated.clone();
    }
    // The positions of this occurrence: one entry per group of *event*
    // occurrences sharing a span, which is how a chord's pitches become one
    // position. A slur laid over the body is a fact, not a position, so a
    // motif that brackets its notes is still counted `note 1`, `note 2`.
    let mut positions: Vec<(usize, usize)> = Vec::new();
    for (index, occurrence) in elaborated.occurrences().iter().enumerate() {
        if !occurrence.payload().kind.is_event() {
            continue;
        }
        match positions.last_mut() {
            Some(&mut (start, ref mut end))
                if elaborated
                    .occurrences()
                    .get(start)
                    .is_some_and(|first| first.span() == occurrence.span())
                    && end.saturating_add(0) == index =>
            {
                *end = index.saturating_add(1);
            }
            _ => positions.push((index, index.saturating_add(1))),
        }
    }

    let mut replacements: indexmap::IndexMap<usize, (WrittenPitch, SourceSpan)> = indexmap::IndexMap::new();
    for each in &overrides {
        let at = resolve::trimmed_span(each.syntax());
        let Some(position) = each
            .position()
            .and_then(|text| text.parse::<usize>().ok())
            .filter(|n| *n > 0)
        else {
            resolver.error("a note override counts from `note 1`", at);
            continue;
        };
        let Some(pitch) = each.pitch().as_deref().and_then(WrittenPitch::parse) else {
            resolver.error("this override does not name a pitch", at);
            continue;
        };
        let Some(&(start, end)) = positions.get(position.saturating_sub(1)) else {
            resolver.error(
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
            resolver.error(
                format!("`note {position}` is a chord; an override respells one note"),
                at,
            );
            continue;
        }
        if elaborated
            .occurrences()
            .get(start)
            .and_then(|it| it.payload().pitch_of())
            .is_none()
        {
            resolver.error(format!("`note {position}` is a rest; an override respells a note"), at);
            continue;
        }
        if replacements.insert(start, (pitch, at)).is_some() {
            resolver.error(format!("`note {position}` is overridden twice"), at);
        }
    }
    if replacements.is_empty() {
        return elaborated.clone();
    }

    let occurrences: Vec<Occurrence<ScoreFact>> = elaborated
        .occurrences()
        .iter()
        .enumerate()
        .map(|(index, occurrence)| {
            let Some(&(pitch, at)) = replacements.get(&index) else {
                return occurrence.clone();
            };
            let mut payload = occurrence.payload().clone();
            if let FactKind::Note { pitch: written, .. } = &mut payload.kind {
                *written = pitch;
            }
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
/// a note, and reversing time does not move it. Ties need no repair: they
/// were merged when the enclosed items were elaborated, so what reverses is
/// an ordinary occurrence with an ordinary span.
fn reverse(timeline: &Timeline<ScoreFact>) -> Timeline<ScoreFact> {
    let extent = timeline.extent();
    let mut mirrored: Vec<Occurrence<ScoreFact>> = timeline
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
    // order the projection expects.
    mirrored.sort_by_key(|occurrence| (occurrence.span().start(), occurrence.span().end()));
    timeline_or_empty(extent, mirrored)
}

/// A timeline over `extent`, or the empty segment when the occurrences do
/// not fit it (unreachable for elaborated music; never a panic).
fn timeline_or_empty(extent: Beat, occurrences: Vec<Occurrence<ScoreFact>>) -> Timeline<ScoreFact> {
    timeline(extent, occurrences).unwrap_or_else(|_| musa_kernel::zero())
}

/// The empty segment `(0, ∅)` — contributes nothing to the sequence.
fn empty_segment() -> Timeline<ScoreFact> {
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
fn single(duration: &NotatedDuration, payload: ScoreFact) -> Timeline<ScoreFact> {
    let span = span_of_duration(duration);
    timeline(span.end(), vec![Occurrence::new(span, payload)]).unwrap_or_else(|_| musa_kernel::zero())
}

/// A region fact laid over the body it encloses: one occurrence spanning
/// `[0, extent)` of `body`, overlaid onto it.
///
/// The region's boundaries coincide with event boundaries by construction —
/// the extent *is* the extent of the items it encloses — which is the
/// invariant the projection relies on to name the events at its ends.
fn over(body: Timeline<ScoreFact>, fact: ScoreFact) -> Timeline<ScoreFact> {
    let extent = body.extent();
    let Ok(span) = Span::new(Beat::ZERO, extent) else {
        return body;
    };
    let region = timeline_or_empty(extent, vec![Occurrence::new(span, fact)]);
    overlay(vec![body, region])
}

/// A point fact at the cursor: an occurrence of zero extent in a segment of
/// zero extent, so sequencing places it exactly where it was written.
fn point(fact: ScoreFact) -> Timeline<ScoreFact> {
    timeline_or_empty(Beat::ZERO, vec![Occurrence::new(Span::ZERO, fact)])
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
    resolver: &mut Resolver,
    pitch: WrittenPitch,
    cx: &ExpandCx,
    node: &SyntaxNode,
) -> Option<WrittenPitch> {
    let mut current = pitch;
    for interval in &cx.intervals {
        let Some(next) = current.transpose(*interval) else {
            resolver.error(
                format!("transposition of `{current}` needs more than a double accidental"),
                resolve::span_of(node),
            );
            return None;
        };
        current = next;
    }
    Some(current)
}

/// Join tied noteheads into single occurrences (roadmap §6.3: a tie is
/// duration structure, not an annotation).
///
/// A tie says two written noteheads spell **one** sound, so this is where a
/// tie stops existing: the merged occurrence's span is the sum, its written
/// duration is the compound spelling, and nothing downstream ever sees a tie
/// flag. A tie onto a different pitch, or with nothing after it, is a
/// diagnostic here rather than a shape the projection has to cope with.
fn merge_ties(resolver: &mut Resolver, timeline: Timeline<ScoreFact>) -> Timeline<ScoreFact> {
    if !timeline.occurrences().iter().any(|it| it.payload().tied) {
        return timeline;
    }
    let extent = timeline.extent();
    let statements = statements(timeline.occurrences());
    let mut merged: Vec<Vec<Occurrence<ScoreFact>>> = Vec::with_capacity(statements.len());
    for statement in statements {
        let joins = merged
            .last()
            .and_then(|previous| previous.first())
            .is_some_and(|first| first.payload().tied);
        if !joins {
            merged.push(statement);
            continue;
        }
        let Some(previous) = merged.last_mut() else {
            merged.push(statement);
            continue;
        };
        if !same_sound(previous, &statement) {
            let at = statement
                .first()
                .map_or_else(|| SourceSpan::new(0, 0), |first| first.payload().origin.definition_span);
            resolver.error("a tie must be followed by the same pitch or chord", at);
            for occurrence in previous.iter_mut() {
                untie(occurrence);
            }
            merged.push(statement);
            continue;
        }
        join(previous, &statement);
    }
    let occurrences: Vec<Occurrence<ScoreFact>> = merged.into_iter().flatten().collect();
    timeline_or_empty(extent, occurrences)
}

/// The occurrences grouped into *statements*: one written note or rest, or
/// the pitches of one chord, which share a span and an origin.
///
/// Region and point facts are statements of one, and never merge: only a
/// notehead can be tied.
fn statements(occurrences: &[Occurrence<ScoreFact>]) -> Vec<Vec<Occurrence<ScoreFact>>> {
    let mut grouped: Vec<Vec<Occurrence<ScoreFact>>> = Vec::with_capacity(occurrences.len());
    for occurrence in occurrences {
        let joins = grouped.last().and_then(|group| group.first()).is_some_and(|first| {
            first.span() == occurrence.span()
                && first.payload().origin == occurrence.payload().origin
                && first.payload().pitch_of().is_some()
                && occurrence.payload().pitch_of().is_some()
        });
        match (joins, grouped.last_mut()) {
            (true, Some(group)) => group.push(occurrence.clone()),
            _ => grouped.push(vec![occurrence.clone()]),
        }
    }
    grouped
}

/// Whether two statements are the same sound: the same pitches, in order.
fn same_sound(left: &[Occurrence<ScoreFact>], right: &[Occurrence<ScoreFact>]) -> bool {
    let pitches = |statement: &[Occurrence<ScoreFact>]| -> Option<Vec<WrittenPitch>> {
        statement.iter().map(|it| it.payload().pitch_of()).collect()
    };
    match (pitches(left), pitches(right)) {
        (Some(left), Some(right)) => !left.is_empty() && left == right,
        _ => false,
    }
}

/// Extend `previous` through `statement`: one occurrence per pitch, spanning
/// both, spelled as the noteheads the composer wrote.
fn join(previous: &mut [Occurrence<ScoreFact>], statement: &[Occurrence<ScoreFact>]) {
    let Some(end) = statement.first().map(|first| first.span().end()) else {
        return;
    };
    let tied = statement.first().is_some_and(|first| first.payload().tied);
    for (index, occurrence) in previous.iter_mut().enumerate() {
        let mut fact = occurrence.payload().clone();
        fact.tied = tied;
        if let (
            FactKind::Note {
                duration,
                articulations,
                ..
            },
            Some(next),
        ) = (&mut fact.kind, statement.get(index).map(Occurrence::payload))
        {
            if let Some(added) = next.kind.duration_of() {
                *duration = duration.tied_to(added);
            }
            if let FactKind::Note {
                articulations: more, ..
            } = &next.kind
            {
                articulations.extend(more.iter().copied());
            }
        }
        let span = Span::new(occurrence.span().start(), end).unwrap_or_else(|_| occurrence.span());
        *occurrence = Occurrence::new(span, fact);
    }
}

/// A tie at the very end of a voice points at nothing.
///
/// It is only an error *here*: a tie at the end of a slur or a repeat block
/// continues into whatever follows the block, and merges at the level that
/// contains both sides. Reported once, and cleared, so no fact leaves
/// elaboration still claiming to be tied.
fn check_dangling_tie(resolver: &mut Resolver, timeline: Timeline<ScoreFact>) -> Timeline<ScoreFact> {
    if !timeline.occurrences().iter().any(|it| it.payload().tied) {
        return timeline;
    }
    let extent = timeline.extent();
    let mut occurrences: Vec<Occurrence<ScoreFact>> = timeline.occurrences().to_vec();
    let mut reported = false;
    for occurrence in &mut occurrences {
        if !occurrence.payload().tied {
            continue;
        }
        if !reported {
            resolver.error(
                "this tie has no note after it",
                occurrence.payload().origin.definition_span,
            );
            reported = true;
        }
        untie(occurrence);
    }
    timeline_or_empty(extent, occurrences)
}

/// Clear a tie that could not be honoured, so nothing downstream sees it.
fn untie(occurrence: &mut Occurrence<ScoreFact>) {
    let mut fact = occurrence.payload().clone();
    fact.tied = false;
    *occurrence = Occurrence::new(occurrence.span(), fact);
}

/// A tuplet has to be spellable, and a group split across a barline is not:
/// the notes on either side would need their own bracket and their own
/// ratio, which is a different piece of music from the one that was written.
fn check_tuplets(resolver: &mut Resolver, snapshot: &ScoreSnapshot) {
    let measure = snapshot.meter().measure_len().as_ratio();
    if measure == Ratio::ZERO {
        return;
    }
    let mut offenders = Vec::new();
    for tuplet in snapshot.annotations().tuplets() {
        let mut start = None;
        let mut end = None;
        for event in snapshot.events_in(tuplet.from, tuplet.to) {
            let event_end = event.onset + event.notated_duration.value;
            start = Some(start.map_or(event.onset, |current: MusicalTime| current.min(event.onset)));
            end = Some(end.map_or(event_end, |current: MusicalTime| current.max(event_end)));
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
        resolver.error("a tuplet must fit inside one measure", span);
    }
}

/// The normalized kernel text of a source's piece timeline, for golden
/// snapshots and semantic hashing (docs/kernel/05 N5–N6). `None` when the
/// source does not elaborate cleanly.
///
/// One timeline, not one per part: after prompt 40 a compilation has exactly
/// one temporal object, and the normal form is the text of that object —
/// key, meter, form markers and chord symbols included.
#[doc(hidden)]
pub fn kernel_normal_form(source: &SourceDocument) -> Option<String> {
    let document = musa_language::parse(source.text());
    if !document.errors().is_empty() {
        return None;
    }
    let piece = PieceDecl::from_root(&document.syntax())?;
    let mut resolver = Resolver::new();
    let mut snapshot = ScoreSnapshot::default();
    resolve::lower_header(&mut resolver, &piece, &mut snapshot);
    let score = piece.score()?;
    let mut lanes = Vec::new();
    for (part_index, part) in score.parts().iter().enumerate() {
        let part_id = u32::try_from(part_index).unwrap_or(u32::MAX);
        for (index, voice) in part.voices().iter().enumerate() {
            let voice_key = resolver.declare(crate::resolve::DeclInfo::Voice);
            let declaration = resolve::ordinal(&resolver, voice_key);
            lanes.push(elaborate_voice(
                &mut resolver,
                voice,
                declaration,
                part_id,
                u32::try_from(index).unwrap_or(u32::MAX),
            ));
        }
    }
    let music = overlay(lanes);
    let extent = music.extent();
    let context = context_facts(&mut resolver, &piece, &score, &mut snapshot, extent);
    Some(overlay(vec![music, context]).to_string())
}
