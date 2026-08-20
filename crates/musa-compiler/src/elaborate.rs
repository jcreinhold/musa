//! Elaboration of the surface language through the temporal kernel
//! (docs/rules/kernel/06-surface-elaboration.md, docs/rules/kernel/05-normalization.md).
//!
//! This is *the* semantic path: name resolution, motif registration and unit
//! checks come from `resolve.rs`, voice content elaborates into
//! `VoiceTrack` values built from kernel `follow`/`together`, and
//! `project.rs` reads a `ScoreSnapshot` back out of the result (§27).
//!
//! Design decisions recorded in docs/rules/kernel/06 and 08:
//! - a `rest` statement elaborates to a `Rest` payload occurrence — notation
//!   intent, a typed fact; the kernel has no silence object (§2);
//! - transposition applies eagerly during elaboration via the shared
//!   interval stack (§19 evaluation strategy); semantically it is a payload
//!   map (§13) and composition is commutative, so eager application is
//!   observably equal by the composition law);
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
use crate::origin::{Origin, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::resolve::{self, Resolver};
use crate::scope::Scope;
use crate::score::{DynamicMark, Meter, Mode, NotatedDuration, Part, PartId, ScoreSnapshot, Voice, VoiceId};
use crate::time::MusicalTime;
use musa_kernel::{Duration, EventTrack, Occurrence, Position, Span, WrittenTime, empty, track};
use musa_language::SyntaxNode;
use musa_language::ast::{AstNode as _, PieceDecl};
use num_rational::Ratio;
use std::fmt::Write as _;

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
        articulations: Vec<crate::Mark>,
        /// The freedom written on this note, when it was given one.
        free: Option<crate::score::FreeDuration>,
    },
    /// A written rest — notation intent, not a silence object (§2).
    Rest {
        duration: NotatedDuration,
        articulations: Vec<crate::Mark>,
        /// As [`FactKind::Note`]'s: a rest can be held too.
        free: Option<crate::score::FreeDuration>,
    },
    /// A notation mark that is not written on a note: a point at the instant
    /// it is written, or a span over the music its block covers. Which of the
    /// two this occurrence is, is its span — a point's is empty.
    ///
    /// Note-anchored marks are *not* here: a staccato dot has no extent and no
    /// identity of its own, so it stays a field of the note (see this enum's
    /// own doc). A pedal has both.
    Mark {
        mark: crate::Mark,
        argument: Option<crate::marks::MarkArgument>,
    },
    /// One grace note, as a **point** occurrence at the principal note's
    /// onset: a written pitch with no written duration.
    ///
    /// Not a [`FactKind::Mark`], by this enum's own rule. A grace note has its
    /// own pitch, its own accidental, its own beam and its own slur to the
    /// note it leans on, and four of them stand in an order that matters —
    /// that is an identity. A mark whose payload was a list of pitches would
    /// be a note under another name.
    ///
    /// What time it steals is *not* here: that is a reading, and readings
    /// belong to the profile (§2 — notated duration ≠ performed duration).
    Grace {
        pitch: WrittenPitch,
        articulations: Vec<crate::Mark>,
        /// Where this grace note stands among the ones written with it.
        ///
        /// Load-bearing rather than decorative. N2 orders occurrences by
        /// `(start, end, payload key)`, and every grace note in one group
        /// shares a start and an end — so without the index in the payload,
        /// `grace { c5 d5 }` and `grace { d5 c5 }` normalize to the same
        /// timeline and the kernel calls them equal music. They are not. The
        /// alternative, giving them nonzero written durations so they sort,
        /// would put performed time into the notation.
        index: u8,
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
    /// often to sample it is the consumer's policy (docs/rules/kernel/07).
    Hairpin {
        grows: bool,
        target: DynamicMark,
        shape: musa_kernel::Progress,
    },
    /// The key signature, over the region it governs — the whole piece
    /// while the grammar has no `modulate`.
    Key { tonic: PitchClass, mode: Mode },
    /// The meter, over the region it governs — likewise the whole piece.
    Meter { numerator: u32, denominator: u32 },
    /// The clef a staff is read in, over the region it governs.
    Clef { clef: crate::Clef },
    /// The tempo *marking*, over the region it governs.
    ///
    /// The marking, not the map. `♩ = 92` is notation written at a place —
    /// the engraver prints it, the exporters carry it — and the written-time
    /// → second function performance integrates is *derived* from the markings
    /// (docs/rules/kernel/06-surface-elaboration.md). Keeping the two apart is why this is a fact:
    /// a fact has a place in the piece, and a function does not.
    ///
    /// Both halves are optional and neither implies the other. `tempo
    /// "Andante";` prints a word and changes no clock — which is what most
    /// tempo markings in most scores do — and a metronome mark with no word
    /// is the common modern case.
    Tempo {
        /// The metronome mark, when the marking states one.
        metronome: Option<crate::score::Metronome>,
        /// The word printed with it, when the marking states one.
        text: Option<String>,
        /// How it gets somewhere else, when the change is gradual.
        ramp: Option<crate::score::Ramp>,
    },
    /// A form marker at the place it names.
    Section { name: String },
    /// A chord symbol at the place it is written; a region once a chord's
    /// duration can be written.
    Harmony { symbol: crate::harmony::ChordSymbol },
    /// A repeat over every pass it plays. The page prints the body once
    /// between repeat barlines; the timeline holds all `times` of it, which is
    /// the layer table's own example (roadmap §2).
    Repeat {
        times: u32,
        /// The passes the *source* asked for, when it left the count open:
        /// `repeat 4 to 16` is `Some((4, 16))` and `times` is the reading this
        /// performance took. The page prints the range and plays the count.
        range: Option<(u32, u32)>,
    },
    /// A mobile over the region it plays: the fragments as written, and the
    /// order this performance chose. The realized music is the fragments'
    /// own occurrences; this is the instruction the page prints over them
    /// (the rule the mobile and open-form facts share).
    Mobile { fragments: Vec<String>, order: Vec<u32> },
    /// An improvised frame over the region it occupies. It sounds as silence,
    /// because musa does not improvise.
    Improvise { over: Option<String> },
    /// One ending, over the region one pass of it plays.
    ///
    /// `bracket` is which volta is printed — the page has one per distinct
    /// ending — and `pass` is which time through it sounds. They differ
    /// whenever there are fewer endings than passes, which is what a bracket
    /// labelled `2.–4.` means.
    Ending { bracket: u32, pass: u32 },
}

impl FactKind {
    /// Whether this fact is a note or a rest — the facts that become events
    /// and are given identities.
    pub(crate) fn is_event(&self) -> bool {
        matches!(self, Self::Note { .. } | Self::Rest { .. })
    }

    /// The articulations written on this fact, if any.
    pub(crate) fn articulations_of(&self) -> &[crate::Mark] {
        match self {
            Self::Note { articulations, .. } | Self::Rest { articulations, .. } => articulations,
            Self::Mark { .. }
            | Self::Grace { .. }
            | Self::Slur
            | Self::Phrase { .. }
            | Self::Tuplet { .. }
            | Self::Dynamic { .. }
            | Self::Hairpin { .. }
            | Self::Key { .. }
            | Self::Meter { .. }
            | Self::Clef { .. }
            | Self::Tempo { .. }
            | Self::Section { .. }
            | Self::Harmony { .. }
            | Self::Repeat { .. }
            | Self::Mobile { .. }
            | Self::Improvise { .. }
            | Self::Ending { .. } => &[],
        }
    }

    /// The written duration, for the facts that have one.
    pub(crate) fn duration_of(&self) -> Option<&NotatedDuration> {
        match self {
            Self::Note { duration, .. } | Self::Rest { duration, .. } => Some(duration),
            Self::Mark { .. }
            | Self::Grace { .. }
            | Self::Slur
            | Self::Phrase { .. }
            | Self::Tuplet { .. }
            | Self::Dynamic { .. }
            | Self::Hairpin { .. }
            | Self::Key { .. }
            | Self::Meter { .. }
            | Self::Clef { .. }
            | Self::Tempo { .. }
            | Self::Section { .. }
            | Self::Harmony { .. }
            | Self::Repeat { .. }
            | Self::Mobile { .. }
            | Self::Improvise { .. }
            | Self::Ending { .. } => None,
        }
    }

    /// The bounds of a freely-held note, for the facts that have them.
    pub(crate) fn free_of(&self) -> Option<&crate::score::FreeDuration> {
        match self {
            Self::Note { free, .. } | Self::Rest { free, .. } => free.as_ref(),
            Self::Mark { .. }
            | Self::Grace { .. }
            | Self::Slur
            | Self::Phrase { .. }
            | Self::Tuplet { .. }
            | Self::Dynamic { .. }
            | Self::Hairpin { .. }
            | Self::Key { .. }
            | Self::Meter { .. }
            | Self::Clef { .. }
            | Self::Tempo { .. }
            | Self::Section { .. }
            | Self::Harmony { .. }
            | Self::Repeat { .. }
            | Self::Mobile { .. }
            | Self::Improvise { .. }
            | Self::Ending { .. } => None,
        }
    }
}

/// One elaborated fact of a score: what is stated, where in the score's
/// structure it belongs, and why it exists (docs/rules/kernel/06).
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
    pub(crate) fn stretched(&self, factor: Ratio<i64>) -> Self {
        let mut stretched = self.clone();
        match &mut stretched.kind {
            FactKind::Note { duration, .. } | FactKind::Rest { duration, .. } => {
                *duration = duration.stretched(factor);
            }
            FactKind::Mark { .. }
            | FactKind::Grace { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Clef { .. }
            | FactKind::Tempo { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. }
            | FactKind::Repeat { .. }
            | FactKind::Mobile { .. }
            | FactKind::Improvise { .. }
            | FactKind::Ending { .. } => {}
        }
        stretched
    }

    /// The same fact with its pitch raised by `interval`, or `None` only when a
    /// fixed-width storage coordinate would overflow.
    ///
    /// Beside [`Self::inverted`] rather than inside the fold, because the
    /// contextual path transposes by rebasing an [`ExpandCx`] and the track
    /// builtin registered in [`crate::registry::track`] has no context to
    /// rebase: it holds facts that already have written pitches. A non-note
    /// fact transposes to itself, which is the one thing both readings agree on
    /// and is why this is a method rather than a match at each caller.
    pub(crate) fn transposed(&self, interval: crate::Interval) -> Option<Self> {
        let FactKind::Note { pitch, .. } = &self.kind else {
            return Some(self.clone());
        };
        let raised = pitch.transpose(interval)?;
        let mut transposed = self.clone();
        if let FactKind::Note { pitch, .. } = &mut transposed.kind {
            *pitch = raised;
        }
        Some(transposed)
    }

    /// The same fact with its pitch mirrored about `axis`, or `None` only
    /// when a fixed-width storage coordinate would overflow.
    pub(crate) fn inverted(&self, axis: WrittenPitch) -> Option<Self> {
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
            | FactKind::Mark { .. }
            | FactKind::Grace { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Clef { .. }
            | FactKind::Tempo { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. }
            | FactKind::Repeat { .. }
            | FactKind::Ending { .. }
            | FactKind::Mobile { .. }
            | FactKind::Improvise { .. } => None,
        }
    }

    /// The pitch this fact *sounds*, and where it is stored.
    ///
    /// Wider than [`Self::pitch_of`] by exactly one case, and separate from it
    /// on purpose. `pitch_of` answers "is this a notehead a reader can point
    /// at", which is what numbers positions, groups a chord, and finds a tie —
    /// and a grace note is none of those, because `04-provenance.md`'s inspector
    /// numbers what a reader points at. This one answers "does this fact carry a
    /// pitch a mapper must see", and a grace note plainly does: a
    /// `map_note_pitches(pedal, …)` that left the ornaments alone would give
    /// back a passage the composer did not write.
    ///
    /// A get/set pair over one table, so the half of the controlled traversal
    /// that *collects* pitches and the half that *puts them back* cannot
    /// disagree about which facts carry one — and a newly-added [`FactKind`]
    /// fails to compile in both until its policy is chosen deliberately.
    pub(crate) const fn sounding_pitch(&self) -> Option<WrittenPitch> {
        match &self.kind {
            FactKind::Note { pitch, .. } | FactKind::Grace { pitch, .. } => Some(*pitch),
            FactKind::Rest { .. }
            | FactKind::Mark { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Clef { .. }
            | FactKind::Tempo { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. }
            | FactKind::Repeat { .. }
            | FactKind::Ending { .. }
            | FactKind::Mobile { .. }
            | FactKind::Improvise { .. } => None,
        }
    }

    /// The same place, to write through. See [`Self::sounding_pitch`].
    pub(crate) const fn sounding_pitch_mut(&mut self) -> Option<&mut WrittenPitch> {
        match &mut self.kind {
            FactKind::Note { pitch, .. } | FactKind::Grace { pitch, .. } => Some(pitch),
            FactKind::Rest { .. }
            | FactKind::Mark { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Clef { .. }
            | FactKind::Tempo { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. }
            | FactKind::Repeat { .. }
            | FactKind::Ending { .. }
            | FactKind::Mobile { .. }
            | FactKind::Improvise { .. } => None,
        }
    }
}

impl musa_kernel::Canonical for ScoreFact {
    const OWNER_TYPE_ID: &'static str = "musa.compiler.ScoreFact";
    const QUOTIENT_VERSION: u32 = 1;

    /// Deterministic key for canonical ordering and the admitted score-fact
    /// equality (docs/rules/kernel/05 N3, 12): scope, kind, source span, and
    /// expansion path. Other stored compilation details are deliberately not
    /// part of this quotient.
    ///
    /// A note or rest with nothing written on it keys exactly as it did
    /// before facts were heterogeneous, so a piece of plain notes has the
    /// normal form it has always had.
    fn canonical_key(&self) -> String {
        let articulations = |marks: &[crate::Mark]| {
            if marks.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = marks.iter().map(|mark| mark.name()).collect();
                format!("|artic:{}", names.join(","))
            }
        };
        let held = |free: Option<&crate::score::FreeDuration>| {
            free.map_or_else(String::new, |free| format!("|to:{}", free.most.as_ratio()))
        };
        let kind = match &self.kind {
            FactKind::Note {
                pitch,
                duration,
                articulations: marks,
                free,
            } => format!(
                "note:{pitch}|{}{}{}",
                duration.spelling,
                articulations(marks),
                held(free.as_ref())
            ),
            FactKind::Rest {
                duration,
                articulations: marks,
                free,
            } => format!(
                "rest|{}{}{}",
                duration.spelling,
                articulations(marks),
                held(free.as_ref())
            ),
            // The argument is in the key: two `mark text` occurrences over one
            // span say different things, and N3 must be able to tell them
            // apart or the semantic hash would call them equal.
            FactKind::Mark { mark, argument } => match argument {
                Some(argument) => format!("mark:{mark}:{argument}|"),
                None => format!("mark:{mark}|"),
            },
            // The index is in the key for the reason its own doc gives: every
            // grace note of a group shares a span, so this string is the only
            // thing N2 has to order them by.
            FactKind::Grace {
                pitch,
                articulations: marks,
                index,
            } => format!("grace:{pitch}:{index}|{}", articulations(marks)),
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
            FactKind::Clef { clef } => format!("clef:{}|", clef.name()),
            FactKind::Tempo { metronome, text, ramp } => {
                let mark = metronome.map_or_else(String::new, |mark| format!("{}={}", mark.beat, mark.bpm));
                let ramp = ramp.as_ref().map_or_else(String::new, |ramp| {
                    format!(
                        "{}>{}>{}",
                        ramp.to.map_or_else(String::new, |bpm| bpm.to_string()),
                        ramp.over.as_ratio(),
                        ramp.shape.canonical_key()
                    )
                });
                format!("tempo:{mark}:{}:{ramp}|", text.as_deref().unwrap_or_default())
            }
            FactKind::Section { name } => format!("section:{name}|"),
            FactKind::Harmony { symbol } => format!("harmony:{}|", symbol.text()),
            FactKind::Repeat { times, range } => match range {
                Some((least, most)) => format!("repeat:{times}:{least}:{most}|"),
                None => format!("repeat:{times}|"),
            },
            FactKind::Ending { bracket, pass } => format!("ending:{bracket}:{pass}|"),
            FactKind::Mobile { fragments, order } => {
                let order: Vec<String> = order.iter().map(u32::to_string).collect();
                format!("mobile:{}:{}|", fragments.join(","), order.join(","))
            }
            FactKind::Improvise { over } => format!("improvise:{}|", over.as_deref().unwrap_or_default()),
        };
        // Written rather than `format!`ed so the scope costs no second
        // allocation: P4 walks every occurrence on every edit.
        let mut key = String::with_capacity(kind.len().saturating_add(32));
        match self.scope {
            // `*` sorts before any part number, so at one instant the context
            // a reader meets first is the context that prints first.
            Scope::Piece => key.push_str("*|*|"),
            // A part sorts with its own number and before any of its voices,
            // which is where a reader meets its clef.
            Scope::Part { part } => {
                let _ = write!(key, "{part}|*|");
            }
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
/// a `ScoreSnapshot` (docs/rules/kernel/06).
pub(crate) fn elaborate(source: &SourceDocument, options: &crate::CompileOptions) -> Compilation {
    let document = musa_language::parse(source.text());
    // Parsing is the one phase with its own timing question — a large file
    // that is slow to *parse* and a large file that is slow to *elaborate*
    // are different bugs — and the boundary between them is this line.
    tracing::debug!(phase = "parse", "parsed");
    let mut resolver = Resolver::new();
    elaborate_parsed(&document, source.name(), options, &mut resolver)
}

/// One voice's elaborated timeline, before the snapshot adapter sees it.
pub(crate) type VoiceTrack = EventTrack<WrittenTime, ScoreFact>;

/// Everything after parsing (docs/rules/kernel/06): elaborate, adapt, check.
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
        tracing::debug!(
            phase = "syntax",
            errors = resolver.diagnostics.len(),
            "stopped at syntax"
        );
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }
    let root = document.syntax();
    let mut templates = crate::template::Templates::collect(resolver, &root);
    // The piece a document declares: written out, or made by an instance
    // standing where it would be. Both are one piece, and everything after
    // this line reads the same `PieceDecl` either way.
    let made = musa_language::ast::MakeStmt::from_root(&root)
        .and_then(|site| templates.instance(resolver, &site, "piece", crate::template::Kind::Piece, None, name));
    let Some(piece) = PieceDecl::from_root(&root).or_else(|| made.as_ref().and_then(crate::template::Instance::piece))
    else {
        if let Some(library) = musa_language::ast::LibraryDecl::from_root(&root) {
            return elaborate_material(resolver, &library, name, options);
        }
        if musa_language::ast::MakeStmt::from_root(&root).is_none() {
            resolver.report(
                Diagnostic::error(Code::Misplaced, "this file declares no piece")
                    .at(SourceSpan::new(0, 0), "expected `piece \"…\" { … }`")
                    .help("every musa file is one piece, or a `library { … }` for others to import"),
            );
        }
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    };
    if piece.is_template() && made.is_none() {
        resolver.report(
            Diagnostic::error(Code::Misplaced, "a piece with parameters needs `template`")
                .at(resolve::trimmed_span(piece.syntax()), "this piece takes parameters")
                .help("write `template piece …` and a `make … as …;` for each instance"),
        );
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }

    resolver.realization = options.realization.clone();
    let mut snapshot = ScoreSnapshot::default();
    let mut imports = musa_language::ast::ImportStmt::all_at_root(&root);
    imports.extend(piece.imports());
    let libraries = crate::imports::load(resolver, name, &imports, &options.imports);
    let sources = declaring(&root, &libraries, piece.syntax());
    let Some(mut elaborated) = crate::document::elaborate(resolver, &sources, made.as_ref()) else {
        tracing::debug!(
            phase = "check",
            diagnostics = resolver.diagnostics.len(),
            "stopped at the core"
        );
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    };
    tracing::debug!(phase = "check", diagnostics = resolver.diagnostics.len(), "checked");
    // Before the piece is read, because reading it mutates the site table and
    // a machine is a *declaration*: what this answers is the same either way,
    // and asking first is what keeps the two readings independent.
    let machines = elaborated.machines();
    elaborate_libraries(resolver, &libraries, &mut snapshot);
    resolve::lower_header(resolver, &piece, &mut snapshot);
    let identity = elaborate_score(resolver, &mut elaborated, &piece, name, &mut snapshot);
    // The identity hash is the one fact that says *which* piece was produced,
    // and it is what two runs that should agree are compared on.
    tracing::debug!(phase = "elaborate", %identity, "elaborated");
    snapshot.set_annotations(std::mem::take(&mut resolver.annotations));
    // Advice about a piece that does not compile is advice about a piece that
    // does not exist. A bar reported as a quarter too long already makes every
    // later barline wrong, and "this voice stops part-way through measure 3"
    // is that same quarter, said again from further away.
    if !reported_an_error(resolver) {
        resolve::check_measure_sanity(resolver, &snapshot);
        resolve::check_groove_has_a_meter(resolver, &snapshot);
        check_tuplets(resolver, &snapshot);
    }
    if reported_an_error(resolver) {
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
    // The lint pass reads the resolver's reference index and the lowered
    // studio, so it runs before either leaves the resolver — and after every
    // error check above, because advice about a piece that does not compile
    // is advice about a piece that does not exist.
    let references = std::mem::take(&mut resolver.references);
    let lints = crate::lint::lint(document, &piece, &references, &studio);
    resolver.diagnostics.extend(lints);
    // A piece that asked nothing was not realized, it was compiled, and the
    // score says so by having no performance at all. Everything downstream —
    // the seed field, the Origin step, the export note — appears and vanishes
    // on this one value.
    let mut snapshot = snapshot;
    if !resolver.decisions.is_empty() {
        snapshot.set_performance(resolver.realization.seed());
    }
    Compilation::new(Some(snapshot), std::mem::take(&mut resolver.diagnostics))
        .with_studio(studio)
        .with_machines(machines)
        .with_identity(identity)
        .with_decisions(std::mem::take(&mut resolver.decisions))
        .with_references(references)
}

/// Every node whose declarations this piece is read against, in reading order.
///
/// The import closure first, then the document's own root, then the piece —
/// which is a source of its own because a `piece` *declares*: its motifs, its
/// fragments and its `let`s are the names its voices write, and a walk that
/// stopped at the root would leave every one of them unbound. A template's body
/// arrives the same way, since the piece a root `make` names is a node like any
/// other.
///
/// None of them is in phase: [`crate::imports::load`] leaves an
/// `import … changes syntax` out of the closure, and a phase module is
/// elaborated by [`crate::expand`] under vocabulary of its own.
fn declaring(
    root: &SyntaxNode,
    libraries: &crate::imports::Libraries,
    piece: &SyntaxNode,
) -> Vec<crate::document::Source> {
    libraries
        .each()
        .map(|(from, library)| crate::document::Source::imported(library.syntax(), from))
        .chain([crate::document::Source::own(root), crate::document::Source::own(piece)])
        .collect()
}

/// The piece, read and evaluated: everything it claims proved, everything it
/// sounds projected into `snapshot`, and the identity of the whole.
///
/// One compilation, one temporal object
/// (docs/rules/kernel/06-surface-elaboration.md). Part and voice identity live
/// in `Scope` rather than in a timeline per voice, which is why nothing here
/// holds a lane: the projection buckets the piece's own occurrences by the scope
/// each fact was constructed at.
///
/// The order below is forced, and worth saying because three of the steps could
/// look independent and are not. The piece is evaluated before the barlines,
/// because the meters it states are occurrences *in* it. The barlines come
/// before the claims, because "this bar is a quarter short" is a sentence
/// measured in bars. And the markers come after the barlines and before the
/// hash, because `at bar 9` is a coordinate the meters decide, and because the
/// piece's identity is the identity of the whole piece, markers included.
fn elaborate_score(
    resolver: &mut Resolver,
    elaborated: &mut crate::document::Document,
    piece: &PieceDecl,
    namespace: &str,
    snapshot: &mut ScoreSnapshot,
) -> musa_kernel::SemanticHash {
    let Some(score) = piece.score() else {
        return musa_kernel::SemanticHash::default();
    };
    // Named bars join the namespace before any voice is read, so a bar in the
    // cello can be answered by the violin above it — or refused, if the answer
    // comes first. Either way the name exists.
    resolve::register_bars(resolver, snapshot, &score);
    let Some(read) = elaborated.piece(resolver, piece.syntax(), namespace) else {
        return musa_kernel::SemanticHash::default();
    };
    let Some(sounding) = evaluated(resolver, elaborated, &read.track) else {
        return musa_kernel::SemanticHash::default();
    };
    for part in &read.parts {
        if !part.name.is_empty()
            && let Some(span) = part.name_span
        {
            resolver
                .references
                .declare(crate::resolve::NameKind::Part, &part.name, span);
        }
        // A part's own meter is what the barlines in *this* part are counted
        // against, and the claims below are proved before any projection exists,
        // so the resolver carries it rather than reading it back off the score.
        if let Some(meter) = part.meter {
            resolver.part_meters.insert(part.id, meter);
        }
        assign_profile(resolver, snapshot, part);
        for voice in &part.voices {
            if !voice.name.is_empty()
                && let Some(span) = voice.name_span
            {
                resolver
                    .references
                    .declare(crate::resolve::NameKind::Voice, &voice.name, span);
            }
        }
    }
    // The meters first: every check below is measured against the barlines, and
    // where the barlines fall is what the meters decide.
    let bars = resolve_meters(resolver, stated(&sounding, meter_of));
    check_keys(resolver, &bars, stated(&sounding, key_of));
    prove(resolver, elaborated, &read, &bars);
    if resolver.track_sink.is_some() {
        // Measurement only, and the one place a voice is wanted on its own; the
        // piece itself is one term and was evaluated once, above.
        let lanes: Vec<VoiceTrack> = read
            .parts
            .iter()
            .flat_map(|part| part.voices.iter())
            .filter_map(|voice| elaborated.track(&voice.track).ok())
            .collect();
        if let Some(sink) = &mut resolver.track_sink {
            sink.extend(lanes);
        }
    }
    let whole = musa_kernel::together(vec![placed(resolver, &score, &bars, sounding.duration()), sounding]);
    // The piece's identity, taken where the piece exists as one temporal object
    // and nowhere else: after this line the timeline is a projection, and a hash
    // of the projection would be a hash of a view.
    let identity = whole.semantic_hash();
    let projection = crate::project::project(resolver, &whole);
    snapshot.set_contexts(projection.contexts);
    let mut projected = projection.voices;
    for part in &read.parts {
        let id = PartId(part.id);
        let mut names = indexmap::IndexMap::with_capacity(part.voices.len());
        let mut voices = indexmap::IndexMap::with_capacity(part.voices.len());
        for voice in &part.voices {
            let held = VoiceId(voice.id);
            names.insert(held, voice.name.clone());
            voices.insert(
                held,
                projected
                    .swap_remove(&(part.id, voice.id))
                    .unwrap_or_else(|| Voice::new(Vec::new())),
            );
        }
        snapshot
            .parts_mut()
            .insert(id, Part::new(id, part.name.clone(), voices, names));
    }
    identity
}

/// Record which profile realizes `part`, or say the piece declares no such
/// profile.
///
/// Here rather than in the reading that walked the part, because this is the
/// one place both halves are in hand: [`crate::lower::piece::Part::profile`] is
/// what the part says, and `snapshot` is what the piece declares. A profile
/// changes no note and no barline — it is how the marks a note carries become
/// numbers, which is `lower_performance`'s question and nothing the notation
/// asks.
fn assign_profile(resolver: &mut Resolver, snapshot: &mut ScoreSnapshot, part: &crate::lower::piece::Part) {
    let Some((profile, span)) = &part.profile else {
        return;
    };
    if snapshot.profiles().declares(profile) {
        snapshot.profiles_mut().assign(&part.name, profile.clone());
        return;
    }
    let known: Vec<&str> = snapshot.profiles().names().collect();
    let help = resolve::suggest(profile, &known, "profiles");
    resolver.report(
        Diagnostic::error(Code::UnknownName, format!("cannot find profile `{profile}`"))
            .at(*span, "not declared in this piece")
            .help(help),
    );
}

/// `raw` as the track it denotes, with any refusal restated where it was
/// written.
///
/// [`None`] rather than an empty track, because a piece that did not evaluate is
/// not a piece that sounds nothing: every check below it would then be run
/// against silence and report a second complaint about the first one.
fn evaluated(
    resolver: &mut Resolver,
    elaborated: &crate::document::Document,
    raw: &musa_core::Raw,
) -> Option<VoiceTrack> {
    match elaborated.track(raw) {
        Ok(track) => Some(track),
        Err(error) => {
            resolver.report(crate::lower::refusals::restate(elaborated.sites(), &error));
            None
        }
    }
}

/// Prove every claim the piece writes, each against the barlines its own part
/// counts by.
///
/// Per part rather than per piece because `Meter` inherits by `Override`
/// (`crate::scope`): a part in 7/8 does not hear the piece's changes at all, so
/// a bar written in it is a measure nothing else in the score agrees about.
///
/// The claims are placed by [`crate::document::Document::passage`] rather than
/// by a cursor this walk keeps, which is the whole of what a fold buys: where a
/// passage begins is how long the music before it lasts, and both are exact
/// rational arithmetic on terms the reading already recorded.
fn prove(
    resolver: &mut Resolver,
    elaborated: &crate::document::Document,
    read: &crate::lower::piece::Piece,
    bars: &crate::BarLines,
) {
    for part in &read.parts {
        let here = part_bars(resolver, Scope::Part { part: part.id });
        for claimed in part.voices.iter().flat_map(|voice| voice.claims.iter()) {
            let (claim, passage) = match elaborated.passage(claimed) {
                Ok(placed) => placed,
                Err(error) => {
                    resolver.report(crate::lower::refusals::restate(elaborated.sites(), &error));
                    continue;
                }
            };
            let settled = crate::assert::Settled {
                bars: here.as_ref().unwrap_or(bars),
                meter_written: resolver.meter_written,
            };
            if let Some(diagnostic) = crate::assert::check(&claim, &passage, &settled) {
                resolver.report(diagnostic);
            }
        }
    }
}

/// Every change of one kind of context the evaluated piece states, in the order
/// the barlines are folded in.
///
/// A **change** is a fact that begins somewhere, and that is what tells the two
/// kinds of statement apart here: [`crate::lower::notation`] writes a `meter` or
/// a `key` among a voice's items as a *point*, at the instant the fold had
/// reached, while [`crate::lower::piece`] writes a header's over the region it
/// governs. A header says what is in force, not what changes, so a fold that
/// counted it would start the piece over at its own first barline.
///
/// A part's own meter is not a change either, and needs no test: it is
/// constructed at `Scope::Part` and read through [`part_bars`], because `Meter`
/// inherits by `Override` and a part that states one does not hear the piece's.
fn stated<T>(sounding: &VoiceTrack, select: impl Fn(&FactKind) -> Option<T>) -> Vec<(MusicalTime, T, SourceSpan)> {
    let mut changes: Vec<(MusicalTime, T, SourceSpan)> = sounding
        .occurrences()
        .iter()
        .filter(|occurrence| {
            occurrence.payload().scope == Scope::Piece && occurrence.span().start() == occurrence.span().end()
        })
        .filter_map(|occurrence| {
            let fact = occurrence.payload();
            Some((
                MusicalTime::new(occurrence.span().start().as_ratio()),
                select(&fact.kind)?,
                fact.origin.source_span,
            ))
        })
        .collect();
    changes.sort_by_key(|(at, _, span)| (*at, span.start));
    changes
}

/// The meter a fact states, when it states one.
fn meter_of(kind: &FactKind) -> Option<Meter> {
    if let FactKind::Meter { numerator, denominator } = *kind {
        return Some(Meter::new(numerator, denominator));
    }
    None
}

/// The key a fact states, when it states one.
fn key_of(kind: &FactKind) -> Option<crate::Key> {
    if let FactKind::Key { tonic, mode } = *kind {
        return Some(crate::Key::new(tonic, mode));
    }
    None
}

/// The markers a score places by coordinate rather than by where a fold
/// reached: its form sections and its chord symbols (roadmap §8.2).
///
/// Here rather than in the reading, because `at bar 9` is a position in *bars*
/// and where the bars fall is what the meters decide — which is not settled
/// until the piece has been evaluated. Both are points, because a position is
/// only meaningful once the piece has a length to be inside of, and nothing
/// interprets either: a chord symbol is parsed so that a later library can read
/// it, and that is the end of the core's involvement.
fn placed(
    resolver: &mut Resolver,
    score: &musa_language::ast::ScoreDecl,
    bars: &crate::BarLines,
    extent: Duration<WrittenTime>,
) -> VoiceTrack {
    let declaration = crate::origin::DeclarationId::default();
    let at_span = |span: SourceSpan| Origin {
        source_span: span,
        definition_span: span,
        declaration,
        expansion_path: Vec::new(),
    };
    let extent_time = MusicalTime::new(extent.as_ratio());
    let mut occurrences = Vec::new();
    for section in score.sections() {
        let span = resolve::trimmed_span(section.syntax());
        let Some(at) = resolve_position(resolver, section.position().as_ref(), span, bars, extent_time) else {
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
        let Some(at) = resolve_position(resolver, chord.position().as_ref(), span, bars, extent_time) else {
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
    track_or_empty(extent, occurrences)
}

/// A point occurrence at an absolute time, for the facts that are placed by
/// coordinate rather than by where the cursor reached.
fn point_at(at: MusicalTime, fact: ScoreFact) -> Occurrence<WrittenTime, ScoreFact> {
    let instant = Position::new(at.as_ratio());
    let span = Span::new(instant, instant).unwrap_or(Span::ZERO);
    Occurrence::new(span, fact)
}

/// Register everything the imported libraries declare, before the piece's
/// own declarations, so a collision is reported against the library that
/// caused it (roadmap §16).
///
/// A library declares and does not sound, so there is no score to build and
/// the absence of one is not a failure — that is the whole difference between
/// this path and the piece path, and it is why [`Compilation::kind`] exists.
/// What it *does* do is everything a check is for: resolve what the library
/// builds on, register its declarations so a duplicate or a malformed motif is
/// reported, and hold its `studio` to a library's rules.
///
/// The library's own studio arrives as an *imported* block rather than as the
/// document's. That is not a trick: the rules for a library's studio are the
/// rules that apply to it wherever it is read, and a library that could wire
/// itself to a score when opened directly and not when imported would compile
/// two different ways.
fn elaborate_material(
    resolver: &mut Resolver,
    library: &musa_language::ast::LibraryDecl,
    name: &str,
    options: &crate::CompileOptions,
) -> Compilation {
    let mut snapshot = ScoreSnapshot::default();
    let libraries = crate::imports::load(resolver, name, &library.imports(), &options.imports);
    // The one checker, here as everywhere: the library and its imports as a
    // document, declared by musa-core. A document that came back is not yet a
    // library that checks — `elaborate` answers `Some` beside refusals it
    // reported — so the diagnostics decide.
    let sources: Vec<crate::document::Source> = libraries
        .each()
        .map(|(from, imported)| crate::document::Source::imported(imported.syntax(), from))
        .chain(std::iter::once(crate::document::Source::own(library.syntax())))
        .collect();
    let _ = crate::document::elaborate(resolver, &sources, None);
    if resolver
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error)
    {
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics)).into_material();
    }
    elaborate_libraries(resolver, &libraries, &mut snapshot);
    if let Some(performance) = library.performance() {
        let profiles = resolve::parse_profiles(resolver, &performance);
        let span = resolve::span_of(performance.syntax());
        resolve::merge_profiles(resolver, &mut snapshot, &profiles, span, None);
    }
    resolve::register_motifs(resolver, &mut snapshot, &library.motifs(), None);
    resolve::register_fragments(resolver, &mut snapshot, &library.fragments(), None);
    let studios: Vec<musa_language::ast::StudioDecl> = libraries
        .each()
        .filter_map(|(_, imported)| imported.studio())
        .chain(library.studio())
        .collect();
    let mut references = std::mem::take(&mut resolver.references);
    crate::studio::resolve(None, &studios, &[], &mut references, &mut resolver.diagnostics);
    Compilation::new(None, std::mem::take(&mut resolver.diagnostics)).into_material()
}

fn elaborate_libraries(resolver: &mut Resolver, libraries: &crate::imports::Libraries, snapshot: &mut ScoreSnapshot) {
    for (from, library) in libraries.each() {
        let path = from.path;
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
        resolve::register_fragments(resolver, snapshot, &library.fragments(), Some(path));
    }
}

/// Turn a `measure:beat` coordinate into time, or report why it is not one.
///
/// Measures and beats count from one, the way a composer reads them off the
/// page, and a position past the end of the piece is an error: a chord symbol
/// nobody will ever reach is a mistake, not a comment.
///
/// `bars` are folded from the meter *occurrences* and `extent` is the
/// timeline's own extent — neither is recomputed from the snapshot. The
/// extent is exact rather than a maximum over event ends,
/// and a piece that ends in a rest still ends where the rest ends, because a
/// rest is an occurrence.
fn resolve_position(
    resolver: &mut Resolver,
    position: Option<&musa_language::ast::Position>,
    span: SourceSpan,
    bars: &crate::BarLines,
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
    let measure = u32::try_from(measure).unwrap_or(u32::MAX);
    let at = bars.time_of(measure, beat)?;
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

/// The placeholder a shared body carries where the call site would be.
///
/// A motif body's occurrences take their `source_span` from the *call*, which
/// is exactly what cannot be baked into a shared body. They carry this
/// instead, and each reference's mark says what to put there. No document is
/// four gigabytes, so it cannot collide with a real span.
pub(crate) const SHARED_ORIGIN: SourceSpan = SourceSpan::new(u32::MAX, u32::MAX);

/// The placeholder a shared body carries where the voice would be, for the
/// same reason: the same motif called from two voices is one body.
pub(crate) const SHARED_SCOPE: Scope = Scope::Voice {
    part: u32::MAX,
    voice: u32::MAX,
};

/// Apply a mark to a freshly instantiated body (E-Mark).
///
/// Payloads only, which is the whole of T6's contract: the spans, the extent,
/// the count and the order are the instantiated timeline's own and are not
/// touched here.
pub(crate) fn instantiate(mark: &str, instance: &mut VoiceTrack) {
    let Some(crate::factext::ReferenceMark {
        depth,
        steps,
        origin,
        scope,
    }) = crate::factext::read_reference_mark(mark)
    else {
        return;
    };
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

/// The score facts one voicing sounds, as one simultaneous segment.
///
/// Shared by the `stack` sugar and by `play`, so the two cannot disagree
/// about what a voicing sounds. The enclosing transposition applies to each
/// pitch exactly as it does to a written chord: a voicing is notes, and notes
/// move.
fn reported_an_error(resolver: &Resolver) -> bool {
    resolver
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error)
}

/// The barlines a scope counts against, when they are not the piece's.
///
/// `None` is not "no barlines" — it is "the piece's", which every scope in a
/// piece that is not polymetric answers.
fn part_bars(resolver: &Resolver, scope: Scope) -> Option<crate::BarLines> {
    if resolver.part_meters.is_empty() {
        return None;
    }
    let part = match scope {
        Scope::Piece => return None,
        Scope::Part { part } | Scope::Voice { part, .. } => part,
    };
    // Uniform, and that is the whole of it: `Meter` inherits by `Override`
    // (`scope.rs`), so a part that states its own meter does not hear the
    // piece's changes at all, and the grammar gives a part exactly one.
    resolver.part_meters.get(&part).copied().map(crate::BarLines::uniform)
}

/// Fold the meters the piece states into barlines, refusing any change that
/// does not land on one.
///
/// Ascending, because the question "is this a barline" is answered by the
/// changes before it and by nothing else. This is why the meters are a pass
/// of their own rather than a lookup: a measure coordinate is a position in
/// bars, and where the bars fall is what the meters decide.
fn resolve_meters(resolver: &mut Resolver, changes: Vec<(MusicalTime, Meter, SourceSpan)>) -> crate::BarLines {
    let mut bars = crate::BarLines::uniform(resolver.meter);
    let mut stated: Vec<(MusicalTime, Meter, SourceSpan)> = Vec::new();
    for (at, meter, span) in changes {
        if let Some((_, already, first)) = stated.iter().find(|(other, _, _)| *other == at) {
            // Two voices naming the same change is how a composer writes it,
            // and repeating a fact is not a mistake. Naming two different
            // meters for one place is.
            if *already != meter {
                resolver.report(two_at_once("meters", span, *first));
            }
            continue;
        }
        if !bars.change(at, meter) {
            resolver.report(off_barline("meter", &bars, at, span));
            continue;
        }
        stated.push((at, meter, span));
    }
    bars
}

/// Refuse any modulation that does not land on a barline.
///
/// A key signature is printed at a barline, so a modulation a third of the
/// way through a measure is a page nobody can engrave. Same refusal as the
/// meter's, and deliberately *not* the clef's: run after the meters, because
/// the barlines it is measured against are what the meters decided.
fn check_keys(resolver: &mut Resolver, bars: &crate::BarLines, changes: Vec<(MusicalTime, crate::Key, SourceSpan)>) {
    let mut stated: Vec<(MusicalTime, crate::Key, SourceSpan)> = Vec::new();
    for (at, key, span) in changes {
        if let Some((_, already, first)) = stated.iter().find(|(other, _, _)| *other == at) {
            if *already != key {
                resolver.report(two_at_once("keys", span, *first));
            }
            continue;
        }
        if bars.meter_at(at).is_measured() && bars.at(at).into != crate::MusicalDuration::ZERO {
            resolver.report(off_barline("key", bars, at, span));
            continue;
        }
        stated.push((at, key, span));
    }
}

/// "You wrote it here, and here is not a barline" — the same sentence for a
/// meter and for a key, because it is the same mistake and the composer's fix
/// is the same either way.
fn off_barline(what: &str, bars: &crate::BarLines, at: MusicalTime, span: SourceSpan) -> Diagnostic {
    let here = bars.at(at);
    Diagnostic::error(Code::DoesNotAddUp, format!("a {what} change must land on a barline"))
        .at(
            span,
            format!(
                "this is {} into measure {}",
                fraction(here.into.as_ratio()),
                here.measure
            ),
        )
        .help(format!(
            "add {} before it, or move it past the next barline",
            fraction(bars.measure_at(at).end.as_ratio() - at.as_ratio())
        ))
}

/// Two voices may both name a change — repeating a fact is not a mistake —
/// but they may not disagree about it.
fn two_at_once(what: &str, span: SourceSpan, first: SourceSpan) -> Diagnostic {
    Diagnostic::error(Code::Misplaced, format!("two {what} at the same place"))
        .at(span, "the second of two")
        .also(first, "the first is here")
}

/// A musical amount, spelled the way the language spells it.
fn fraction(value: Ratio<i64>) -> String {
    if *value.denom() == 1 {
        value.numer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}

fn track_or_empty(extent: Duration<WrittenTime>, occurrences: Vec<Occurrence<WrittenTime, ScoreFact>>) -> VoiceTrack {
    track(extent, occurrences).unwrap_or_else(|_| empty_segment())
}

/// The empty segment `(0, ∅)` — contributes nothing to the sequence.
fn empty_segment() -> VoiceTrack {
    empty(Duration::ZERO)
}

/// A tuplet has to be spellable, and a group split across a barline is not:
/// the notes on either side would need their own bracket and their own
/// ratio, which is a different piece of music from the one that was written.
fn check_tuplets(resolver: &mut Resolver, snapshot: &ScoreSnapshot) {
    // Which part each event belongs to, so a tuplet in a 7/8 part is measured
    // against the 7/8 barlines. Built once: a tuplet names its first event,
    // and there is no other way from an event back to its staff.
    let mut owner: std::collections::BTreeMap<crate::EventId, u32> = std::collections::BTreeMap::new();
    for (id, part) in snapshot.parts().iter() {
        for (_, voice) in part.voices() {
            for event in voice.events() {
                owner.insert(event.id, id.0);
            }
        }
    }
    let mut offenders = Vec::new();
    for tuplet in snapshot.annotations().tuplets() {
        let bars = owner.get(&tuplet.from).map_or_else(
            || snapshot.bars(Scope::Piece),
            |part| snapshot.bars(Scope::Part { part: *part }),
        );
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
        // `closing` is the reason the epsilon this replaced is gone: a group
        // ending exactly on a barline closes the measure before it, which is
        // the question being asked, said exactly rather than nearly.
        if bars.at(start).measure != bars.closing(end) {
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

/// The one exhaustive coverage table for the controlled traversal. Keeping
/// it at the fact boundary makes a newly-added `FactKind` a compile error
/// until its sounding-pitch policy is chosen deliberately.
#[cfg(test)]
pub(crate) fn map_note_pitch_fact(
    payload: &ScoreFact,
    mut mapper: impl FnMut(WrittenPitch) -> Option<WrittenPitch>,
) -> Option<ScoreFact> {
    let mut mapped = payload.clone();
    match &mut mapped.kind {
        FactKind::Note { pitch, .. } | FactKind::Grace { pitch, .. } => *pitch = mapper(*pitch)?,
        FactKind::Rest { .. }
        | FactKind::Mark { .. }
        | FactKind::Slur
        | FactKind::Phrase { .. }
        | FactKind::Tuplet { .. }
        | FactKind::Dynamic { .. }
        | FactKind::Hairpin { .. }
        | FactKind::Key { .. }
        | FactKind::Meter { .. }
        | FactKind::Clef { .. }
        | FactKind::Tempo { .. }
        | FactKind::Section { .. }
        | FactKind::Harmony { .. }
        | FactKind::Repeat { .. }
        | FactKind::Mobile { .. }
        | FactKind::Improvise { .. }
        | FactKind::Ending { .. } => {}
    }
    Some(mapped)
}

/// The normalized human-display text of a source's piece timeline, for golden
/// snapshots (docs/rules/kernel/05 N5). Semantic hashing uses separate framed N6
/// bytes. `None` when the
/// source does not elaborate cleanly.
///
/// One timeline, not one per part: a compilation has exactly
/// one temporal object, and the normal form is the text of that object —
/// key, meter, form markers and chord symbols included.
#[doc(hidden)]
pub fn kernel_normal_form(
    source: &SourceDocument,
    realization: &crate::Realization,
    imports: &crate::imports::ImportSources,
) -> Option<String> {
    let (_, term, _) = piece_term(source, realization, imports)?;
    Some(musa_kernel::evaluate_marked(term, instantiate).to_string())
}

/// The piece as a **term** (docs/rules/kernel/10): its name, and an `over` of one
/// literal per voice plus one for the piece-wide context.
///
/// One literal per voice and not one for the whole, because that shape is what
/// the interchange spelling is *for*: a reader of the text can see which lane a
/// fact belongs to without taking a scope apart, and the normalized spelling
/// ([`crate::kernel_normalized_text`]) is the one that collapses it. The sharing
/// the replaced elaborator wrapped around the `over` is gone with it: a motif
/// called from two voices is one definition of the *document* now, and its body
/// is shared where documents share things rather than in the kernel text.
pub(crate) fn piece_term(
    source: &SourceDocument,
    realization: &crate::Realization,
    imports: &crate::imports::ImportSources,
) -> Option<(
    String,
    musa_kernel::Term<WrittenTime, ScoreFact>,
    Vec<crate::DecisionRecord>,
)> {
    let document = musa_language::parse(source.text());
    if !document.errors().is_empty() {
        return None;
    }
    let root = document.syntax();
    let mut resolver = Resolver::new();
    resolver.realization = realization.clone();
    // A made piece is this document's piece, so the term of a document whose
    // piece is an instance is the term of what the instance makes.
    let mut templates = crate::template::Templates::collect(&mut resolver, &root);
    let made = musa_language::ast::MakeStmt::from_root(&root).and_then(|site| {
        templates.instance(
            &mut resolver,
            &site,
            "piece",
            crate::template::Kind::Piece,
            None,
            source.name(),
        )
    });
    let piece = PieceDecl::from_root(&root).or_else(|| made.as_ref().and_then(crate::template::Instance::piece))?;
    let mut snapshot = ScoreSnapshot::default();
    // The same closure full compilation reads: a `use` of imported material
    // is the piece's own music, and an export that could not name it would be
    // an export some pieces cannot make.
    let mut wanted = musa_language::ast::ImportStmt::all_at_root(&root);
    wanted.extend(piece.imports());
    let libraries = crate::imports::load(&mut resolver, source.name(), &wanted, imports);
    let sources = declaring(&root, &libraries, piece.syntax());
    let mut elaborated = crate::document::elaborate(&mut resolver, &sources, made.as_ref())?;
    resolve::lower_header(&mut resolver, &piece, &mut snapshot);
    let score = piece.score()?;
    resolve::register_bars(&mut resolver, &mut snapshot, &score);
    let read = elaborated.piece(&mut resolver, piece.syntax(), source.name())?;
    let sounding = elaborated.track(&read.track).ok()?;
    // The barlines, and only because the markers below are placed in them: this
    // helper answers a term rather than a diagnosis, so the meters are folded for
    // the coordinates they decide and the refusals go nowhere.
    let bars = resolve_meters(&mut resolver, stated(&sounding, meter_of));
    let mut parts = Vec::new();
    for part in &read.parts {
        for voice in &part.voices {
            let lane = elaborated.track(&voice.track).ok()?;
            // A voice's fold sounds the *piece's* and the *part's* facts too
            // — a `key` written mid-voice is the piece's from there — and the
            // piece layer below carries exactly those (`spoken_of` splits by
            // the same test). Printed in both, one fact would hash as two on
            // reparse: `a_piece_and_its_kernel_printing_have_one_meaning`.
            let own = lane
                .occurrences()
                .iter()
                .filter(|occurrence| occurrence.payload().scope.voice() == Some((part.id, voice.id)))
                .cloned()
                .collect();
            parts.push(musa_kernel::Term::literal(track_or_empty(lane.duration(), own)));
        }
    }
    let marked = placed(&mut resolver, &score, &bars, sounding.duration());
    parts.push(musa_kernel::Term::literal(spoken_of(&sounding, &marked)));
    let term = musa_kernel::Term::together(parts).ok()?;
    term.check().ok()?;
    Some((piece.name().unwrap_or_default(), term, resolver.decisions))
}

/// Everything the piece states about itself rather than about a voice, as one
/// track: the header's context, each part's, and the markers a coordinate
/// placed.
///
/// Split off the evaluated piece by the same test the projection buckets with —
/// a fact's scope names a voice or it does not — because [`piece_term`]'s shape
/// is one literal per voice and one for the piece, and each voice is a term of
/// its own already.
fn spoken_of(sounding: &VoiceTrack, marked: &VoiceTrack) -> VoiceTrack {
    let occurrences = sounding
        .occurrences()
        .iter()
        .filter(|occurrence| occurrence.payload().scope.voice().is_none())
        .chain(marked.occurrences().iter())
        .cloned()
        .collect();
    track_or_empty(sounding.duration(), occurrences)
}
