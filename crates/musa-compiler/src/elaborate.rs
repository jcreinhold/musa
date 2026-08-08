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
use crate::diagnose::{Code, Diagnostic};
use crate::origin::{ExpansionStep, Origin, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::resolve::{self, ExpandCx, Resolver};
use crate::score::{
    ArticulationMark, DynamicMark, MeterMap, Mode, NotatedDuration, Part, PartId, ScoreSnapshot, TempoChange, Voice,
    VoiceId,
};
use crate::time::MusicalTime;
use musa_kernel::{Beat, Occurrence, Span, Term, Timeline, sequence, timeline};
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
#[derive(Clone, Debug, PartialEq)]
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
    /// A hairpin over the region it spans, the mark it arrives at, and the
    /// shape of the growth. The shape is a kernel value (`Progress`), so it
    /// survives serialization and every consumer reads the same curve; how
    /// often to sample it is the consumer's policy (docs/kernel/07).
    Hairpin {
        grows: bool,
        target: DynamicMark,
        shape: musa_kernel::Progress,
    },
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
#[derive(Clone, Debug, PartialEq)]
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
            FactKind::Hairpin { grows, target, shape } => {
                format!(
                    "hairpin:{}:{}:{}|",
                    if *grows { "cres" } else { "dim" },
                    target.name(),
                    shape.canonical_key()
                )
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
        let span = SourceSpan::new(u32::from(range.start()), u32::from(range.end()));
        let mut diagnostic = Diagnostic::error(Code::Syntax, error.message()).at(span, error.label());
        if let Some(help) = error.help() {
            diagnostic = diagnostic.help(help);
        }
        if let Some((title, replacement)) = error.fix() {
            diagnostic = diagnostic.fix(title, span, replacement);
        }
        resolver.report(diagnostic);
    }
    if resolver
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::diagnose::Severity::Error)
    {
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }
    let Some(piece) = PieceDecl::from_root(&document.syntax()) else {
        resolver.report(
            Diagnostic::error(Code::Misplaced, "this file declares no piece")
                .at(SourceSpan::new(0, 0), "expected `piece \"…\" { … }`")
                .help("every musa file is one piece, or a `library { … }` for others to import"),
        );
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    };

    let mut snapshot = ScoreSnapshot::default();
    let libraries = crate::imports::load(resolver, name, &piece, &options.imports);
    elaborate_libraries(resolver, &libraries, &mut snapshot);
    resolve::lower_header(resolver, &piece, &mut snapshot);
    let mut identity = musa_kernel::SemanticHash::default();
    if let Some(score) = piece.score() {
        let context = elaborate_score(resolver, &piece, &score, &mut snapshot);
        identity = context.identity;
        elaborate_tempo_changes(resolver, &piece, &mut snapshot, &context);
    }
    snapshot.set_annotations(std::mem::take(&mut resolver.annotations));
    resolve::check_measure_sanity(resolver, &snapshot);
    check_tuplets(resolver, &snapshot);
    if resolver
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::diagnose::Severity::Error)
    {
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }
    let imported_studios: Vec<musa_language::ast::StudioDecl> =
        libraries.each().filter_map(|(_, library)| library.studio()).collect();
    let studio = resolve::lower_studio(resolver, &piece, &snapshot, &imported_studios);
    if resolver
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::diagnose::Severity::Error)
    {
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }
    Compilation::new(Some(snapshot), std::mem::take(&mut resolver.diagnostics))
        .with_studio(studio)
        .with_identity(identity)
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
    /// The semantic identity of the whole piece (docs/kernel/05 N6).
    identity: musa_kernel::SemanticHash,
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
    let mut lanes: Vec<Segment> = Vec::new();
    // One set of bindings for the whole piece: two voices calling the same
    // motif elaborate its body once between them.
    let mut share = Share::default();
    for part in score.parts() {
        let name = part.name().unwrap_or_default();
        let part_key = resolver.declare(crate::resolve::DeclInfo::Part);
        let _ = resolve::ordinal(resolver, part_key);
        if metadata.iter().any(|(_, existing, _)| *existing == name) {
            resolver.error(
                Code::DuplicateName,
                format!("this score already has a part called `{name}`"),
                resolve::span_of(part.syntax()),
                "declared again here",
            );
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
                    Code::DuplicateName,
                    format!("part `{name}` already has a voice called `{voice_name}`"),
                    resolve::span_of(voice.syntax()),
                    "declared again here",
                );
                continue;
            }
            let voice_id = VoiceId(u32::try_from(index).unwrap_or(u32::MAX));
            lanes.push(elaborate_voice(
                resolver,
                &mut share,
                voice,
                declaration,
                id.0,
                voice_id.0,
            ));
            names.insert(voice_id, voice_name);
        }
        voice_names.insert(id, names);
        metadata.push((id, name, clef));
    }

    // The overlay's extent without building the overlay: D3 says it is the
    // maximum of the parts', and the context facts need it before they exist.
    let extent = lanes
        .iter()
        .map(|lane| lane.extent)
        .max()
        .unwrap_or(musa_kernel::Beat::ZERO);
    let context = context_facts(resolver, piece, score, snapshot, extent);
    if let Some(sink) = &mut resolver.timeline_sink {
        // Measurement only, and the one place a voice is wanted on its own;
        // the piece itself is evaluated once, below.
        let voices: Vec<_> = lanes.iter().map(|lane| share.evaluate(lane.term.clone())).collect();
        sink.extend(voices);
    }
    // One close and one evaluation for the whole piece: closing wraps the live
    // bindings, and doing it per voice would clone every shared body once per
    // voice — which is the cost this prompt exists to remove.
    let parts: Vec<_> = lanes
        .into_iter()
        .map(|lane| lane.term)
        .chain(std::iter::once(Term::literal(context)))
        .collect();
    let whole = Term::over(parts).map_or_else(|_| musa_kernel::zero(), |term| share.evaluate(term));
    // The piece's identity, taken where the piece exists as one temporal
    // object and nowhere else: after this line the timeline is a projection,
    // and a hash of the projection would be a hash of a view.
    let identity = whole.semantic_hash();
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
        identity,
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
            Code::Misplaced,
            "this score already has a harmony lane",
            resolve::trimmed_span(extra.syntax()),
            "write every chord in the first one",
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
            resolver.error(
                Code::NotAValue,
                "a chord symbol is one word",
                span,
                "expected something like `am` or `fmaj7`",
            );
            continue;
        }
        let text = written.text();
        let Some(symbol) = crate::harmony::ChordSymbol::parse(&text) else {
            resolver.report(
                Diagnostic::error(Code::NotAValue, format!("`{text}` is not a chord symbol musa reads"))
                    .at(span, "unknown chord")
                    .note("a root, an optional quality, and an optional seventh: `am`, `fmaj7`, `g7`, `bdim`"),
            );
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
            resolver.report(
                Diagnostic::error(Code::Misplaced, "the tempo at `1:1` is the piece's starting tempo")
                    .at(span, "drop the `at 1:1`")
                    .help("write `tempo quarter = 72;` in the header"),
            );
            continue;
        }
        if changes.iter().any(|existing| existing.at == at) {
            resolver.error(
                Code::Misplaced,
                "two tempos at the same place",
                span,
                "the second of two",
            );
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
        resolver.error(
            Code::OutOfRange,
            "measures and beats count from `1:1`",
            span,
            "before the piece starts",
        );
        return None;
    }
    let measure_len = meter.measure_len().as_ratio();
    let beat_len = Ratio::new(1, i64::from(meter.denominator().max(1)));
    let at = MusicalTime::new(measure_len * (measure - 1) + beat_len * (beat - Ratio::ONE));
    if at >= extent && extent > MusicalTime::default() {
        resolver.error(
            Code::OutOfRange,
            format!("the piece ends before `{measure}:{beat_text}`"),
            span,
            "past the last note",
        );
        return None;
    }
    Some(at)
}

/// One elaborated voice item: the term that denotes it, how long it is, and
/// whether a tie is still open at its end.
///
/// The extent is tracked rather than computed, because a term's extent is
/// structural (`seq` sums, `over` maxes, a literal knows its own) and the
/// region facts — slur, phrase, hairpin, tuplet — need it *before* anything is
/// evaluated. Computing it by evaluating would defeat the point of emitting a
/// term at all.
struct Segment {
    term: Term<ScoreFact>,
    extent: Beat,
    /// A tie left open at this segment's end. Merging a tie needs two
    /// occurrences in one value, so a level that has one cannot stay a term.
    tied: bool,
}

impl Segment {
    fn literal(value: Timeline<ScoreFact>) -> Self {
        Self {
            extent: value.extent(),
            tied: value.occurrences().iter().any(|it| it.payload().tied),
            term: Term::literal(value),
        }
    }

    fn empty() -> Self {
        Self::literal(empty_segment())
    }
}

/// A body elaborated once and referenced many times, and the bindings that
/// hold them (course correction §19: "nothing requires duplicating thousands
/// of nodes merely to obey the normalized model").
///
/// Bindings are piece-level because their references are: two voices calling
/// the same motif with the same arguments share one body, and the `let` has to
/// dominate both. They are recorded in completion order — a motif body that
/// itself calls a motif finishes second — which is a valid dependency order by
/// construction, since a motif may only reference motifs declared before it.
#[derive(Default)]
struct Share {
    bindings: Vec<(String, Term<ScoreFact>)>,
    /// What has already been elaborated, keyed by everything its payloads
    /// depend on, mapping to the binding's name and extent.
    named: std::collections::HashMap<String, (String, Beat)>,
}

/// The placeholder a shared body carries where the call site would be.
///
/// A motif body's occurrences take their `source_span` from the *call*, which
/// is exactly what cannot be baked into a shared body. They carry this
/// instead, and each reference's mark says what to put there. No document is
/// four gigabytes, so it cannot collide with a real span.
const SHARED_ORIGIN: SourceSpan = SourceSpan::new(u32::MAX, u32::MAX);

/// The placeholder a shared body carries where the voice would be, for the
/// same reason: the same motif called from two voices is one body.
const SHARED_SCOPE: Scope = Scope::Voice {
    part: u32::MAX,
    voice: u32::MAX,
};

impl Share {
    /// `term` wrapped in the bindings it actually reaches, innermost first,
    /// so it is closed and can be evaluated.
    ///
    /// Bindings are recorded in completion order and a body may only reference
    /// bodies completed before it, so one reverse pass reaches every live
    /// name: a binding is kept exactly when what has been wrapped so far
    /// mentions it. Dropping the rest matters — a level that had to be
    /// evaluated spent its sharing, and its binding would otherwise be printed
    /// and cloned for nothing.
    fn close(&self, term: Term<ScoreFact>) -> Term<ScoreFact> {
        let mut closed = term;
        for (name, value) in self.bindings.iter().rev() {
            if closed.references_name(name) {
                closed = Term::bind(name, value.clone(), closed);
            }
        }
        closed
    }

    /// The value a term denotes, with marks honoured (T6).
    ///
    /// Called on the paths that genuinely need a value — a tie to merge, a
    /// retrograde to mirror, an inversion to map — and at the boundary, once
    /// per compilation. It takes the term **by value**: on material with no
    /// sharing the piece's term holds every occurrence exactly once, and
    /// cloning it to evaluate it would double the compiler's allocations for
    /// nothing.
    fn evaluate(&self, term: Term<ScoreFact>) -> Timeline<ScoreFact> {
        musa_kernel::evaluate_marked(self.close(term), instantiate)
    }

    /// The binding for `key`, or `None` if this body has not been elaborated.
    fn lookup(&self, key: &str) -> Option<(String, Beat)> {
        self.named.get(key).cloned()
    }

    /// Record an elaborated body under `key` and return its binding name.
    fn bind(&mut self, key: String, body: Segment) -> String {
        let name = self.bind_anonymous(body.term);
        self.named.insert(key, (name.clone(), body.extent));
        name
    }

    /// Record a body that nothing else can share and return its binding name.
    ///
    /// A repeat's body is written in exactly one place, so there is no key that
    /// another site could arrive with; keying it would only invite a false
    /// match between two bodies whose payloads differ.
    fn bind_anonymous(&mut self, body: Term<ScoreFact>) -> String {
        let name = format!("shared{}", self.bindings.len());
        self.bindings.push((name.clone(), body));
        name
    }
}

/// A reference's mark: `<depth>|<origin-span>|<scope>|<steps>`, with `-` for
/// the two that a given reference does not rewrite.
///
/// The steps come last so they are escaped once rather than twice: `depth`,
/// the span and the scope contain no `|`, so the reader splits three times and
/// takes the rest verbatim. Marks land in the interchange file, and a reader
/// counting backslashes is a reader who has stopped reading the music.
///
/// The mark is what makes sharing and provenance compatible
/// (`10-term-calculus.md` T6): the body is stated once, and each *use* says
/// how its instantiation differs. `depth` is where the steps belong in the
/// expansion path — appending would put a repeat's iteration index after the
/// steps of everything inside it, which is not where direct expansion puts
/// it. A repeat inserts one step at the body's own depth; a motif call inserts
/// the whole path leading to the call at depth zero, because its body was
/// elaborated with no path at all so that two call sites could share it.
fn mark_of(depth: usize, steps: &[ExpansionStep], origin: Option<SourceSpan>, scope: Option<Scope>) -> String {
    let origin = origin.map_or_else(|| "-".to_owned(), crate::factext::span_text);
    let scope = scope.map_or_else(|| "-".to_owned(), crate::factext::scope_text);
    let steps: Vec<_> = steps.iter().map(crate::factext::step_text).collect();
    format!("{depth}|{origin}|{scope}|{}", crate::factext::join(&steps, ','))
}

/// Apply a mark to a freshly instantiated body (E-Mark).
///
/// Payloads only, which is the whole of T6's contract: the spans, the extent,
/// the count and the order are the instantiated timeline's own and are not
/// touched here.
pub(crate) fn instantiate(mark: &str, instance: &mut Timeline<ScoreFact>) {
    let fields: Vec<_> = mark.splitn(4, '|').collect();
    let [depth, origin, scope, steps] = fields.as_slice() else {
        return;
    };
    let depth: usize = depth.parse().unwrap_or_default();
    let steps: Vec<_> = crate::factext::split_escaped(steps, ',')
        .iter()
        .filter_map(|step| crate::factext::read_step(step))
        .collect();
    let origin = crate::factext::read_span(origin);
    let scope = crate::factext::read_scope(scope);
    for payload in instance.payloads_mut() {
        let path = &mut payload.origin.expansion_path;
        let at = depth.min(path.len());
        path.splice(at..at, steps.iter().cloned());
        if let Some(span) = origin
            && payload.origin.source_span == SHARED_ORIGIN
        {
            payload.origin.source_span = span;
        }
        if let Some(scope) = scope
            && payload.scope == SHARED_SCOPE
        {
            payload.scope = scope;
        }
    }
}

/// A region fact — slur, phrase, hairpin, tuplet — laid over a body.
///
/// `overlay { body; timeline extent { occurrence fact from 0 to extent } }`,
/// in that order, which is the order the direct lowerer built and therefore
/// the order the storage-order goldens expect.
fn region(body: Segment, fact: ScoreFact) -> Segment {
    let extent = body.extent;
    let Ok(span) = Span::new(Beat::from_integer(0), extent) else {
        return body;
    };
    let Ok(mark) = timeline(extent, vec![Occurrence::new(span, fact)]) else {
        return body;
    };
    let Ok(term) = Term::over(vec![body.term, Term::literal(mark)]) else {
        return Segment::empty();
    };
    Segment {
        term,
        extent,
        tied: body.tied,
    }
}

/// Runs of adjacent literals folded into one literal each.
///
/// A `seq` of literals denotes exactly the literal their sequence denotes
/// (D2), so this changes no meaning — it changes what gets *printed*, which
/// is the point: eight plain notes in a row are eight literals of one
/// occurrence, and a file that shows them as eight nested `timeline` blocks
/// has buried the structure a composer wrote under structure they did not.
fn coalesce(terms: impl ExactSizeIterator<Item = Term<ScoreFact>>) -> Vec<Term<ScoreFact>> {
    let count = terms.len();
    let mut folded: Vec<Term<ScoreFact>> = Vec::with_capacity(count);
    let mut run: Vec<Timeline<ScoreFact>> = Vec::with_capacity(count);
    for term in terms {
        match term.into_literal() {
            Ok(value) => run.push(value),
            Err(term) => {
                flush(&mut run, &mut folded);
                folded.push(term);
            }
        }
    }
    flush(&mut run, &mut folded);
    folded
}

/// Push the pending run of literals, as one literal, and clear it.
fn flush(run: &mut Vec<Timeline<ScoreFact>>, folded: &mut Vec<Term<ScoreFact>>) {
    match run.len() {
        0 => {}
        1 => folded.extend(run.drain(..).map(Term::literal)),
        _ => folded.push(Term::literal(sequence(std::mem::take(run)))),
    }
}

/// The extent of a sequence of segments: exact rational addition (D2).
fn total_extent(segments: &[Segment]) -> Beat {
    Beat::new(
        segments
            .iter()
            .map(|segment| segment.extent.as_ratio())
            .fold(Ratio::ZERO, |total, next| total + next),
    )
}

/// Elaborate one voice: `sequence` of its items (docs/kernel/06).
fn elaborate_voice(
    resolver: &mut Resolver,
    share: &mut Share,
    voice: &musa_language::ast::VoiceDecl,
    declaration: crate::origin::DeclarationId,
    part: u32,
    voice_id: u32,
) -> Segment {
    let cx = ExpandCx {
        params: indexmap::IndexMap::new(),
        intervals: Vec::new(),
        path: Vec::new(),
        declaration,
        origin_span: None,
        max_motif: usize::MAX,
        scale: Ratio::ONE,
    };
    let segment = elaborate_items(
        resolver,
        share,
        &voice.items(),
        &cx,
        Scope::Voice { part, voice: voice_id },
    );
    if !segment.tied {
        return segment;
    }
    // A tie with nothing after it is a diagnostic, and reporting it means
    // untying the occurrence — a payload rewrite on a value. Only this case
    // collapses a voice's structure, and only because it is already an error.
    Segment::literal(check_dangling_tie(resolver, share.evaluate(segment.term)))
}

/// Elaborate voice items into a kernel timeline (sequence of item segments),
/// with tied noteheads merged.
///
/// Merging here rather than once per voice is what makes a tie invisible to
/// everything above: an inner block's ties are resolved before the block is
/// reversed or scaled, so `retrograde` mirrors ordinary occurrences and needs
/// no repair, and a tie that crosses a block boundary merges at the level
/// that contains both sides.
fn elaborate_items(
    resolver: &mut Resolver,
    share: &mut Share,
    items: &[VoiceItem],
    cx: &ExpandCx,
    scope: Scope,
) -> Segment {
    let mut segments = Vec::with_capacity(items.len());
    for item in items {
        segments.push(elaborate_item(resolver, share, item, cx, scope));
    }
    if segments.is_empty() {
        return Segment::empty();
    }
    if segments.iter().all(|segment| !segment.tied) {
        // The common case, and the one the whole change is for: nothing needs
        // a value, so nothing is evaluated and any sharing below survives
        // into the printed term.
        let extent = total_extent(&segments);
        let parts = coalesce(segments.into_iter().map(|segment| segment.term));
        let term = match <[_; 1]>::try_from(parts) {
            Ok([only]) => only,
            Err(parts) => Term::seq(parts).unwrap_or_else(|_| Term::literal(empty_segment())),
        };
        return Segment {
            term,
            extent,
            tied: false,
        };
    }
    // A tie crosses an item boundary here. Merging joins two occurrences into
    // one, which is neither a payload map nor anything a term can say, so this
    // level becomes a value. Evaluating a reference instantiates its body, so
    // the result is byte-identical to elaborating everything expanded — the
    // sharing below is spent rather than lost, and the binding it made is
    // pruned when the piece's term is closed.
    let values = segments
        .into_iter()
        .map(|segment| share.evaluate(segment.term))
        .collect();
    Segment::literal(merge_ties(resolver, sequence(values)))
}

/// The articulations written on a note or chord statement.
fn articulations_of(resolver: &mut Resolver, names: &[String], span: SourceSpan) -> Vec<ArticulationMark> {
    let mut articulations = Vec::new();
    for name in names {
        match ArticulationMark::parse(name) {
            Some(mark) => articulations.push(mark),
            None => resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("`{name}` is not an articulation"))
                    .at(span, "unknown articulation")
                    .help(crate::resolve::suggest(name, ArticulationMark::NAMES, "articulations")),
            ),
        }
    }
    articulations
}

/// Elaborate one item; malformed items elaborate to the empty segment
/// `(0, ∅)` — the direct lowerer's `continue` (diagnostic already emitted).
fn elaborate_item(
    resolver: &mut Resolver,
    share: &mut Share,
    item: &VoiceItem,
    cx: &ExpandCx,
    scope: Scope,
) -> Segment {
    match item {
        VoiceItem::Note(note) => {
            let Some(duration) = resolve_scaled_duration(resolver, note.syntax(), cx) else {
                return Segment::empty();
            };
            let pitch_text = note.pitch().unwrap_or_default();
            let Some(pitch) = resolve::resolve_pitch(resolver, &pitch_text, note.syntax(), cx) else {
                return Segment::empty();
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
            Segment::literal(single(&duration, fact))
        }
        VoiceItem::Rest(rest) => {
            let Some(duration) = resolve_scaled_duration(resolver, rest.syntax(), cx) else {
                return Segment::empty();
            };
            let span = resolve::trimmed_span(rest.syntax());
            let origin = origin_of(cx, span);
            Segment::literal(single(
                &duration,
                ScoreFact::new(
                    scope,
                    FactKind::Rest {
                        duration: duration.clone(),
                        articulations: Vec::new(),
                    },
                    origin,
                ),
            ))
        }
        VoiceItem::Chord(chord) => {
            let Some(duration) = resolve_scaled_duration(resolver, chord.syntax(), cx) else {
                return Segment::empty();
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
                        resolver.report(
                            Diagnostic::error(Code::NotAValue, format!("`{text}` is not a pitch"))
                                .at(resolve::trimmed_span(chord.syntax()), "inside this chord")
                                .note("a pitch is a letter, an optional `s` or `f`, and an octave: `c4`, `gs5`, `bf3`"),
                        );
                    }
                }
            }
            let span = span_of_duration(&duration);
            Segment::literal(
                timeline(
                    span.end(),
                    pitches
                        .into_iter()
                        .map(|payload| Occurrence::new(span, payload))
                        .collect(),
                )
                .unwrap_or_else(|_| musa_kernel::zero()),
            )
        }
        VoiceItem::Use(call) => elaborate_use(resolver, share, call, cx, scope),
        VoiceItem::Transpose(transpose) => {
            let text = transpose.interval().unwrap_or_default();
            let Some(interval) = crate::origin::Interval::parse(&text, transpose.is_down()) else {
                resolver.report(
                    Diagnostic::error(Code::NotAValue, format!("`{text}` is not an interval"))
                        .at(resolve::trimmed_span(transpose.syntax()), "unknown interval")
                        .note("a quality and a number: `P5`, `M3`, `m6`, `A4`, `d5`"),
                );
                return Segment::empty();
            };
            let mut inner = cx.clone();
            inner.intervals.push(interval);
            inner.path.push(ExpansionStep::Transposition(interval));
            elaborate_items(resolver, share, &transpose.items(), &inner, scope)
        }
        VoiceItem::Repeat(repeat) => {
            let count: u32 = repeat.count().and_then(|text| text.parse().ok()).unwrap_or(0);
            if count == 0 {
                return Segment::empty();
            }
            // The body once, then a reference per iteration. What separates
            // the iterations is one expansion step, and the *reference*
            // carries it (T6) — which is the only reason the body can be
            // stated once at all.
            let body = elaborate_items(resolver, share, &repeat.items(), cx, scope);
            if body.tied {
                // A tie open at the body's end would join into the next
                // iteration's first note, and merging needs both in one
                // value. Rare, and expanding is exactly what happened before.
                let value = share.evaluate(body.term);
                let mut segments = Vec::with_capacity(count as usize);
                for iteration in 0..count {
                    let mut copy = value.clone();
                    instantiate(
                        &mark_of(cx.path.len(), &[ExpansionStep::RepeatIteration(iteration)], None, None),
                        &mut copy,
                    );
                    segments.push(copy);
                }
                return Segment::literal(sequence(segments));
            }
            let once = body.extent;
            let name = share.bind_anonymous(body.term);
            let references = (0..count)
                .map(|iteration| {
                    Term::var_marked(
                        &name,
                        mark_of(cx.path.len(), &[ExpansionStep::RepeatIteration(iteration)], None, None),
                    )
                })
                .collect();
            Segment {
                term: Term::seq(references).unwrap_or_else(|_| Term::literal(empty_segment())),
                extent: Beat::new(once.as_ratio() * i64::from(count)),
                tied: false,
            }
        }
        VoiceItem::Slur(slur) => {
            let origin = origin_of(cx, resolve::trimmed_span(slur.syntax()));
            let body = elaborate_items(resolver, share, &slur.items(), cx, scope);
            region(body, ScoreFact::new(scope, FactKind::Slur, origin))
        }
        VoiceItem::Phrase(phrase) => {
            let origin = origin_of(cx, resolve::trimmed_span(phrase.syntax()));
            let name = phrase.name().unwrap_or_default();
            let body = elaborate_items(resolver, share, &phrase.items(), cx, scope);
            region(body, ScoreFact::new(scope, FactKind::Phrase { name }, origin))
        }
        VoiceItem::Hairpin(hairpin) => {
            let span = resolve::trimmed_span(hairpin.syntax());
            let text = hairpin.target().unwrap_or_default();
            let Some(target) = DynamicMark::parse(&text) else {
                resolver.report(
                    Diagnostic::error(Code::UnknownWord, format!("`{text}` is not a dynamic marking"))
                        .at(span, "unknown marking")
                        .help(crate::resolve::suggest(&text, DynamicMark::NAMES, "markings")),
                );
                return Segment::empty();
            };
            let origin = origin_of(cx, span);
            let grows = hairpin.grows();
            let body = elaborate_items(resolver, share, &hairpin.items(), cx, scope);
            // The grammar writes `cres.`/`dim.` and nothing about shape, so
            // every hairpin is a straight line today. The value is in the
            // timeline rather than invented during lowering, which is what
            // lets a second implementation sound the same (§32 Q4).
            let shape = musa_kernel::Progress::linear();
            region(
                body,
                ScoreFact::new(scope, FactKind::Hairpin { grows, target, shape }, origin),
            )
        }
        VoiceItem::Tuplet(tuplet) => {
            let text = tuplet.ratio().unwrap_or_default();
            let span = resolve::trimmed_span(tuplet.syntax());
            let Some((num, den)) = parse_tuplet_ratio(&text) else {
                resolver.error(
                    Code::NotAValue,
                    format!("`{text}` is not a tuplet ratio"),
                    span,
                    "expected something like `3/2`",
                );
                return Segment::empty();
            };
            let origin = origin_of(cx, span);
            let mut inner = cx.clone();
            inner.scale = cx.scale * Ratio::new(i64::from(den), i64::from(num));
            let body = elaborate_items(resolver, share, &tuplet.items(), &inner, scope);
            region(body, ScoreFact::new(scope, FactKind::Tuplet { num, den }, origin))
        }
        VoiceItem::Stretch(stretch) => {
            let text = stretch.factor().unwrap_or_default();
            let span = resolve::trimmed_span(stretch.syntax());
            let factor = resolve::parse_ratio(&text).or_else(|| text.parse::<i64>().ok().map(Ratio::from_integer));
            let Some(factor) = factor.filter(|factor| *factor > Ratio::ZERO) else {
                resolver.error(
                    Code::NotAValue,
                    format!("`{text}` is not a stretch factor"),
                    span,
                    "expected a positive number, like `2` or `3/2`",
                );
                return Segment::empty();
            };
            let mut inner = cx.clone();
            inner.path.push(ExpansionStep::Stretch(factor));
            let segment = elaborate_items(resolver, share, &stretch.items(), &inner, scope);
            let elaborated = share.evaluate(segment.term);
            // The kernel's time-scaling action (course correction §14) plus
            // the matching renotation: a stretched quarter is *written* as a
            // half, not as a quarter that lasts twice as long.
            Segment::literal(
                elaborated
                    .map_payload(|payload| payload.stretched(factor))
                    .scale(factor)
                    .unwrap_or_else(|_| empty_segment()),
            )
        }
        VoiceItem::Retrograde(retrograde) => {
            let mut inner = cx.clone();
            inner.path.push(ExpansionStep::Retrograde);
            let segment = elaborate_items(resolver, share, &retrograde.items(), &inner, scope);
            let elaborated = share.evaluate(segment.term);
            Segment::literal(reverse(&elaborated))
        }
        VoiceItem::Invert(invert) => {
            let text = invert.axis().unwrap_or_default();
            let span = resolve::trimmed_span(invert.syntax());
            let Some(axis) = WrittenPitch::parse(&text) else {
                resolver.error(
                    Code::NotAValue,
                    format!("`{text}` is not a pitch"),
                    span,
                    "inversion needs a pitch to mirror about",
                );
                return Segment::empty();
            };
            let mut inner = cx.clone();
            inner.path.push(ExpansionStep::Inversion { axis: text.clone() });
            let segment = elaborate_items(resolver, share, &invert.items(), &inner, scope);
            let elaborated = share.evaluate(segment.term);
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
                resolver.report(
                    Diagnostic::error(
                        Code::OutOfRange,
                        format!("`{pitch}` cannot be spelled when mirrored around `{text}`"),
                    )
                    .at(at, "would need a triple accidental")
                    .help("mirror around a different pitch, or write the passage out"),
                );
            }
            Segment::literal(inverted)
        }
        VoiceItem::Dynamic(dynamic) => {
            let text = dynamic.mark().unwrap_or_default();
            let span = resolve::trimmed_span(dynamic.syntax());
            match DynamicMark::parse(&text) {
                Some(mark) => Segment::literal(point(ScoreFact::new(
                    scope,
                    FactKind::Dynamic { mark },
                    origin_of(cx, span),
                ))),
                None => {
                    resolver.report(
                        Diagnostic::error(Code::UnknownWord, format!("`{text}` is not a dynamic marking"))
                            .at(span, "unknown marking")
                            .help(crate::resolve::suggest(&text, DynamicMark::NAMES, "markings")),
                    );
                    Segment::empty()
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
    share: &mut Share,
    call: &musa_language::ast::UseStmt,
    cx: &ExpandCx,
    scope: Scope,
) -> Segment {
    let name = call.motif().unwrap_or_default();
    let found = resolver
        .motifs
        .get_full(&name)
        .map(|(index, _, motif)| (index, motif.params.clone(), motif.body.clone(), motif.declaration));
    let Some((index, motif_params, body, declaration)) = found else {
        let known: Vec<&str> = resolver.motifs.keys().map(String::as_str).collect();
        resolver.report(
            Diagnostic::error(Code::UnknownName, format!("cannot find motif `{name}`"))
                .at(resolve::trimmed_span(call.syntax()), "not declared in this piece")
                .help(crate::resolve::suggest(&name, &known, "motifs")),
        );
        return Segment::empty();
    };
    if index >= cx.max_motif {
        resolver.report(
            Diagnostic::error(Code::Misplaced, format!("motif `{name}` is declared after this one"))
                .at(resolve::trimmed_span(call.syntax()), "used before it exists")
                .help("move the declaration above the motif that uses it")
                .note("a motif sees only the motifs above it, which is what makes a cycle impossible"),
        );
        return Segment::empty();
    }
    let args = call.args();
    if args.len() > motif_params.len() {
        resolver.report(
            Diagnostic::error(
                Code::NotAValue,
                format!(
                    "motif `{name}` takes {} argument{}, and this passes {}",
                    motif_params.len(),
                    if motif_params.len() == 1 { "" } else { "s" },
                    args.len()
                ),
            )
            .at(resolve::trimmed_span(call.syntax()), "too many arguments"),
        );
        return Segment::empty();
    }
    let mut params = indexmap::IndexMap::new();
    for (position, param) in motif_params.iter().enumerate() {
        let text = args.get(position).cloned().or_else(|| param.default.clone());
        let Some(text) = text else {
            resolver.report(
                Diagnostic::error(
                    Code::NotAValue,
                    format!("motif `{name}` needs a value for `{}`", param.name),
                )
                .at(
                    resolve::trimmed_span(call.syntax()),
                    format!("no `{}` here", param.name),
                )
                .help(format!(
                    "pass one, or give `{}` a default in the declaration",
                    param.name
                )),
            );
            return Segment::empty();
        };
        let Some(value) = resolve::bind_argument(resolver, &name, param, &text, cx, call.syntax()) else {
            return Segment::empty();
        };
        params.insert(param.name.clone(), value);
    }
    let call_span = resolve::trimmed_span(call.syntax());
    // The body is elaborated *without* the path that leads to this call, and
    // with placeholders where the call site and the voice would be, so that a
    // second call site can reach the same body. Everything a call contributes
    // rides on the reference's mark instead (T6).
    let inner = ExpandCx {
        params,
        intervals: cx.intervals.clone(),
        path: Vec::new(),
        declaration,
        origin_span: Some(SHARED_ORIGIN),
        max_motif: index,
        scale: cx.scale,
    };
    let key = motif_key(&name, &inner);
    let (binding, extent) = match share.lookup(&key) {
        Some(found) => found,
        None => {
            let elaborated = elaborate_items(resolver, share, &body, &inner, SHARED_SCOPE);
            let extent = elaborated.extent;
            (share.bind(key, elaborated), extent)
        }
    };
    let steps = cx
        .path
        .iter()
        .cloned()
        .chain(std::iter::once(ExpansionStep::MotifApplication {
            call_site: call_span,
        }))
        .collect::<Vec<_>>();
    let reference = Term::var_marked(binding, mark_of(0, &steps, Some(call_span), Some(scope)));
    if call.overrides().is_empty() {
        return Segment {
            term: reference,
            extent,
            tied: false,
        };
    }
    // Overrides rewrite named notes of *this* call, which needs the notes.
    Segment::literal(specialize(resolver, call, &share.evaluate(reference)))
}

/// The sharing key for a motif body: the motif, and everything its payloads
/// depend on that is not supplied by the reference's mark.
fn motif_key(name: &str, inner: &ExpandCx) -> String {
    use std::fmt::Write as _;
    let mut key = format!("{name}|{}|", inner.scale);
    for interval in &inner.intervals {
        let _ = write!(key, "{interval:?},");
    }
    key.push('|');
    for (param, value) in &inner.params {
        let _ = write!(key, "{param}={value:?};");
    }
    key
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
            resolver.error(
                Code::OutOfRange,
                "notes are counted from `note 1`",
                at,
                "there is no note 0",
            );
            continue;
        };
        let Some(pitch) = each.pitch().as_deref().and_then(WrittenPitch::parse) else {
            resolver.error(Code::NotAValue, "this override names no pitch", at, "expected a pitch");
            continue;
        };
        let Some(&(start, end)) = positions.get(position.saturating_sub(1)) else {
            resolver.error(
                Code::OutOfRange,
                format!(
                    "this occurrence has {} note{}",
                    positions.len(),
                    if positions.len() == 1 { "" } else { "s" }
                ),
                at,
                format!("so there is no note {position}"),
            );
            continue;
        };
        if end.saturating_sub(start) > 1 {
            resolver.error(
                Code::Misplaced,
                format!("note {position} is a chord"),
                at,
                "an override respells one note, not a chord",
            );
            continue;
        }
        if elaborated
            .occurrences()
            .get(start)
            .and_then(|it| it.payload().pitch_of())
            .is_none()
        {
            resolver.error(
                Code::Misplaced,
                format!("note {position} is a rest"),
                at,
                "a rest has no pitch to respell",
            );
            continue;
        }
        if replacements.insert(start, (pitch, at)).is_some() {
            resolver.error(
                Code::DuplicateName,
                format!("note {position} is overridden twice"),
                at,
                "the second of two",
            );
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
            resolver.report(
                Diagnostic::error(
                    Code::OutOfRange,
                    format!("`{current}` cannot be spelled after this transposition"),
                )
                .at(resolve::trimmed_span(node), "would need a triple accidental")
                .help("transpose by a different interval, or write the passage out"),
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
            resolver.error(
                Code::Misplaced,
                "a tie joins two of the same note",
                at,
                "the next note is a different pitch",
            );
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
                Code::Misplaced,
                "this tie has nothing to tie to",
                occurrence.payload().origin.definition_span,
                "no note follows it",
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
        resolver.error(
            Code::DoesNotAddUp,
            "this tuplet is longer than a measure",
            span,
            "spills past the barline",
        );
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
    let (_, term) = piece_term(source)?;
    Some(musa_kernel::evaluate_marked(term, instantiate).to_string())
}

/// The piece as a **term** (docs/kernel/10): its name, and an `over` of one
/// literal per voice plus one for the piece-wide context.
///
/// The shared bodies are `let`-bound around the whole `over`, because a motif
/// called from two voices is one body and its binding has to dominate both.
/// Bindings nothing references — a level that had to be evaluated spent its
/// sharing — are dropped, so the printed term names only what it uses.
pub(crate) fn piece_term(source: &SourceDocument) -> Option<(String, musa_kernel::Term<ScoreFact>)> {
    let document = musa_language::parse(source.text());
    if !document.errors().is_empty() {
        return None;
    }
    let piece = PieceDecl::from_root(&document.syntax())?;
    let mut resolver = Resolver::new();
    let mut snapshot = ScoreSnapshot::default();
    resolve::lower_header(&mut resolver, &piece, &mut snapshot);
    let score = piece.score()?;
    let mut share = Share::default();
    let mut lanes = Vec::new();
    for (part_index, part) in score.parts().iter().enumerate() {
        let part_id = u32::try_from(part_index).unwrap_or(u32::MAX);
        for (index, voice) in part.voices().iter().enumerate() {
            let voice_key = resolver.declare(crate::resolve::DeclInfo::Voice);
            let declaration = resolve::ordinal(&resolver, voice_key);
            lanes.push(elaborate_voice(
                &mut resolver,
                &mut share,
                voice,
                declaration,
                part_id,
                u32::try_from(index).unwrap_or(u32::MAX),
            ));
        }
    }
    // The overlay's extent without building the overlay: D3 says it is the
    // maximum of the parts', and the context facts need it before they exist.
    let extent = lanes
        .iter()
        .map(|lane| lane.extent)
        .max()
        .unwrap_or(musa_kernel::Beat::ZERO);
    let context = context_facts(&mut resolver, &piece, &score, &mut snapshot, extent);
    let parts: Vec<musa_kernel::Term<ScoreFact>> = lanes
        .into_iter()
        .map(|lane| lane.term)
        .chain(std::iter::once(musa_kernel::Term::literal(context)))
        .collect();
    let term = share.close(musa_kernel::Term::over(parts).ok()?);
    Some((piece.name().unwrap_or_default(), term))
}
