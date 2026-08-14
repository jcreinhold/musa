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
use crate::origin::{ExpansionStep, Origin, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::resolve::{self, ExpandCx, Resolver};
use crate::scope::Scope;
use crate::score::{DynamicMark, Meter, Mode, NotatedDuration, Part, PartId, ScoreSnapshot, Voice, VoiceId};
use crate::time::MusicalTime;
use musa_kernel::{Duration, EventTrack, Occurrence, Position, Span, Term, WrittenTime, empty, follow, track};
use musa_language::ast::{AstNode as _, PieceDecl, VoiceItem};
use musa_language::{SyntaxKind, SyntaxNode};
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
    fn stretched(&self, factor: Ratio<i64>) -> Self {
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

    /// The same fact with its pitch mirrored about `axis`, or `None` only
    /// when a fixed-width storage coordinate would overflow.
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
    let made = musa_language::ast::MakeStmt::from_root(&root).and_then(|site| {
        templates.instance(
            resolver,
            &site,
            "piece".to_owned(),
            crate::template::Kind::Piece,
            None,
            name,
        )
    });
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
    // Every `make` inside this piece is resolved before it is checked: the
    // arguments belong to the scope the sites stand in, so they are evaluated
    // with the piece's own declarations, in one pass, and each instance reads
    // its own values back out afterwards.
    let enclosing = made.as_ref().map(crate::template::Instance::template);
    let sites = collect_voice_sites(resolver, &mut templates, &piece, enclosing, name);
    let mut bindings: Vec<crate::core::Binding> = Vec::new();
    if let Some(instance) = &made {
        let arguments = crate::core::check_arguments(resolver, &libraries, &root, instance.holders());
        let Some(arguments) = arguments else {
            return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
        };
        bindings.extend(instance.bindings(&arguments));
    }
    bindings.extend(sites.values().flat_map(crate::template::Instance::holders));
    let Some(core) = crate::core::check_piece(resolver, &libraries, &root, &piece, bindings) else {
        tracing::debug!(
            phase = "check",
            diagnostics = resolver.diagnostics.len(),
            "stopped at the core"
        );
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    };
    tracing::debug!(phase = "check", diagnostics = resolver.diagnostics.len(), "checked");
    let expansion = Expansion {
        root: root.clone(),
        libraries: Some(&libraries),
        sites: &sites,
        prefix: made.as_ref().map(crate::template::Instance::step).into_iter().collect(),
    };
    elaborate_libraries(resolver, &libraries, &mut snapshot);
    resolve::lower_header(resolver, &piece, &mut snapshot, &core);
    let identity = piece.score().map_or_else(musa_kernel::SemanticHash::default, |score| {
        elaborate_score(resolver, &piece, &score, &mut snapshot, &core, &expansion)
    });
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
        .with_machines(core.machines())
        .with_identity(identity)
        .with_decisions(std::mem::take(&mut resolver.decisions))
        .with_references(references)
}

/// What a score needs to expand the instance sites written in it.
///
/// One value rather than four parameters because it is one idea — where the
/// templates are, and how far in from the document the score already is —
/// and because a voice instance and a made piece read the same fields.
struct Expansion<'a> {
    root: SyntaxNode,
    /// The import closure a template body is read against, or `None` on the
    /// import-free interchange path (see [`piece_term`]).
    libraries: Option<&'a crate::imports::Libraries>,
    /// Every voice instance in this piece, by the span of its `make`. The
    /// sites are resolved once, before the piece is checked, because their
    /// arguments are part of what the piece's own pass evaluates.
    sites: &'a indexmap::IndexMap<u64, crate::template::Instance>,
    /// The expansion steps this whole piece already stands behind: one, when
    /// the piece itself was made.
    prefix: Vec<crate::origin::ExpansionStep>,
}

impl Expansion<'_> {
    /// Check one instance's body with its arguments bound, by whichever rule
    /// this elaboration reads declarations under.
    fn check_voice(
        &self,
        resolver: &mut Resolver,
        voice: &musa_language::ast::VoiceDecl,
        bindings: Vec<crate::core::Binding>,
    ) -> Option<crate::core::Program> {
        match self.libraries {
            Some(libraries) => crate::core::check_template_voice(resolver, libraries, &self.root, voice, bindings),
            None => crate::core::check_for_kernel(resolver, &self.root, Some(voice.syntax()), bindings),
        }
    }
}

/// One item of a part, read as the voice it stands for.
///
/// A written voice is itself, read with the piece's own program; a `make` is
/// the template's voice, read with a program of its own arguments, standing
/// one expansion step deeper than the score around it.
struct VoiceSite {
    voice: musa_language::ast::VoiceDecl,
    name: String,
    /// The instance's program, when the site is a `make`. A written voice
    /// borrows the piece's, which this value cannot hold.
    program: Option<crate::core::Program>,
    name_span: Option<SourceSpan>,
    path: Vec<crate::origin::ExpansionStep>,
}

impl VoiceSite {
    /// The part item as a voice, or `None` when it is not one: an instance of
    /// a template that failed to resolve or to check, or a parameterized
    /// voice written where only a plain one belongs.
    fn read(
        resolver: &mut Resolver,
        expansion: &Expansion<'_>,
        core: &crate::core::Program,
        item: &musa_language::ast::PartItem,
    ) -> Option<Self> {
        match item {
            musa_language::ast::PartItem::Voice(voice) => {
                if voice.is_template() {
                    resolver.report(
                        Diagnostic::error(Code::Misplaced, "a voice with parameters needs `template`")
                            .at(resolve::trimmed_span(voice.syntax()), "this voice takes parameters")
                            .help("write it as `template voice …` at the top of the file, and `make` it here"),
                    );
                    return None;
                }
                Some(Self {
                    voice: voice.clone(),
                    name: voice.name().unwrap_or_default(),
                    program: None,
                    name_span: resolve::token_span(voice.syntax(), SyntaxKind::Identifier),
                    path: expansion.prefix.clone(),
                })
            }
            musa_language::ast::PartItem::Make(stmt) => {
                let instance = expansion.sites.get(&site_key(resolve::trimmed_span(stmt.syntax())))?;
                let voice = instance.voice()?;
                let program = expansion.check_voice(resolver, &voice, instance.bindings(core))?;
                let mut path = expansion.prefix.clone();
                path.push(instance.step());
                Some(Self {
                    voice,
                    name: instance.alias().to_owned(),
                    program: Some(program),
                    name_span: Some(instance.span()),
                    path,
                })
            }
        }
    }

    /// The program this voice's body is read with.
    fn program<'a>(&'a self, core: &'a crate::core::Program) -> &'a crate::core::Program {
        self.program.as_ref().unwrap_or(core)
    }
}

/// Resolve every voice instance written among this piece's parts.
///
/// Once, here, rather than during elaboration: resolving twice would report
/// an unknown template twice, and the arguments have to be known before the
/// piece is checked in any case.
fn collect_voice_sites(
    resolver: &mut Resolver,
    templates: &mut crate::template::Templates,
    piece: &PieceDecl,
    enclosing: Option<&str>,
    namespace: &str,
) -> indexmap::IndexMap<u64, crate::template::Instance> {
    let mut sites = indexmap::IndexMap::new();
    let Some(score) = piece.score() else {
        return sites;
    };
    for part in score.parts() {
        let part_name = part.name().unwrap_or_default();
        for (index, item) in part.items().into_iter().enumerate() {
            let musa_language::ast::PartItem::Make(stmt) = item else {
                continue;
            };
            let path = format!("score/part[{part_name}]/{index}");
            let span = resolve::trimmed_span(stmt.syntax());
            if let Some(instance) = templates.instance(
                resolver,
                &stmt,
                path,
                crate::template::Kind::Voice,
                enclosing,
                namespace,
            ) {
                sites.insert(site_key(span), instance);
            }
        }
    }
    sites
}

/// A site's span, as the key the resolved instances are held under.
const fn site_key(span: SourceSpan) -> u64 {
    ((span.start as u64) << 32) | span.end as u64
}

/// What the piece timeline says about the piece as a whole, for the callers
/// that need it after the projection has run.
///
/// Both fields are read *off the timeline* — the extent is the kernel's own,
/// and the barlines are folded from the meter occurrences — which is why no
/// function in this module recomputes either from the snapshot.
/// Walk parts and voices exactly as the direct lowerer does, elaborating
/// each voice into kernel facts — and then stack every one of them, plus
/// the piece's key, meter, form markers and chord symbols, into a single
/// `VoiceTrack` for the whole piece, which is projected once.
///
/// One compilation, one temporal object (docs/rules/kernel/06-surface-elaboration.md).
/// Part and
/// voice identity live in `Scope`, not in a timeline per voice, which is the
/// evidence Q3's working stance asked for.
fn elaborate_score(
    resolver: &mut Resolver,
    piece: &PieceDecl,
    score: &musa_language::ast::ScoreDecl,
    snapshot: &mut ScoreSnapshot,
    core: &crate::core::Program,
    expansion: &Expansion<'_>,
) -> musa_kernel::SemanticHash {
    let mut voice_names: indexmap::IndexMap<PartId, indexmap::IndexMap<VoiceId, String>> = indexmap::IndexMap::new();
    let mut metadata: Vec<(PartId, String)> = Vec::new();
    // A part's clef, meter and tempo are context, so they enter the timeline
    // with the piece's rather than riding on the part; the part loop only
    // collects them.
    let mut declared: Vec<(u32, resolve::PartContext)> = Vec::new();
    let mut lanes: Vec<Segment> = Vec::new();
    // One set of bindings for the whole piece: two voices calling the same
    // motif elaborate its body once between them.
    let mut share = Share::default();
    // Named bars join the namespace before any voice is read, so a bar in the
    // cello can be answered by the violin above it — or refused, if the answer
    // comes first. Either way the name exists.
    resolve::register_bars(resolver, snapshot, score);
    for part in score.parts() {
        let name = part.name().unwrap_or_default();
        let part_key = resolver.declare(crate::resolve::DeclInfo::Part);
        let _ = resolve::ordinal(resolver, part_key);
        if metadata.iter().any(|(_, existing)| *existing == name) {
            resolver.error(
                Code::DuplicateName,
                format!("this score already has a part called `{name}`"),
                resolve::span_of(part.syntax()),
                "declared again here",
            );
            continue;
        }
        if !name.is_empty()
            && let Some(name_span) = resolve::token_span(part.syntax(), SyntaxKind::Identifier)
        {
            resolver
                .references
                .declare(crate::resolve::NameKind::Part, &name, name_span);
        }
        let id = PartId(resolver.next_part);
        resolver.next_part = resolver.next_part.saturating_add(1);

        let mut context = resolve::part_context(resolver, &part, snapshot.profiles());
        if let Some(profile) = context.profile.take() {
            snapshot.profiles_mut().assign(&name, profile);
        }
        // A part's own meter is what the barlines in *this* part are counted
        // against, and the checks below run before the projection exists, so
        // the resolver carries it rather than reading it back off the score.
        if let Some((meter, _)) = context.meter {
            resolver.part_meters.insert(id.0, meter);
        }
        declared.push((id.0, context));

        let mut names = indexmap::IndexMap::new();
        // A part's voices are its written ones and the ones its instance
        // sites make, in the order they stand: a `make` between two voices
        // makes a voice there, and the numbering is positional.
        for (index, item) in part.items().into_iter().enumerate() {
            let Some(site) = VoiceSite::read(resolver, expansion, core, &item) else {
                continue;
            };
            let voice = &site.voice;
            let voice_name = site.name.clone();
            let name_span = site.name_span;
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
            if !voice_name.is_empty()
                && let Some(name_span) = name_span
            {
                resolver
                    .references
                    .declare(crate::resolve::NameKind::Voice, &voice_name, name_span);
            }
            let voice_id = VoiceId(u32::try_from(index).unwrap_or(u32::MAX));
            lanes.push(elaborate_voice(
                resolver,
                &mut share,
                voice,
                declaration,
                id.0,
                voice_id.0,
                site.program(core),
                site.path.clone(),
            ));
            names.insert(voice_id, voice_name);
        }
        voice_names.insert(id, names);
        metadata.push((id, name));
    }

    // The stack's extent without building the stack: D3 says it is the
    // maximum of the parts', and the context facts need it before they exist.
    let extent = lanes.iter().map(|lane| lane.extent).max().unwrap_or(Duration::ZERO);
    // The meters first: every check below is measured against the barlines,
    // and where the barlines fall is what the meters decide.
    let bars = resolve_meters(resolver);
    check_keys(resolver, &bars);
    for obligation in std::mem::take(&mut resolver.obligations) {
        let here = part_bars(resolver, obligation.scope);
        let settled = crate::assert::Settled {
            bars: here.as_ref().unwrap_or(&bars),
            meter_written: resolver.meter_written,
        };
        if let Some(diagnostic) = crate::assert::check(&obligation.claim, &obligation.passage, &settled) {
            resolver.report(diagnostic);
        }
    }
    let context = context_facts(resolver, piece, score, &declared, &bars, extent);
    let lane_occurrences = lanes
        .iter()
        .fold(0u64, |count, lane| count.saturating_add(lane.occurrences));
    let context_occurrences = u64::try_from(context.occurrences().len()).unwrap_or(u64::MAX);
    let output_occurrences = lane_occurrences.saturating_add(context_occurrences);
    if !share.reserve_output(
        "elaborating the piece timeline",
        output_occurrences,
        resolve::trimmed_span(score.syntax()),
    ) {
        share.report_exhaustion(resolver);
        return musa_kernel::SemanticHash::default();
    }
    if let Some(sink) = &mut resolver.track_sink {
        // Measurement only, and the one place a voice is wanted on its own;
        // the piece itself is evaluated once, below.
        let voices: Vec<_> = lanes.iter().map(|lane| share.evaluate(lane.term.clone())).collect();
        sink.extend(voices);
    }
    // One close and one evaluation for the whole piece: closing wraps the live
    // bindings, and doing it per voice would clone every shared body once per
    // voice — the cost this structure exists to remove.
    let parts: Vec<_> = lanes
        .into_iter()
        .map(|lane| lane.term)
        .chain(std::iter::once(Term::literal(context)))
        .collect();
    let open = Term::together(parts).unwrap_or_else(|_| musa_kernel::Term::literal(empty_segment()));
    let closed = share.close(open);
    if let Err(error) = closed.check() {
        resolver.report(
            Diagnostic::error(Code::TypeMismatch, "elaboration produced an invalid kernel term")
                .at(resolve::trimmed_span(score.syntax()), error.to_string())
                .note("this is a compiler invariant failure"),
        );
        return musa_kernel::SemanticHash::default();
    }
    let whole = musa_kernel::evaluate_marked(closed, instantiate);
    // The piece's identity, taken where the piece exists as one temporal
    // object and nowhere else: after this line the timeline is a projection,
    // and a hash of the projection would be a hash of a view.
    let identity = whole.semantic_hash();
    let projection = crate::project::project(resolver, &whole);
    snapshot.set_contexts(projection.contexts);
    let mut projected = projection.voices;
    for (id, name) in metadata {
        let names = voice_names.swap_remove(&id).unwrap_or_default();
        let mut voices = indexmap::IndexMap::with_capacity(names.len());
        for voice_id in names.keys() {
            let voice = projected
                .swap_remove(&(id.0, voice_id.0))
                .unwrap_or_else(|| Voice::new(Vec::new()));
            voices.insert(*voice_id, voice);
        }
        snapshot.parts_mut().insert(id, Part::new(id, name, voices, names));
    }
    identity
}

/// The facts that are about the piece rather than about a voice: its key,
/// its meter, its form markers, and its chord symbols (roadmap §8.2).
///
/// Key and meter are *regions*. Today they cover `[0, d]`, because the
/// grammar has no `modulate` and no mid-piece `meter`; when it grows one the
/// change is more occurrences, not a second representation of the same
/// question. A region that happens to cover everything is not a
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
    declared: &[(u32, resolve::PartContext)],
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
    // `lower_header` parsed the key and the meter out of the header and left
    // them on the resolver; the only thing that puts either into the snapshot
    // is the projection of the timeline they are about to enter.
    let meter = resolver.meter;
    let key = resolver.key;

    let region = Span::new(Position::ZERO, extent.reach()).unwrap_or(Span::ZERO);
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
    // Unlike the meter, an unwritten tempo is *not* a tempo: a page with no
    // marking on it is a page with no marking on it, and the 120 a performance
    // falls back to is the performance layer's default, not something the
    // piece said. So the fact exists only where a marking does.
    if let Some(header) = piece.tempo() {
        occurrences.push(Occurrence::new(
            region,
            ScoreFact::new(
                Scope::Piece,
                resolve::tempo_fact(resolver, &header),
                at_span(resolve::span_of(header.syntax())),
            ),
        ));
    }
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

    // A clef is context, not part metadata: it is in force from where it is
    // written until something replaces it, in the scope of one part, which is
    // the same shape the key and the meter have. A part's own meter and tempo
    // are the same shape again, and that is all polymeter and polytempo are:
    // the fact a scope in, with `Override` inheritance (`scope.rs`) already
    // saying that a 7/8 part does not rejoin the piece's 4/4.
    for (part, context) in declared {
        let scope = Scope::Part { part: *part };
        if let Some((clef, span)) = context.clef {
            occurrences.push(Occurrence::new(
                region,
                ScoreFact::new(scope, FactKind::Clef { clef }, at_span(span)),
            ));
        }
        if let Some((meter, span)) = context.meter {
            occurrences.push(Occurrence::new(
                region,
                ScoreFact::new(
                    scope,
                    FactKind::Meter {
                        numerator: meter.numerator(),
                        denominator: meter.denominator(),
                    },
                    at_span(span),
                ),
            ));
        }
        if let Some((tempo, span)) = &context.tempo {
            occurrences.push(Occurrence::new(
                region,
                ScoreFact::new(scope, tempo.clone(), at_span(*span)),
            ));
        }
    }

    let extent_time = MusicalTime::new(extent.as_ratio());
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
    if !crate::core::check_material(resolver, &libraries, library) {
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

/// One elaborated voice item: the term that denotes it, how long it is, and
/// whether a tie is still open at its end.
///
/// The extent is tracked rather than computed, because a term's extent is
/// structural (`seq` sums, `over` maxes, a literal knows its own) and the
/// region facts — slur, phrase, hairpin, tuplet — need it *before* anything is
/// evaluated. Computing it by evaluating would defeat the point of emitting a
/// term at all.
struct Segment {
    term: Term<WrittenTime, ScoreFact>,
    extent: Duration<WrittenTime>,
    /// Exact number of occurrences after this term is instantiated. Keeping
    /// it structurally avoids evaluating a shared term merely to decide
    /// whether evaluating it is safe.
    occurrences: u64,
    /// A tie left open at this segment's end. Merging a tie needs two
    /// occurrences in one value, so a level that has one cannot stay a term.
    tied: bool,
}

impl Segment {
    fn literal(value: VoiceTrack) -> Self {
        let occurrences = u64::try_from(value.occurrences().len()).unwrap_or(u64::MAX);
        Self {
            extent: value.duration(),
            tied: value.occurrences().iter().any(|it| it.payload().tied),
            occurrences,
            term: Term::literal(value),
        }
    }

    fn empty() -> Self {
        Self::literal(empty_segment())
    }
}

/// A body elaborated once and referenced many times, and the bindings that
/// hold them (docs/rules/kernel/06-surface-elaboration.md: "nothing requires duplicating thousands
/// of nodes merely to obey the normalized model").
///
/// Bindings are piece-level because their references are: two voices calling
/// the same motif with the same arguments share one body, and the `let` has to
/// dominate both. They are recorded in completion order — a motif body that
/// itself calls a motif finishes second — which is a valid dependency order by
/// construction, since a motif may only reference motifs declared before it.
#[derive(Default)]
struct Share {
    bindings: Vec<(String, Term<WrittenTime, ScoreFact>)>,
    /// What has already been elaborated, keyed by everything its payloads
    /// depend on, mapping to the binding's name and extent.
    named: std::collections::HashMap<String, (String, Duration<WrittenTime>, u64)>,
    output_meter: crate::core_budget::WorkMeter,
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
    fn close(&self, term: Term<WrittenTime, ScoreFact>) -> Term<WrittenTime, ScoreFact> {
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
    fn evaluate(&self, term: Term<WrittenTime, ScoreFact>) -> VoiceTrack {
        musa_kernel::evaluate_marked(self.close(term), instantiate)
    }

    /// The binding for `key`, or `None` if this body has not been elaborated.
    fn lookup(&self, key: &str) -> Option<(String, Duration<WrittenTime>, u64)> {
        self.named.get(key).cloned()
    }

    /// Record an elaborated body under `key` and return its binding name.
    fn bind(&mut self, key: String, body: Segment) -> String {
        let extent = body.extent;
        let occurrences = body.occurrences;
        let name = self.bind_anonymous(body.term);
        self.named.insert(key, (name.clone(), extent, occurrences));
        name
    }

    fn preflight_output(&mut self, operation: &'static str, amount: u64, span: SourceSpan) -> bool {
        self.output_meter.preflight_output(operation, amount, span).is_some()
    }

    fn reserve_output(&mut self, operation: &'static str, amount: u64, span: SourceSpan) -> bool {
        self.output_meter.output(operation, amount, span).is_some()
    }

    fn report_exhaustion(&self, resolver: &mut Resolver) {
        crate::core::report_exhaustion(resolver, &self.output_meter);
    }

    /// Record a body that nothing else can share and return its binding name.
    ///
    /// A repeat's body is written in exactly one place, so there is no key that
    /// another site could arrive with; keying it would only invite a false
    /// match between two bodies whose payloads differ.
    fn bind_anonymous(&mut self, body: Term<WrittenTime, ScoreFact>) -> String {
        let name = format!("shared{}", self.bindings.len());
        self.bindings.push((name.clone(), body));
        name
    }
}

/// A reference's mark, in `factext`'s word stream:
/// `depth N ("origin" span)? ("scope" scope)? ("via" step+)?`.
///
/// Marks land in the interchange file beside the labels, and they are written
/// by the same writer for that reason: one tokenizer, one escape rule, and a
/// mark that can be read aloud like the facts around it.
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
    crate::factext::reference_mark(&crate::factext::ReferenceMark {
        depth,
        steps: steps.to_vec(),
        origin,
        scope,
    })
}

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

/// A region fact — slur, phrase, hairpin, tuplet — laid over a body.
///
/// `together { body; track extent { occurrence fact from 0 to extent } }`,
/// in that order, which is the order the direct lowerer built and therefore
/// the order the storage-order goldens expect.
fn region(body: Segment, fact: ScoreFact) -> Segment {
    let extent = body.extent;
    let Ok(span) = Span::new(Position::ZERO, extent.reach()) else {
        return body;
    };
    let Ok(mark) = track(extent, vec![Occurrence::new(span, fact)]) else {
        return body;
    };
    let Ok(term) = Term::together(vec![body.term, Term::literal(mark)]) else {
        return Segment::empty();
    };
    Segment {
        term,
        extent,
        occurrences: body.occurrences.saturating_add(1),
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
fn coalesce(terms: impl ExactSizeIterator<Item = Term<WrittenTime, ScoreFact>>) -> Vec<Term<WrittenTime, ScoreFact>> {
    let count = terms.len();
    let mut folded: Vec<Term<WrittenTime, ScoreFact>> = Vec::with_capacity(count);
    let mut run: Vec<VoiceTrack> = Vec::with_capacity(count);
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
fn flush(run: &mut Vec<VoiceTrack>, folded: &mut Vec<Term<WrittenTime, ScoreFact>>) {
    match run.len() {
        0 => {}
        1 => folded.extend(run.drain(..).map(Term::literal)),
        _ => folded.push(Term::literal(follow(std::mem::take(run)))),
    }
}

/// The extent of a sequence of segments: exact rational addition (D2).
fn total_extent(segments: &[Segment]) -> Duration<WrittenTime> {
    beats(
        segments
            .iter()
            .map(|segment| segment.extent.as_ratio())
            .fold(Ratio::ZERO, |total, next| total + next),
    )
}

/// Elaborate one voice: `sequence` of its items (docs/rules/kernel/06).
fn elaborate_voice(
    resolver: &mut Resolver,
    share: &mut Share,
    voice: &musa_language::ast::VoiceDecl,
    declaration: crate::origin::DeclarationId,
    part: u32,
    voice_id: u32,
    core: &crate::core::Program,
    path: Vec<crate::origin::ExpansionStep>,
) -> Segment {
    // Sites written among a voice's own items are numbered from zero in every
    // voice, so the k-th of them is the *same* site in all of them — which is
    // what makes a repeat the page can draw take one count rather than one per
    // voice. See [`crate::ChoicePath`].
    resolver.sites.remove(&crate::ChoicePath::default());
    let root = core.root_music();
    let material = crate::resolve::MaterialCx::root(&root, core.named_music_values(), declaration);
    // A voice a template made stands behind the instance that made it, and
    // every event in it says so.
    let cx = ExpandCx::root(&material, path);
    resolver.cursor = MusicalTime::ZERO;
    let segment = elaborate_place(
        resolver,
        share,
        &voice.items(),
        &cx,
        Scope::Voice { part, voice: voice_id },
        Place::Voice,
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
    cx: &ExpandCx<'_>,
    scope: Scope,
) -> Segment {
    elaborate_place(resolver, share, items, cx, scope, Place::Material)
}

/// Whether these items are a voice's own music or the inside of something
/// reusable.
///
/// The distinction exists for one statement. A `meter` change is a fact about
/// a *place in the piece*; a motif body is elaborated once and can stand at
/// several places, so a meter written inside one would be in force at places
/// that have nothing to do with each other (§2: motif definition ≠ its
/// expansions). Every body is `Material`; only a voice's own item list is
/// `Voice`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    /// A voice's own items, at absolute time.
    Voice,
    /// The inside of a motif, bar, repeat, or any other reusable body.
    Material,
}

fn elaborate_place(
    resolver: &mut Resolver,
    share: &mut Share,
    items: &[VoiceItem],
    cx: &ExpandCx<'_>,
    scope: Scope,
    place: Place,
) -> Segment {
    let opened_at = resolver.cursor;
    let mut at = opened_at;
    let mut segments = Vec::with_capacity(items.len());
    for item in items {
        resolver.cursor = at;
        let segment = elaborate_item(resolver, share, item, cx, scope, place);
        at = MusicalTime::new(at.as_ratio() + segment.extent.as_ratio());
        segments.push(segment);
    }
    // Left as it was found: the caller's loop sets the cursor for each of its
    // own items, and a body that moved it would move the item after itself.
    resolver.cursor = opened_at;
    one_after_another(resolver, share, segments)
}

/// Segments played in order, as one segment.
///
/// Separated from the walk above because `senza` needs it without having a
/// list of items to walk: what it plays is a meter change, a body, and the
/// meter back.
fn one_after_another(resolver: &mut Resolver, share: &Share, segments: Vec<Segment>) -> Segment {
    if segments.is_empty() {
        return Segment::empty();
    }
    if segments.iter().all(|segment| !segment.tied) {
        // The common case, and the one the whole change is for: nothing needs
        // a value, so nothing is evaluated and any sharing below survives
        // into the printed term.
        let extent = total_extent(&segments);
        let occurrences = segments
            .iter()
            .fold(0u64, |count, segment| count.saturating_add(segment.occurrences));
        let parts = coalesce(segments.into_iter().map(|segment| segment.term));
        let term = match <[_; 1]>::try_from(parts) {
            Ok([only]) => only,
            Err(parts) => Term::follow(parts).unwrap_or_else(|_| Term::literal(empty_segment())),
        };
        return Segment {
            term,
            extent,
            occurrences,
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
    Segment::literal(merge_ties(resolver, follow(values)))
}

/// The articulations written on a note or chord statement.
fn articulations_of(resolver: &mut Resolver, names: &[String], span: SourceSpan) -> Vec<crate::Mark> {
    let mut articulations = Vec::new();
    for name in names {
        match crate::Mark::parse(name) {
            // The anchor decides where a mark may be written, so a name that
            // exists in the wrong half of the table gets the sentence that
            // actually helps rather than "is not a mark", which would be a
            // lie about a word the vocabulary contains.
            Some(mark) if mark.slot().is_some() => articulations.push(mark),
            Some(mark) => {
                // The help spells the row's own shape: a pedal needs a block
                // and a rehearsal letter needs its letter, so "write it as a
                // statement" alone would be advice that does not compile.
                let argument = match mark.takes() {
                    crate::marks::Argument::None => String::new(),
                    crate::marks::Argument::Text => " \"…\"".to_owned(),
                    crate::marks::Argument::Number => " 1".to_owned(),
                };
                let tail = match mark.anchor() {
                    crate::marks::Anchor::Span => " { … }",
                    crate::marks::Anchor::Point | crate::marks::Anchor::Note(_) => ";",
                };
                resolver.report(
                    Diagnostic::error(Code::Misplaced, format!("`{name}` is not written on a note"))
                        .at(span, "this mark stands on its own")
                        .help(format!("write it as a statement: `mark {mark}{argument}{tail}`")),
                );
            }
            None => resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("`{name}` is not a mark"))
                    .at(span, "unknown mark")
                    .help(crate::resolve::suggest(name, &crate::marks::note_names(), "marks")),
            ),
        }
    }
    articulations
}

/// `grace { c5 d5 }` — the group as point occurrences at one instant.
///
/// Every occurrence gets `Span::ZERO`, so the group advances the cursor by
/// nothing and the note written after it starts exactly where it would have
/// without the graces. That is what a grace note *is* in notation: it takes no
/// written time, and the time it takes in performance is the profile's answer.
fn elaborate_grace(
    resolver: &mut Resolver,
    statement: &musa_language::ast::GraceStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
) -> Segment {
    let span = resolve::trimmed_span(statement.syntax());
    let notes = statement.notes();
    if notes.is_empty() {
        resolver.report(
            Diagnostic::error(Code::NotAValue, "this grace group has no notes")
                .at(span, "nothing to play")
                .help("write the pitches it leans on: `grace { c5 d5 }`"),
        );
        return Segment::empty();
    }
    // The index orders the group and must be unique within it, so a group too
    // long to index is refused rather than silently collapsed into equal music.
    if u8::try_from(notes.len()).is_err() {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, "this grace group is too long")
                .at(span, "more than 255 notes")
                .note("grace notes are ordered by their place in the group, and musa counts that place in one byte"),
        );
        return Segment::empty();
    }

    let mut occurrences = Vec::with_capacity(notes.len());
    for (index, note) in notes.iter().enumerate() {
        let note_span = resolve::trimmed_span(note.syntax());
        let pitch_text = note.pitch().unwrap_or_default();
        let Some(pitch) = resolve::resolve_pitch(resolver, &pitch_text, note.syntax(), cx) else {
            continue;
        };
        let articulations = articulations_of(resolver, &note.articulations(), note_span);
        let index = u8::try_from(index).unwrap_or(u8::MAX);
        let fact = ScoreFact::new(
            scope,
            FactKind::Grace {
                pitch,
                articulations,
                index,
            },
            origin_of(cx, note_span),
        );
        occurrences.push(Occurrence::new(Span::ZERO, fact));
    }
    Segment::literal(track_or_empty(Duration::ZERO, occurrences))
}

/// `mark <name> [<argument>] ;` or `… { … }`.
///
/// Everything the grammar deliberately did not check happens here, against the
/// vocabulary row: that the name exists, that it may be written as a statement
/// at all, that its argument is the kind the row asks for, and that a span was
/// given a block and a point was not. A row is still the whole cost of a new
/// mark, because none of these checks name one.
fn elaborate_mark(
    resolver: &mut Resolver,
    share: &mut Share,
    statement: &musa_language::ast::MarkStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
) -> Segment {
    use crate::marks::{Anchor, Argument, MarkArgument};

    let span = resolve::trimmed_span(statement.syntax());
    let written = statement.name().unwrap_or_default();
    let Some(mark) = crate::Mark::parse(&written) else {
        resolver.report(
            Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a mark"))
                .at(span, "unknown mark")
                .help(crate::resolve::suggest(
                    &written,
                    &crate::marks::statement_names(),
                    "marks",
                )),
        );
        return Segment::empty();
    };
    if mark.slot().is_some() {
        resolver.report(
            Diagnostic::error(Code::Misplaced, format!("`{mark}` is written on a note"))
                .at(span, "not a statement of its own")
                .help(format!("write it after a duration: `g4 1/4 {mark}`")),
        );
        return Segment::empty();
    }

    let written_argument = match (statement.text(), statement.number()) {
        (Some(text), _) => Some(MarkArgument::Text(text)),
        (None, Some(number)) => number.parse().ok().map(MarkArgument::Number),
        (None, None) => None,
    };
    let argument = match (mark.takes(), written_argument) {
        (Argument::None, None) => None,
        (wanted, Some(given)) if wanted == given.kind() => Some(given),
        (Argument::None, Some(_)) => {
            resolver.report(
                Diagnostic::error(Code::Misplaced, format!("`{mark}` is written on its own"))
                    .at(span, "nothing follows the name"),
            );
            return Segment::empty();
        }
        (wanted, given) => {
            let (wanted, example) = match wanted {
                Argument::Text => ("a name in quotes", format!("mark {mark} \"…\";")),
                Argument::Number => ("a whole number", format!("mark {mark} 1;")),
                Argument::None => ("nothing", format!("mark {mark};")),
            };
            let saw = if given.is_some() { "the wrong kind" } else { "nothing" };
            resolver.report(
                Diagnostic::error(Code::Misplaced, format!("`{mark}` is written with {wanted}"))
                    .at(span, saw)
                    .help(format!("for example `{example}`")),
            );
            return Segment::empty();
        }
    };

    let fact = ScoreFact::new(scope, FactKind::Mark { mark, argument }, origin_of(cx, span));
    match (mark.anchor(), statement.has_block()) {
        (Anchor::Span, true) => {
            let body = elaborate_items(resolver, share, &statement.items(), cx, scope);
            region(body, fact)
        }
        (Anchor::Point, false) => Segment::literal(point(fact)),
        (Anchor::Span, false) => {
            resolver.report(
                Diagnostic::error(Code::Misplaced, format!("`{mark}` covers music"))
                    .at(span, "no music under it")
                    .help(format!("wrap what it covers: `mark {mark} {{ … }}`")),
            );
            Segment::empty()
        }
        (Anchor::Point, true) => {
            resolver.report(
                Diagnostic::error(Code::Misplaced, format!("`{mark}` stands at one place"))
                    .at(span, "it covers nothing")
                    .help(format!("write it on its own: `mark {mark};`")),
            );
            Segment::empty()
        }
        // A note-anchored mark returned above; this arm is here because the
        // match is total.
        (Anchor::Note(_), _) => Segment::empty(),
    }
}

/// The span of what a node actually spells, from its first significant token
/// to its last.
fn written_span(node: &musa_language::SyntaxNode) -> SourceSpan {
    let mut tokens = node
        .descendants_with_tokens()
        .filter_map(musa_language::SyntaxElement::into_token)
        .filter(|token| !token.kind().is_trivia())
        .map(|token| token.text_range());
    let Some(first) = tokens.next() else {
        return resolve::span_of(node);
    };
    let last = tokens.last().unwrap_or(first);
    SourceSpan::new(u32::from(first.start()), u32::from(last.end()))
}

/// The scale `step` reads at this point: the innermost `in scale`, and
/// otherwise the default collection of the key the piece has stated.
///
/// The key's default is a *default*, not a claim: `key c minor` says the
/// signature is three flats, and the natural-minor collection is what a bare
/// `step` then walks. Writing `in scale c harmonic_minor` around the passage
/// is how the other minor collections are asked for, and neither reading
/// changes the other.
fn scale_in_force(resolver: &Resolver, cx: &ExpandCx<'_>) -> Option<crate::scale::Scale> {
    if let Some(scale) = cx.pitch_scale() {
        return Some(scale);
    }
    // The key in force is the latest one written at or before the cursor,
    // falling back to the one the header stated. `Latest` is the same rule the
    // score snapshot reads for the printed signature.
    resolver
        .key_changes
        .iter()
        .filter(|(at, _, _)| *at <= resolver.cursor)
        .max_by_key(|(at, _, _)| *at)
        .map(|(_, key, _)| *key)
        .or(resolver.key)
        .map(crate::scale::signature_scale)
}

/// Finish a note's pitch under the scale in force, reporting exactly what is
/// missing when it cannot be finished.
fn resolve_pitch_term(
    resolver: &mut Resolver,
    term: &crate::core::PitchTerm,
    cx: &ExpandCx<'_>,
    span: SourceSpan,
) -> Option<WrittenPitch> {
    let scale = scale_in_force(resolver, cx);
    match term.resolve(scale) {
        Ok(pitch) => Some(pitch),
        Err(crate::core::PitchTermError::NoScale) => {
            resolver.report(
                Diagnostic::error(Code::Misplaced, "`step` needs a scale in force")
                    .at(span, "no scale reaches this note")
                    .help("wrap the music in `in scale c major { ... }`, or write a `key` before it")
                    .note("an absent scale is never an implicit C major: the coordinate has to come from somewhere"),
            );
            None
        }
        Err(crate::core::PitchTermError::NotInScale { pitch, scale }) => {
            resolver.report(
                Diagnostic::error(Code::NotAValue, format!("`{pitch}` is not a member of `{scale}`"))
                    .at(span, "`step` has no coordinate to move from here")
                    .help(format!(
                        "step from a member of `{scale}`, or put another scale in force with `in scale`"
                    ))
                    .note("membership is spelled: `eb5` and `d#5` are different written pitches"),
            );
            None
        }
        Err(crate::core::PitchTermError::OutOfRange) => {
            resolver.report(
                Diagnostic::error(Code::NotAValue, "this pitch leaves the written range")
                    .at(span, "the step lands outside the coordinates a written pitch can hold"),
            );
            None
        }
    }
}

/// `in scale s { ... }` — Reader `local`.
///
/// It puts one scale in force for the items it encloses, emits no `Key` fact,
/// and adds one `ScaleContext` step to their origins so the page can say why
/// a note was spelled the way it was. A nested `in scale` shadows it; a `use`
/// inside it reads it, which is what lets one saved phrase elaborate
/// differently at two use sites.
fn elaborate_in_scale(
    resolver: &mut Resolver,
    share: &mut Share,
    stmt: &musa_language::ast::InScaleStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
    place: Place,
) -> Segment {
    let span = resolve::trimmed_span(stmt.syntax());
    let Some(scale) = cx.music().scale_at(span) else {
        resolver.error(
            Code::NotAValue,
            "this scale cannot be read",
            span,
            "expected a scale, like `scale c dorian`",
        );
        return Segment::empty();
    };
    elaborate_place(resolver, share, &stmt.items(), &cx.in_scale(scale), scope, place)
}

/// `stack c4 major7/2` — a chord class sounded in close position.
///
/// The one place a chord type becomes notes without a policy being named,
/// and it names one anyway: close position, rooted at the written pitch. The
/// register is the composer's, which is why a pitch class is refused here
/// rather than resolved to some default octave.
fn elaborate_stack(
    resolver: &mut Resolver,
    stmt: &musa_language::ast::StackStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
) -> Segment {
    let node_span = resolve::trimmed_span(stmt.syntax());
    let Some(duration) = resolve_scaled_duration(resolver, stmt.syntax(), cx) else {
        return Segment::empty();
    };
    let Some(root) = stmt.root().as_deref().and_then(WrittenPitch::parse) else {
        let (message, note) = if stmt.root_is_class() {
            (
                "a stacked chord needs a register",
                "`stack c4 major7/2` sounds notes, and a pitch class chooses no octave",
            )
        } else {
            (
                "a stacked chord needs a written root",
                "the root is a pitch: `c4`, `g#3`, `bb5`",
            )
        };
        resolver.report(
            Diagnostic::error(Code::NotAValue, message)
                .at(node_span, "expected a written pitch here")
                .note(note),
        );
        return Segment::empty();
    };
    let word = stmt.chord_type().unwrap_or_default();
    let Some(kind) = crate::chord::ChordType::named(&word) else {
        resolver.report(
            Diagnostic::error(Code::UnknownName, format!("unknown chord type `{word}`"))
                .at(node_span, "not a named chord type")
                .note("the same words `chord c major7` uses; a chord symbol above the staff is a separate annotation"),
        );
        return Segment::empty();
    };
    let class = crate::chord::ChordClass::new(root.pitch_class(), kind);
    let Ok(voicing) = crate::chord::Voicing::close_position(class, root) else {
        resolver.report(
            Diagnostic::error(Code::NotAValue, format!("`{class}` cannot be stacked on `{root}`"))
                .at(node_span, "the close-position policy declined")
                .note("stacking starts from the root; write the voicing out to choose another bass"),
        );
        return Segment::empty();
    };
    let articulations = articulations_of(resolver, &stmt.articulations(), node_span);
    sounded_voicing(
        resolver,
        &voicing,
        &duration,
        &articulations,
        stmt.tied(),
        cx,
        scope,
        node_span,
    )
}

/// The score facts one voicing sounds, as one simultaneous segment.
///
/// Shared by the `stack` sugar and by `play`, so the two cannot disagree
/// about what a voicing sounds. The enclosing transposition applies to each
/// pitch exactly as it does to a written chord: a voicing is notes, and notes
/// move.
#[expect(
    clippy::too_many_arguments,
    reason = "one sounded chord needs every one of these, and grouping them would name nothing"
)]
fn sounded_voicing(
    resolver: &mut Resolver,
    voicing: &crate::chord::Voicing,
    duration: &NotatedDuration,
    articulations: &[crate::Mark],
    tied: bool,
    cx: &ExpandCx<'_>,
    scope: Scope,
    node_span: SourceSpan,
) -> Segment {
    let mut facts = Vec::new();
    for pitch in voicing.pitches() {
        let Some(pitch) = apply_intervals(resolver, pitch, cx, node_span) else {
            return Segment::empty();
        };
        let mut fact = ScoreFact::new(
            scope,
            FactKind::Note {
                pitch,
                duration: duration.clone(),
                articulations: articulations.to_vec(),
                free: None,
            },
            origin_of(cx, node_span),
        );
        fact.tied = tied;
        facts.push(fact);
    }
    let span = span_of_duration(duration);
    Segment::literal(track_or_empty(
        span.duration(),
        facts.into_iter().map(|fact| Occurrence::new(span, fact)).collect(),
    ))
}

/// Elaborate one item; malformed items elaborate to the empty segment
/// `(0, ∅)` — the direct lowerer's `continue` (diagnostic already emitted).
fn elaborate_item(
    resolver: &mut Resolver,
    share: &mut Share,
    item: &VoiceItem,
    cx: &ExpandCx<'_>,
    scope: Scope,
    place: Place,
) -> Segment {
    match item {
        VoiceItem::Tempo(stmt) => elaborate_tempo(resolver, stmt, cx, place),
        VoiceItem::Meter(stmt) => elaborate_meter(resolver, stmt, cx, place),
        VoiceItem::Senza(stmt) => elaborate_senza(resolver, share, stmt, cx, scope, place),
        VoiceItem::Key(stmt) => elaborate_key(resolver, stmt, cx, place),
        VoiceItem::InScale(stmt) => elaborate_in_scale(resolver, share, stmt, cx, scope, place),
        VoiceItem::Stack(stmt) => elaborate_stack(resolver, stmt, cx, scope),
        VoiceItem::Clef(stmt) => elaborate_clef(resolver, stmt, cx, scope, place),
        VoiceItem::Mobile(stmt) => elaborate_mobile(resolver, share, stmt, cx, scope),
        VoiceItem::Improvise(stmt) => elaborate_improvise(resolver, stmt, cx, scope),
        VoiceItem::Grace(grace) => elaborate_grace(resolver, grace, cx, scope),
        VoiceItem::Assert(stmt) => elaborate_assert(resolver, share, stmt, cx, scope, place),
        VoiceItem::Note(note) => {
            let Some(duration) = resolve_scaled_duration(resolver, note.syntax(), cx) else {
                return Segment::empty();
            };
            let span = resolve::trimmed_span(note.syntax());
            let pitch = if let Some(term) = cx.music().pitch_at(span) {
                // The statement's span is the lookup key; the diagnostic wants
                // the pitch the composer wrote, not the whole event.
                let written = note.pitch_expr().map_or(span, |node| written_span(&node));
                let Some(pitch) = resolve_pitch_term(resolver, term, cx, written) else {
                    return Segment::empty();
                };
                // A computed pitch enters the transposition stack exactly as a
                // written one does: the enclosing `transpose` block moves the
                // sound, and the scale it stepped through is untouched.
                let Some(pitch) = resolve::apply_intervals(resolver, pitch, cx, note.syntax()) else {
                    return Segment::empty();
                };
                pitch
            } else {
                let pitch_text = note.pitch().unwrap_or_default();
                let Some(pitch) = resolve::resolve_pitch(resolver, &pitch_text, note.syntax(), cx) else {
                    return Segment::empty();
                };
                pitch
            };
            let articulations = articulations_of(resolver, &note.articulations(), span);
            let origin = origin_of(cx, span);
            let (duration, free) = held(resolver, note.held_to().as_deref(), cx, duration, span);
            let mut fact = ScoreFact::new(
                scope,
                FactKind::Note {
                    pitch,
                    duration: duration.clone(),
                    articulations,
                    free,
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
            let (duration, free) = held(resolver, rest.held_to().as_deref(), cx, duration, span);
            Segment::literal(single(
                &duration,
                ScoreFact::new(
                    scope,
                    FactKind::Rest {
                        duration: duration.clone(),
                        articulations: Vec::new(),
                        free,
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
                match WrittenPitch::parse(&text).map(|pitch| apply_intervals(resolver, pitch, cx, node_span)) {
                    Some(Some(pitch)) => {
                        let origin = origin_of(cx, node_span);
                        let mut fact = ScoreFact::new(
                            scope,
                            FactKind::Note {
                                pitch,
                                duration: duration.clone(),
                                articulations: articulations.clone(),
                                free: None,
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
                                .note("a pitch is a letter, an optional `#` or `b`, and an octave: `c4`, `g#5`, `bb3`"),
                        );
                    }
                }
            }
            let span = span_of_duration(&duration);
            Segment::literal(track_or_empty(
                span.duration(),
                pitches
                    .into_iter()
                    .map(|payload| Occurrence::new(span, payload))
                    .collect(),
            ))
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
            elaborate_items(resolver, share, &transpose.items(), &cx.transposed(interval), scope)
        }
        VoiceItem::Repeat(repeat) => elaborate_repeat(resolver, share, repeat, cx, scope),
        VoiceItem::Ending(ending) => {
            // The repeat arm takes the endings that belong to it, so anything
            // arriving here has no pass to belong to.
            let span = resolve::trimmed_span(ending.syntax());
            resolver.report(
                Diagnostic::error(Code::Misplaced, "an ending belongs to a repeat")
                    .at(span, "no `repeat` encloses this")
                    .help("put the passage in `repeat 2 { … }` and this ending at the end of it")
                    .note("an ending says which pass to play, and outside a repeat there is only one pass"),
            );
            elaborate_items(resolver, share, &ending.items(), cx, scope)
        }
        VoiceItem::Bar(bar) => elaborate_bar(resolver, share, bar, cx, scope, place),
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
        VoiceItem::Mark(statement) => elaborate_mark(resolver, share, statement, cx, scope),
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
            let inner = cx.scaled(Ratio::new(i64::from(den), i64::from(num)));
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
            let inner = cx.with_step(ExpansionStep::Stretch(factor));
            let segment = elaborate_items(resolver, share, &stretch.items(), &inner, scope);
            stretch_segment(share, segment, factor)
        }
        VoiceItem::Retrograde(retrograde) => {
            let inner = cx.with_step(ExpansionStep::Retrograde);
            let segment = elaborate_items(resolver, share, &retrograde.items(), &inner, scope);
            retrograde_segment(share, segment)
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
            let inner = cx.with_step(ExpansionStep::Inversion { axis: text.clone() });
            let segment = elaborate_items(resolver, share, &invert.items(), &inner, scope);
            invert_segment(resolver, share, segment, axis, &text)
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
fn resolve_scaled_duration(resolver: &mut Resolver, node: &SyntaxNode, cx: &ExpandCx<'_>) -> Option<NotatedDuration> {
    let duration = resolve::resolve_duration(resolver, node, cx)?;
    Some(if cx.scale() == Ratio::ONE {
        duration
    } else {
        duration.scaled(cx.scale())
    })
}

/// A written duration and the freedom written on it.
///
/// `g4/4 to 2/1` is a quarter the performer may hold to a double whole. The
/// returned duration is what the note *sounds* — the realization's answer, so
/// everything after it lands where it should — and the [`crate::score::FreeDuration`]
/// is what recovers the symbol. Roadmap §2's row with both values kept, rather
/// than one standing in for the other.
fn held(
    resolver: &mut Resolver,
    to: Option<&str>,
    cx: &ExpandCx<'_>,
    duration: NotatedDuration,
    span: SourceSpan,
) -> (NotatedDuration, Option<crate::score::FreeDuration>) {
    let Some(to) = to else {
        return (duration, None);
    };
    let Some(most) = crate::resolve::parse_ratio(to) else {
        resolver.report(
            Diagnostic::error(Code::NotAValue, format!("`{to}` is not a duration"))
                .at(span, "expected the longest this note may be held")
                .help("write a duration such as `2/1`"),
        );
        return (duration, None);
    };
    let most = crate::MusicalDuration::new(most * cx.scale());
    if most.as_ratio() < duration.value.as_ratio() {
        resolver.report(
            Diagnostic::error(Code::NotAValue, "a held note counts upwards")
                .at(span, format!("`{}` is longer than `{to}`", duration.spelling))
                .help("write the written value first and the longest hold second"),
        );
        return (duration, None);
    }
    let least = duration.value;
    let sounds = resolver.decide_duration(cx.choice(), least.as_ratio(), most.as_ratio(), span);
    (
        NotatedDuration {
            value: crate::MusicalDuration::new(sounds),
            spelling: duration.spelling,
            pieces: vec![crate::MusicalDuration::new(sounds)],
        },
        Some(crate::score::FreeDuration { least, most }),
    )
}

/// `mobile { a; b; c; }` — its fragments in an order the performance chose.
///
/// The mobile rule, applied a third time: the timeline holds the fragments in
/// the order they are played, and one region fact carries the instruction the
/// page prints over them.
fn elaborate_mobile(
    resolver: &mut Resolver,
    share: &mut Share,
    mobile: &musa_language::ast::MobileStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
) -> Segment {
    let span = resolve::trimmed_span(mobile.syntax());
    let names = mobile.fragments();
    if names.len() < 2 {
        resolver.report(
            Diagnostic::error(Code::NotAValue, "a mobile arranges at least two fragments")
                .at(span, format!("this one lists {}", names.len()))
                .help("write the fragment out instead, or add the ones it is arranged with"),
        );
        return Segment::empty();
    }
    let order = resolver.decide_order(cx.choice(), &names, span);
    let name_tokens = mobile.fragment_tokens();
    let mut played: Vec<Segment> = Vec::with_capacity(names.len());
    for index in &order {
        let Some(name) = names.get(*index as usize) else {
            continue;
        };
        // The list and its tokens are read off the same tokens, so they
        // cannot disagree about which span this name is.
        let use_span = name_tokens.get(*index as usize).map_or(span, resolve::source_span_of);
        played.push(elaborate_fragment(resolver, share, name, cx, scope, span, use_span));
    }
    let fact = ScoreFact::new(
        scope,
        FactKind::Mobile {
            fragments: names,
            order,
        },
        origin_of(cx, span),
    );
    region(sequence_of(played), fact)
}

/// A run of segments played one after another, as one segment.
///
/// The untied path of [`elaborate_place`], reached from the places that build
/// their own order rather than reading it off the page. A tie cannot cross a
/// boundary here: the material either side is chosen by the performance, so a
/// tie into it would be a tie to a note that may not follow.
fn sequence_of(segments: Vec<Segment>) -> Segment {
    if segments.is_empty() {
        return Segment::empty();
    }
    let extent = total_extent(&segments);
    let occurrences = segments
        .iter()
        .fold(0u64, |count, segment| count.saturating_add(segment.occurrences));
    let parts = coalesce(segments.into_iter().map(|segment| segment.term));
    let term = match <[_; 1]>::try_from(parts) {
        Ok([only]) => only,
        Err(parts) => Term::follow(parts).unwrap_or_else(|_| Term::literal(empty_segment())),
    };
    Segment {
        term,
        extent,
        occurrences,
        tied: false,
    }
}

/// `improvise 8/1 over "Dm7 | G7";` — a frame with unnotated contents.
///
/// It occupies its own length of real time so everything after it lands where
/// it should, and it sounds as silence: musa does not improvise, and a comping
/// pattern here would be musa inventing notes.
fn elaborate_improvise(
    resolver: &mut Resolver,
    stmt: &musa_language::ast::ImproviseStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
) -> Segment {
    let span = resolve::trimmed_span(stmt.syntax());
    let Some(length) = stmt.duration().as_deref().and_then(crate::resolve::parse_ratio) else {
        resolver.report(
            Diagnostic::error(Code::NotAValue, "an improvised frame needs a length")
                .at(span, "expected a duration")
                .help("write how long it lasts: `improvise 8/1 over \"Dm7 | G7\";`"),
        );
        return Segment::empty();
    };
    let extent = beats(length * cx.scale());
    let fact = ScoreFact::new(scope, FactKind::Improvise { over: stmt.over() }, origin_of(cx, span));
    let Ok(region) = Span::new(Position::ZERO, extent.reach()) else {
        return Segment::empty();
    };
    Segment {
        term: Term::literal(track_or_empty(extent, vec![Occurrence::new(region, fact)])),
        extent,
        occurrences: 1,
        tied: false,
    }
}

/// The number of passes a `repeat` takes — written, or chosen — and the range
/// it was chosen from.
///
/// The range is returned because the page has to print it. `repeat 4 to 16`
/// means *play this between four and sixteen times*, and a page that printed
/// only the count this performance drew would have replaced the composer's
/// instruction with one reading of it. The rule again: the timeline
/// holds every pass, the page prints the instruction once.
///
/// `None` is a refusal, not a count of zero: a backwards range is a mistake
/// about the music rather than an empty repeat, and compiling it as silence
/// would hide it.
fn ranged_count(
    resolver: &mut Resolver,
    repeat: &musa_language::ast::RepeatStmt,
    cx: &ExpandCx<'_>,
    least: u32,
) -> Option<(u32, Option<(u32, u32)>)> {
    let Some(most) = repeat.most() else {
        return Some((least, None));
    };
    let span = resolve::trimmed_span(repeat.syntax());
    let Some(most) = most.parse::<u32>().ok().filter(|most| *most >= least) else {
        resolver.report(
            Diagnostic::error(Code::NotAValue, "a repeat range counts upwards")
                .at(span, format!("`{least} to {most}` never happens"))
                .help("write the smaller number first"),
        );
        return None;
    };
    let count = resolver.decide_count(cx.choice(), least, most, span).1;
    Some((count, Some((least, most))))
}

/// `repeat n { … }` and `repeat n to m { … }`, with or without endings.
///
/// The body is stated once and referenced once per pass; the mark on each
/// reference is what tells the passes apart (T6). That the *page* can print the
/// body once follows from the same structure rather than adding to it — one
/// statement, two projections, which is roadmap §2's own example.
///
/// The ranged form is the piece leaving the count to the performance. It is
/// resolved *here*, before a term exists, so everything below this function is
/// the ordinary exact repeat — which is the whole of
/// `docs/rules/kernel/11-realization.md`'s design in one place.
fn elaborate_repeat(
    resolver: &mut Resolver,
    share: &mut Share,
    repeat: &musa_language::ast::RepeatStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
) -> Segment {
    let least: u32 = repeat.count().and_then(|text| text.parse().ok()).unwrap_or(0);
    let Some((count, range)) = ranged_count(resolver, repeat, cx, least) else {
        return Segment::empty();
    };
    if count == 0 {
        return Segment::empty();
    }
    let items = repeat.items();
    let (body_items, endings) = split_endings(resolver, repeat, &items, count);
    let body = elaborate_items(resolver, share, &body_items, cx, scope);
    let played: Vec<Segment> = endings
        .iter()
        .map(|ending| elaborate_items(resolver, share, &ending.items(), cx, scope))
        .collect();

    // A tie open at a body's or an ending's end would join into the next
    // pass's first note, and merging needs both in one value. Rare, and
    // expanding is exactly what happened before — and an expanded repeat is
    // engraved the way it is written, because no repeat fact reaches the page.
    if body.tied || played.iter().any(|segment| segment.tied) {
        return expanded_repeat(resolver, share, cx, body, &played, count);
    }

    let output = body
        .occurrences
        .saturating_mul(u64::from(count))
        .saturating_add(repeated_ending_occurrences(&played, count))
        .saturating_add(if played.is_empty() { 0 } else { u64::from(count) })
        .saturating_add(1);
    if !share.preflight_output("expanding a repeat", output, resolve::trimmed_span(repeat.syntax())) {
        share.report_exhaustion(resolver);
        return Segment::empty();
    }

    let mark = |iteration: u32| {
        mark_of(
            cx.path().len(),
            &[ExpansionStep::RepeatIteration(iteration)],
            None,
            None,
        )
    };
    let body_occurrences = body.occurrences;
    let body_name = share.bind_anonymous(body.term);
    let ending_names: Vec<(String, Duration<WrittenTime>, u64)> = played
        .into_iter()
        .map(|segment| (share.bind_anonymous(segment.term), segment.extent, segment.occurrences))
        .collect();
    let mut passes: Vec<Term<WrittenTime, ScoreFact>> = Vec::with_capacity(count as usize);
    let mut extent = Ratio::ZERO;
    let mut occurrences = 0u64;
    for iteration in 0..count {
        passes.push(Term::var_marked(&body_name, mark(iteration)));
        extent += body.extent.as_ratio();
        occurrences = occurrences.saturating_add(body_occurrences);
        // Fewer endings than passes is legal: the last one covers the rest,
        // which is what `1.–3.` means on a volta bracket.
        let Some((bracket, name, length, ending_occurrences)) = ending_of(&ending_names, iteration) else {
            continue;
        };
        let origin = origin_of(cx, resolve::trimmed_span(repeat.syntax()));
        let reference = Segment {
            term: Term::var_marked(name, mark(iteration)),
            extent: length,
            occurrences: ending_occurrences,
            tied: false,
        };
        let fact = ScoreFact::new(
            scope,
            FactKind::Ending {
                bracket,
                pass: iteration.saturating_add(1),
            },
            origin,
        );
        passes.push(region(reference, fact).term);
        extent += length.as_ratio();
        occurrences = occurrences.saturating_add(ending_occurrences.saturating_add(1));
    }
    let whole = Segment {
        term: Term::follow(passes).unwrap_or_else(|_| Term::literal(empty_segment())),
        extent: beats(extent),
        occurrences,
        tied: false,
    };
    let origin = origin_of(cx, resolve::trimmed_span(repeat.syntax()));
    region(
        whole,
        ScoreFact::new(scope, FactKind::Repeat { times: count, range }, origin),
    )
}

/// The ending a given pass plays — its 1-based bracket, binding, and length —
/// by the rule that the last one covers every pass after it.
fn ending_of(
    endings: &[(String, Duration<WrittenTime>, u64)],
    iteration: u32,
) -> Option<(u32, &str, Duration<WrittenTime>, u64)> {
    let index = (iteration as usize).min(endings.len().checked_sub(1)?);
    let bracket = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
    endings
        .get(index)
        .map(|(name, length, occurrences)| (bracket, name.as_str(), *length, *occurrences))
}

/// The fallback for a repeat that cannot be shared: every pass written out,
/// exactly as it was before sharing existed.
fn expanded_repeat(
    resolver: &mut Resolver,
    share: &mut Share,
    cx: &ExpandCx<'_>,
    body: Segment,
    endings: &[Segment],
    count: u32,
) -> Segment {
    let body_output = body.occurrences.saturating_mul(u64::from(count));
    let ending_output = repeated_ending_occurrences(endings, count);
    let output = body_output.saturating_add(ending_output);
    let span = cx.music().definition_span;
    if !share.preflight_output("expanding a tied repeat", output, span) {
        share.report_exhaustion(resolver);
        return Segment::empty();
    }
    let body = share.evaluate(body.term);
    let played: Vec<VoiceTrack> = endings
        .iter()
        .map(|segment| share.evaluate(segment.term.clone()))
        .collect();
    let mut segments = Vec::with_capacity(count as usize);
    for iteration in 0..count {
        let mark = mark_of(
            cx.path().len(),
            &[ExpansionStep::RepeatIteration(iteration)],
            None,
            None,
        );
        let mut copy = body.clone();
        instantiate(&mark, &mut copy);
        segments.push(copy);
        let index = (iteration as usize).min(played.len().saturating_sub(1));
        if let Some(ending) = played.get(index) {
            let mut copy = ending.clone();
            instantiate(&mark, &mut copy);
            segments.push(copy);
        }
    }
    Segment::literal(follow(segments))
}

fn repeated_ending_occurrences(endings: &[Segment], count: u32) -> u64 {
    if endings.is_empty() {
        0
    } else {
        let distinct = usize::try_from(count).unwrap_or(usize::MAX).min(endings.len());
        let initial = endings
            .get(..distinct)
            .unwrap_or_default()
            .iter()
            .fold(0u64, |sum, ending| sum.saturating_add(ending.occurrences));
        let remaining = u64::from(count).saturating_sub(u64::try_from(distinct).unwrap_or(u64::MAX));
        initial.saturating_add(remaining.saturating_mul(endings.last().map_or(0, |ending| ending.occurrences)))
    }
}

/// A repeat's body and its endings, with everything wrong about them said.
///
/// Endings are collected wherever they are written rather than only from the
/// end, so a misplaced one still plays the notes in it and the composer reads
/// one complaint about placement instead of a silent disappearance.
fn split_endings(
    resolver: &mut Resolver,
    repeat: &musa_language::ast::RepeatStmt,
    items: &[VoiceItem],
    count: u32,
) -> (Vec<VoiceItem>, Vec<musa_language::ast::EndingStmt>) {
    let mut body = Vec::with_capacity(items.len());
    let mut endings: Vec<musa_language::ast::EndingStmt> = Vec::new();
    // The last ending written so far, until something that is not an ending
    // follows it — which is the one thing about their placement that is wrong.
    let mut open: Option<musa_language::ast::EndingStmt> = None;
    for item in items {
        let VoiceItem::Ending(stmt) = item else {
            if let Some(before) = open.take() {
                resolver.report(
                    Diagnostic::error(Code::Misplaced, "an ending is the last thing in a repeat")
                        .at(resolve::trimmed_span(before.syntax()), "music is written after this")
                        .help("move the endings below everything the passes have in common")
                        .note("every pass plays the body, then its ending, so the body comes first"),
                );
            }
            body.push(item.clone());
            continue;
        };
        let written: u32 = stmt.number().and_then(|text| text.parse().ok()).unwrap_or(0);
        let expected = u32::try_from(endings.len().saturating_add(1)).unwrap_or(u32::MAX);
        if written != expected {
            resolver.report(
                Diagnostic::error(
                    Code::Misplaced,
                    format!("this ending is pass {expected}, not pass {written}"),
                )
                .at(number_span(stmt), format!("expected `ending {expected}`"))
                .help("number the endings from 1, in the order they are played"),
            );
        }
        if expected > count {
            let mut diagnostic =
                Diagnostic::error(Code::Misplaced, format!("this repeat never reaches pass {expected}"))
                    .at(number_span(stmt), "no pass plays this")
                    .help(format!("write `repeat {expected}`, or delete this ending"));
            if let Some((start, end)) = repeat.count_span() {
                diagnostic = diagnostic.also(SourceSpan::new(start, end), format!("{count} passes"));
            }
            resolver.report(diagnostic);
            continue;
        }
        open = Some(stmt.clone());
        endings.push(stmt.clone());
    }
    (body, endings)
}

/// Where an ending's number is written, falling back to the whole statement.
fn number_span(ending: &musa_language::ast::EndingStmt) -> SourceSpan {
    ending.number_span().map_or_else(
        || resolve::trimmed_span(ending.syntax()),
        |(start, end)| SourceSpan::new(start, end),
    )
}

/// A bar: its contents, checked against the meter, and then let through.
///
/// The braces are erased. A bar contributes no occurrence, no payload, and no
/// time of its own — the kernel's ontology has no bar in it, and
/// the notation plan already knows where the barlines fall. What the braces
/// contribute is the check, and, with a name on the front, the fact that the
/// same measure can be played again from anywhere below.
///
/// The notes here stay *authored*: they are written at this place, and a bar
/// that turned its own contents into an expansion of itself would make a
/// composer's own notes read as generated in Origin view.
fn elaborate_bar(
    resolver: &mut Resolver,
    share: &mut Share,
    bar: &musa_language::ast::BarStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
    place: Place,
) -> Segment {
    let at = resolver.cursor;
    let before = errors_so_far(resolver);
    // An *unnamed* bar written among a voice's own items is played exactly
    // once and at one place, so it is still that place: a clef change belongs
    // in the middle of a measure, and a measure is a `bar`. A named bar can be
    // answered from another voice, which makes it material like a motif.
    let inside = if bar.name().is_none() { place } else { Place::Material };
    let named = bar.name().map(|name| cx.in_bar(&name));
    let body = elaborate_place(
        resolver,
        share,
        &bar.items(),
        named.as_ref().unwrap_or(cx),
        scope,
        inside,
    );
    // A bar whose contents did not resolve has no length worth reporting, and
    // "this bar is 1/4 short" underneath "`sigb` is not a pitch" is the second
    // sentence of a two-sentence complaint about one mistake.
    if errors_so_far(resolver) == before {
        resolver.obligations.push(PendingClaim {
            claim: crate::assert::Claim::FillsMeter,
            passage: crate::assert::Passage {
                span: resolve::trimmed_span(bar.syntax()),
                at,
                extent: crate::MusicalDuration::new(body.extent.as_ratio()),
                content_end: bar.content_end(),
                // A bar's claim is about how long its contents are, and
                // nothing else, so the notes are never gathered for one: the
                // evaluation an assertion pays for buys a bar nothing.
                notes: Vec::new(),
                noun: "bar",
            },
            scope,
        });
    }
    body
}

/// An assertion: its body, unchanged, and a claim recorded about it.
///
/// The music is what was written. An assertion adds no occurrence, no payload,
/// and no time — the only trace it leaves in the result is one
/// [`ExpansionStep::Assertion`] on the facts underneath it, which is provenance
/// and therefore invisible to `≈facts` (law 13). That is the identity an
/// assertion asks for: a claim that holds gives back exactly the passage it was
/// written on.
///
/// The body is evaluated here, once, and the evaluation is thrown away after
/// the notes are read out of it. It is worth saying why, since evaluating
/// costs the sharing this compiler works to keep: a claim about *which notes
/// sound* cannot be answered from a term that has not been instantiated, and
/// the alternative — a structural approximation over the term — would be a
/// second, weaker semantics that could disagree with the piece the composer
/// gets. An assertion body is a passage a person wrote out, so the cost is
/// bounded by what they were willing to type.
fn elaborate_assert(
    resolver: &mut Resolver,
    share: &mut Share,
    assertion: &musa_language::ast::AssertStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
    place: Place,
) -> Segment {
    let at = resolver.cursor;
    let span = resolve::trimmed_span(assertion.syntax());
    let before = errors_so_far(resolver);
    let claim = cx.music().claim_at(span).cloned();
    // The step goes on before the body is elaborated, exactly as `in scale`
    // puts its own on: a fact carries the steps that were in force when it was
    // made, and an assertion is in force over everything inside its braces.
    let inner = cx.with_step(ExpansionStep::Assertion {
        // The checked claim when there is one, so the step reads back with its
        // arguments; the bare name when the claim did not survive checking, so
        // that a fact under a misspelled assertion still says what was written
        // over it.
        claim: claim
            .as_ref()
            .map_or_else(|| assertion.claim().unwrap_or_default(), crate::assert::Claim::describe),
    });
    let body = elaborate_place(resolver, share, &assertion.items(), &inner, scope, place);
    // A passage whose notes did not resolve has no claim worth testing:
    // "`fs5` is not in scale c major" underneath "`sigb` is not a pitch" is
    // the second sentence of a two-sentence complaint about one mistake. The
    // same rule a bar follows, for the same reason.
    let Some(claim) = claim.filter(|_| errors_so_far(resolver) == before) else {
        return body;
    };
    let sounded = share.evaluate(body.term.clone());
    let notes = sounded
        .occurrences()
        .iter()
        .filter_map(|occurrence| {
            Some(crate::assert::Sounded {
                pitch: occurrence.payload().pitch_of()?,
                start: crate::MusicalTime::new(occurrence.span().start().as_ratio()),
                end: crate::MusicalTime::new(occurrence.span().end().as_ratio()),
                // The span of the note itself and not of whatever played it.
                // A note generated from a motif is written in the motif, and
                // that is where a composer goes to change its pitch.
                at: occurrence.payload().origin.definition_span,
            })
        })
        .collect();
    resolver.obligations.push(PendingClaim {
        claim,
        passage: crate::assert::Passage {
            span,
            at,
            extent: crate::MusicalDuration::new(body.extent.as_ratio()),
            content_end: assertion.content_end(),
            notes,
            noun: "passage",
        },
        scope,
    });
    body
}

/// A `meter` written where the music reaches it.
///
/// It contributes a point occurrence to the piece — `Scope::Piece`, because a
/// meter written in one voice is the piece's meter from there (per-voice
/// meter is polymeter, a separate construct) — and nothing else. Where the
/// barlines then fall is [`crate::BarLines`]'s answer, computed once every
/// voice has been read, because whether *this* change lands on a barline
/// depends on the changes before it.
fn elaborate_meter(
    resolver: &mut Resolver,
    stmt: &musa_language::ast::MeterStmt,
    cx: &ExpandCx<'_>,
    place: Place,
) -> Segment {
    let span = resolve::trimmed_span(stmt.syntax());
    if place == Place::Material {
        resolver.report(misplaced_context("meter", span));
        return Segment::empty();
    }
    let Some(meter) = resolve::parse_meter(stmt) else {
        resolver.error(
            Code::NotAValue,
            "this meter cannot be read",
            span,
            "expected `4/4`, or `none`",
        );
        return Segment::empty();
    };
    resolver.meter_written = true;
    meter_change(resolver, meter, cx, span)
}

/// `senza { ... }` — an unmeasured stretch with a scope.
///
/// It is `meter none;`, the body, and the meter back, which is what makes it
/// sugar and not a mechanism: the barlines stop for exactly as long as the
/// braces say, and the meter that resumes is the one that was in force, so
/// there is no second meter to keep in step with the first.
fn elaborate_senza(
    resolver: &mut Resolver,
    share: &mut Share,
    stmt: &musa_language::ast::SenzaStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
    place: Place,
) -> Segment {
    let span = resolve::trimmed_span(stmt.syntax());
    if place == Place::Material {
        resolver.report(misplaced_context("senza", span));
        return Segment::empty();
    }
    let resume = prevailing_meter(resolver);
    resolver.meter_written = true;
    let opened = meter_change(resolver, crate::Meter::NONE, cx, span);
    // The cursor moves for the body and for the meter that closes it, exactly
    // as the item walk moves it: the restoring change belongs where the
    // unmeasured music stopped, not where it started.
    let opened_at = resolver.cursor;
    let body = elaborate_place(resolver, share, &stmt.items(), cx, scope, place);
    resolver.cursor = MusicalTime::new(opened_at.as_ratio() + body.extent.as_ratio());
    let closed = meter_change(resolver, resume, cx, span);
    resolver.cursor = opened_at;
    one_after_another(resolver, share, vec![opened, body, closed])
}

/// The meter in force where the cursor stands.
///
/// Read from the changes already written rather than from a field, because a
/// meter is a fact with a place: the answer at bar 9 is not the answer at bar
/// 1, and only the changes know that.
fn prevailing_meter(resolver: &Resolver) -> Meter {
    resolver
        .meter_changes
        .iter()
        .filter(|(at, _, _)| *at <= resolver.cursor)
        .max_by_key(|(at, _, _)| *at)
        .map_or(resolver.meter, |(_, meter, _)| *meter)
}

/// The occurrence a meter change contributes, and the record `resolve_meters`
/// folds into barlines afterwards.
fn meter_change(resolver: &mut Resolver, meter: Meter, cx: &ExpandCx<'_>, span: SourceSpan) -> Segment {
    resolver.meter_changes.push((resolver.cursor, meter, span));
    Segment::literal(point(ScoreFact::new(
        Scope::Piece,
        FactKind::Meter {
            numerator: meter.numerator(),
            denominator: meter.denominator(),
        },
        origin_of(cx, span),
    )))
}

/// A `tempo` written where the music reaches it.
///
/// `Scope::Piece`, like the meter, and a point rather than a region: a tempo
/// marking is in force until the next one, which is what a `ContextTrack`
/// already means by a stretch that starts here.
///
/// It is refused inside material for the reason every context statement is
/// (`context.rs`): a motif body sits at several absolute times, and a tempo
/// change written into one would change the piece wherever the motif landed.
fn elaborate_tempo(
    resolver: &mut Resolver,
    stmt: &musa_language::ast::TempoStmt,
    cx: &ExpandCx<'_>,
    place: Place,
) -> Segment {
    let span = resolve::trimmed_span(stmt.syntax());
    if place == Place::Material {
        resolver.report(misplaced_context("tempo", span));
        return Segment::empty();
    }
    Segment::literal(point(ScoreFact::new(
        Scope::Piece,
        resolve::tempo_fact(resolver, stmt),
        origin_of(cx, span),
    )))
}

/// A `key` written where the music reaches it.
///
/// `Scope::Piece`, like the meter and for the same reason: a modulation is
/// something the piece does. Which key a *part* reads is then the `Latest`
/// rule's answer (`context.rs`), so a part that opened in its own key keeps
/// it until the piece says otherwise and follows the piece from there — the
/// viola case the `Latest` rule was written for.
fn elaborate_key(
    resolver: &mut Resolver,
    stmt: &musa_language::ast::KeyStmt,
    cx: &ExpandCx<'_>,
    place: Place,
) -> Segment {
    let span = resolve::trimmed_span(stmt.syntax());
    if place == Place::Material {
        resolver.report(misplaced_context("key", span));
        return Segment::empty();
    }
    // Written out, or named: `key k;` inside a template's voice is the value
    // the instance supplied, evaluated before any of this ran.
    let Some(key) = resolve::parse_key(stmt).or_else(|| cx.music().key_at(span)) else {
        resolver.error(
            Code::NotAValue,
            "this key cannot be read",
            span,
            "expected a note and a mode, like `a minor`",
        );
        return Segment::empty();
    };
    resolver.key_changes.push((resolver.cursor, key, span));
    Segment::literal(point(ScoreFact::new(
        Scope::Piece,
        FactKind::Key {
            tonic: key.tonic(),
            mode: key.mode(),
        },
        origin_of(cx, span),
    )))
}

/// A `clef` written where the music reaches it.
///
/// `Scope::Part`, because a part's clef is the part's business — the
/// `Override` rule — and because the reader whose hands change staff is one
/// player. And, alone among the four context kinds, **no barline check**: a
/// clef change mid-measure is ordinary notation and every backend writes one.
/// The engraver puts the small clef before the note it affects.
fn elaborate_clef(
    resolver: &mut Resolver,
    stmt: &musa_language::ast::ClefStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
    place: Place,
) -> Segment {
    let span = resolve::trimmed_span(stmt.syntax());
    if place == Place::Material {
        resolver.report(misplaced_context("clef", span));
        return Segment::empty();
    }
    let written = stmt.name().unwrap_or_default();
    let Some(clef) = crate::score::Clef::parse(&written) else {
        resolver.report(
            Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a clef"))
                .at(span, "not a clef musa reads")
                .help(resolve::suggest(&written, crate::score::Clef::NAMES, "clefs")),
        );
        return Segment::empty();
    };
    let Scope::Voice { part, .. } = scope else {
        // Unreachable while `Place::Voice` is only ever passed with a voice's
        // own scope; refusing rather than guessing a part keeps it that way.
        return Segment::empty();
    };
    Segment::literal(point(ScoreFact::new(
        Scope::Part { part },
        FactKind::Clef { clef },
        origin_of(cx, span),
    )))
}

/// The one refusal every context statement shares: material is written once
/// and can be played in several places, so a statement whose meaning is
/// "from here on" has no single "here" inside it.
fn misplaced_context(what: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        Code::Misplaced,
        format!("a {what} change belongs to the piece, not to material"),
    )
    .at(span, "written inside a body")
    .help("write it among the voice's own items, before the material that is in it")
    .note("material is written once and can be played in several places, so where a context change falls is a property of the place, not of the notes")
}

fn errors_so_far(resolver: &Resolver) -> usize {
    resolver
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error)
        .count()
}

fn reported_an_error(resolver: &Resolver) -> bool {
    resolver
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error)
}

/// A claim the compiler has taken on and not yet discharged.
///
/// Two constructs raise these and there is exactly one kind, on purpose. A
/// `bar { … }` claims "this is one measure"; an `assert`
/// claims whatever it says. Both are a proposition about a passage that can
/// only be settled once the whole piece has been read — the meter in force at
/// a bar is decided by every `meter` in every voice, and a `meter` in the
/// second voice governs the first voice's bars — so both are recorded here
/// during elaboration and proved afterwards, by [`crate::assert::check`].
///
/// Holding them in one list is what keeps the bar's diagnostic and the
/// assertion's from drifting into two answers to one question.
pub(crate) struct PendingClaim {
    /// What is claimed.
    claim: crate::assert::Claim,
    /// What it is claimed about.
    passage: crate::assert::Passage,
    /// Where it was written, so a polymetric piece checks it against its own
    /// part's barlines rather than the piece's.
    scope: Scope,
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
fn resolve_meters(resolver: &mut Resolver) -> crate::BarLines {
    let mut changes = std::mem::take(&mut resolver.meter_changes);
    changes.sort_by_key(|(at, _, span)| (*at, span.start));
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
fn check_keys(resolver: &mut Resolver, bars: &crate::BarLines) {
    let mut changes = std::mem::take(&mut resolver.key_changes);
    changes.sort_by_key(|(at, _, span)| (*at, span.start));
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

/// Instantiate the checked contextual value named by a `use` statement.
fn elaborate_use(
    resolver: &mut Resolver,
    share: &mut Share,
    call: &musa_language::ast::UseStmt,
    cx: &ExpandCx<'_>,
    scope: Scope,
) -> Segment {
    let call_span = resolve::trimmed_span(call.syntax());
    let Some(music) = cx.music().music_at(call_span) else {
        let name = call.motif().unwrap_or_default();
        let mut known: Vec<&str> = resolver.motifs.keys().map(String::as_str).collect();
        known.extend(cx.known_names());
        resolver.report(
            Diagnostic::error(Code::UnknownName, format!("cannot find `{name}`"))
                .at(call_span, "not declared in this piece")
                .help(crate::resolve::suggest_name(&name, &known)),
        );
        return Segment::empty();
    };
    let reference = instantiate_music(resolver, share, music, cx, scope, call_span);
    if call.overrides().is_empty() {
        return reference;
    }
    // Overrides rewrite named notes of *this* call, which needs the notes.
    Segment::literal(specialize(resolver, call, &share.evaluate(reference.term)))
}

/// Instantiate one contextual value, preserving legacy material roles while
/// giving ordinary `let`/`fn` values the same sharing and provenance path.
fn instantiate_music(
    resolver: &mut Resolver,
    share: &mut Share,
    music: &crate::core::Music,
    cx: &ExpandCx<'_>,
    scope: Scope,
    call_span: SourceSpan,
) -> Segment {
    let legacy = music.role.as_ref().and_then(|role| {
        resolver.motifs.get_full(&role.name).map(|(index, _, definition)| {
            (
                role.name.clone(),
                role.material,
                index,
                definition.declaration,
                definition.span,
                role.foreign,
            )
        })
    });
    if let Some((name, material, index, _, declared_at, _)) = &legacy {
        if *material == crate::resolve::Material::Bar && call_span.start < declared_at.end {
            let diagnostic = if call_span.start >= declared_at.start {
                Diagnostic::error(Code::Misplaced, format!("the bar `{name}` plays itself"))
                    .at(call_span, format!("this is inside `bar {name}`"))
                    .help("write the notes out, or play a bar declared above this one")
            } else {
                Diagnostic::error(Code::Misplaced, format!("the bar `{name}` is written after this"))
                    .at(call_span, "used before it exists")
                    .also(*declared_at, "declared here")
                    .help("move the `use` below the bar, or the bar above the `use`")
                    .note("material is read top to bottom, which is what makes a piece that quotes itself impossible")
            };
            resolver.report(diagnostic);
            return Segment::empty();
        }
        if *index >= cx.max_motif() {
            let word = material.word();
            resolver.report(
                Diagnostic::error(Code::Misplaced, format!("{word} `{name}` is declared after this one"))
                    .at(call_span, "used before it exists")
                    .help(format!(
                        "move the declaration above the motif that uses it, or write the {word}'s notes out here"
                    ))
                    .note("a motif sees only the material above it, which is what makes a cycle impossible"),
            );
            return Segment::empty();
        }
    }

    let (name, material, index, declaration, foreign) = legacy.as_ref().map_or_else(
        || {
            (
                format!("value@{}:{}", music.definition_span.start, music.definition_span.end),
                None,
                cx.max_motif(),
                cx.declaration(),
                cx.foreign(),
            )
        },
        |(name, material, index, declaration, _, foreign)| {
            (name.clone(), Some(*material), *index, *declaration, *foreign)
        },
    );
    let choice = material.map_or_else(
        || cx.choice().clone(),
        |material| {
            crate::ChoicePath::default().then(match material {
                crate::resolve::Material::Bar => crate::ChoiceStep::Bar(name.as_str().into()),
                crate::resolve::Material::Motif => crate::ChoiceStep::Motif(name.as_str().into()),
                crate::resolve::Material::Fragment => crate::ChoiceStep::Fragment(name.as_str().into()),
            })
        },
    );
    let entered = cx.material().entered(music, declaration, index, foreign);
    let inner = cx.entering(&entered, choice);
    let key = music_key(&name, &inner, music.definition_span, scale_in_force(resolver, &inner));
    let (binding, extent, occurrences) = match share.lookup(&key) {
        // A hit charges the output meter nothing, and neither did the miss.
        // That meter is reserved exactly once, at the compilation boundary,
        // against the occurrence count the lanes report; everything before it
        // only *previews* a forthcoming aggregate so an oversized repeat is
        // refused before it allocates. Charging a hit would therefore not
        // "charge the program anyway" — it would charge it twice, and refuse
        // pieces that the same music spelled as one `repeat` is accepted at.
        // What keeps the meter honest across sharing is `occurrences`, which
        // the reference carries out of the table and the boundary counts.
        Some(found) => found,
        None => {
            let elaborated = elaborate_music_value(resolver, share, music, &inner, SHARED_SCOPE);
            let extent = elaborated.extent;
            let occurrences = elaborated.occurrences;
            (share.bind(key, elaborated), extent, occurrences)
        }
    };
    let steps = cx
        .path()
        .iter()
        .cloned()
        .chain(std::iter::once(ExpansionStep::MotifApplication {
            call_site: call_span,
        }))
        .collect::<Vec<_>>();
    Segment {
        term: Term::var_marked(binding, mark_of(0, &steps, Some(call_span), Some(scope))),
        extent,
        occurrences,
        tied: false,
    }
}

/// Everything a body's facts depend on, and nothing about where it is called.
///
/// The call site used to be in this key, which meant two calls of one motif
/// shared a body only when they were written in the same place — so the same
/// music was elaborated once per `use` and the term carried a copy per call.
/// What the call site was standing in for is `pitch`: the scale a `step`
/// reads is the innermost `in scale`, or else the key latest at the cursor,
/// and neither is a property of the motif. Naming it directly is what lets
/// the site itself go, because the site is otherwise carried at the
/// *reference*, in its mark, and never baked into the body.
fn music_key(name: &str, inner: &ExpandCx<'_>, definition: SourceSpan, pitch: Option<crate::scale::Scale>) -> String {
    let mut key = format!(
        "{name}|{}:{}|pitch={}|scale={}|decl={:?}|limit={}|foreign={}|choice={:?}|",
        definition.start,
        definition.end,
        pitch.map_or_else(|| "-".to_owned(), |scale| scale.to_string()),
        inner.scale(),
        inner.declaration(),
        inner.max_motif(),
        inner.foreign(),
        inner.choice(),
    );
    for interval in inner.intervals() {
        let _ = write!(key, "{interval:?},");
    }
    key.push('|');
    for (param, value) in inner.bindings() {
        let _ = write!(key, "{param}={value:?};");
    }
    append_contextual_key(&mut key, inner.music());
    key
}

fn append_contextual_key(key: &mut String, music: &crate::core::Music) {
    let _ = write!(
        key,
        "@{}:{}#{}",
        music.definition_span.start,
        music.definition_span.end,
        music.items.len()
    );
    for (name, value) in &music.bindings {
        let _ = write!(key, "{name}={value:?};");
    }
    for (site, pitch) in music.pitches.iter() {
        let _ = write!(key, "pitch{site}={pitch};");
    }
    for (site, nested) in &music.uses {
        let _ = write!(key, "use{site}[");
        append_contextual_key(key, nested);
        key.push(']');
    }
    match music.operation.as_deref() {
        None => key.push_str("|surface"),
        Some(crate::core::MusicOperation::Transpose { interval, source }) => {
            let _ = write!(key, "|transpose:{interval:?}[");
            append_contextual_key(key, source);
            key.push(']');
        }
        Some(crate::core::MusicOperation::Stretch { factor, source }) => {
            let _ = write!(key, "|stretch:{factor}[");
            append_contextual_key(key, source);
            key.push(']');
        }
        Some(crate::core::MusicOperation::Retrograde { source }) => {
            key.push_str("|retrograde[");
            append_contextual_key(key, source);
            key.push(']');
        }
        Some(crate::core::MusicOperation::Invert { axis, source }) => {
            let _ = write!(key, "|invert:{axis}[");
            append_contextual_key(key, source);
            key.push(']');
        }
        Some(crate::core::MusicOperation::Shift { by, source }) => {
            let _ = write!(key, "|shift:{by}[");
            append_contextual_key(key, source);
            key.push(']');
        }
        Some(crate::core::MusicOperation::Together { left, right }) => {
            key.push_str("|together[");
            append_contextual_key(key, left);
            key.push('|');
            append_contextual_key(key, right);
            key.push(']');
        }
        Some(crate::core::MusicOperation::MapNotePitches { mapper, source }) => {
            let _ = write!(key, "|map:{}[", crate::core::pitch_function_key(mapper));
            append_contextual_key(key, source);
            key.push(']');
        }
        Some(crate::core::MusicOperation::Play { voicing, held }) => {
            let _ = write!(key, "|play:{voicing}:{held}");
        }
        Some(crate::core::MusicOperation::KernelQuote { term, holes }) => {
            // The term is already assembled and cannot read context, so the
            // definition span this key already carries identifies it: one
            // span in one document is one quote. Only the holes can mean
            // different things in different places, so only they are walked.
            let _ = write!(key, "|quote:{}[", term.occurrence_bound());
            for (name, locus, music) in holes {
                let _ = write!(key, "{name}@{locus}(");
                append_contextual_key(key, music);
                key.push(')');
            }
            key.push(']');
        }
    }
}

/// Interpret an opaque contextual value after a voice has supplied scope and
/// onset. Every constructor delegates to the same transform helpers as its
/// block spelling; the kernel still receives only a closed `Term<WrittenTime, ScoreFact>`.
fn elaborate_music_value(
    resolver: &mut Resolver,
    share: &mut Share,
    music: &crate::core::Music,
    cx: &ExpandCx<'_>,
    scope: Scope,
) -> Segment {
    let material = cx.material().nested(music);
    let local = cx.rebased(&material);
    match music.operation.as_deref() {
        None => elaborate_items(resolver, share, &music.items, &local, scope),
        Some(crate::core::MusicOperation::Transpose { interval, source }) => {
            elaborate_music_value(resolver, share, source, &local.transposed(*interval), scope)
        }
        Some(crate::core::MusicOperation::Stretch { factor, source }) => {
            let local = local.with_step(ExpansionStep::Stretch(*factor));
            let segment = elaborate_music_value(resolver, share, source, &local, scope);
            stretch_segment(share, segment, *factor)
        }
        Some(crate::core::MusicOperation::Retrograde { source }) => {
            let local = local.with_step(ExpansionStep::Retrograde);
            let segment = elaborate_music_value(resolver, share, source, &local, scope);
            retrograde_segment(share, segment)
        }
        Some(crate::core::MusicOperation::Invert { axis, source }) => {
            let local = local.with_step(ExpansionStep::Inversion { axis: axis.to_string() });
            let segment = elaborate_music_value(resolver, share, source, &local, scope);
            invert_segment(resolver, share, segment, *axis, &axis.to_string())
        }
        Some(crate::core::MusicOperation::Shift { by, source }) => {
            let segment = elaborate_music_value(resolver, share, source, &local, scope);
            Segment {
                term: Term::shift(beats(*by), segment.term),
                extent: beats(*by + segment.extent.as_ratio()),
                occurrences: segment.occurrences,
                tied: false,
            }
        }
        Some(crate::core::MusicOperation::Together { left, right }) => {
            let left = elaborate_music_value(resolver, share, left, &local, scope);
            let right = elaborate_music_value(resolver, share, right, &local, scope);
            Segment {
                term: Term::together(vec![left.term, right.term]).unwrap_or_else(|_| Term::literal(empty_segment())),
                extent: left.extent.max(right.extent),
                occurrences: left.occurrences.saturating_add(right.occurrences),
                tied: false,
            }
        }
        Some(crate::core::MusicOperation::MapNotePitches { mapper, source }) => {
            let local = local.with_step(ExpansionStep::MapNotePitches);
            let segment = elaborate_music_value(resolver, share, source, &local, scope);
            map_note_pitches_segment(resolver, share, segment, mapper)
        }
        Some(crate::core::MusicOperation::Play { voicing, held }) => {
            // `play` names no articulation and no tie: it is a chord sounding
            // for a length, and anything further is written where it applies.
            let duration = NotatedDuration::spelled(*held);
            sounded_voicing(
                resolver,
                voicing,
                &duration,
                &[],
                false,
                &local,
                scope,
                music.definition_span,
            )
        }
        Some(crate::core::MusicOperation::KernelQuote { term, holes }) => {
            kernel_quote_segment(resolver, share, term, holes, &local, scope, music.definition_span)
        }
    }
}

/// Assemble a checked kernel quote: bind each hole to the material the host
/// spliced there, and hand back the closed term.
///
/// The quote is *already* a composition, so nothing here builds one. What is
/// left is provenance and accounting: every fact that leaves gains a
/// [`ExpansionStep::KernelSplice`] naming the locus it was assembled at — `0`
/// for material written raw in the quote, and the hole's own locus for
/// material spliced through one — so the Origin view can say "this note is
/// here because a quote put it here" about facts the surface never wrote.
///
/// Holes are instantiated **once**, here, and the facts are frozen from then
/// on: a raw `shift`, `scale` or `restrict` around a reference moves the facts
/// it already made, it does not re-elaborate the hole under a new context.
/// That is what makes an outer raw transform able to invalidate a
/// placement-sensitive assertion that passed inside the hole
/// (`docs/rules/language/05-verification.md`).
fn kernel_quote_segment(
    resolver: &mut Resolver,
    share: &mut Share,
    term: &Term<WrittenTime, ScoreFact>,
    holes: &[(String, Ratio<i64>, crate::core::Music)],
    cx: &ExpandCx<'_>,
    scope: Scope,
    span: SourceSpan,
) -> Segment {
    let mut assembled = term.clone();
    let mut occurrences = term.occurrence_bound();
    // The raw payloads are the quote's own writing, at the quote's own zero.
    // Scope and origin are supplied here rather than read: the checker made
    // the quote leave both blank, because material does not choose the voice
    // it is used in.
    let mut path = cx.path().to_vec();
    path.push(ExpansionStep::KernelSplice { at: Ratio::new(0, 1) });
    assembled.map_payloads(&mut |fact| {
        fact.scope = scope;
        fact.origin.source_span = cx.source_span(span);
        fact.origin.definition_span = span;
        fact.origin.declaration = cx.declaration();
        fact.origin.expansion_path.clone_from(&path);
    });
    // Reverse, so the first hole's binding ends up outermost and a later
    // hole's material may not shadow an earlier one's name — the fresh names
    // make that impossible anyway, and the order keeps the printed term
    // reading in the order the quote's holes do.
    for (name, locus, music) in holes.iter().rev() {
        let inner = cx.with_step(ExpansionStep::KernelSplice { at: *locus });
        let segment = elaborate_music_value(resolver, share, music, &inner, scope);
        occurrences = occurrences.saturating_add(segment.occurrences);
        assembled = Term::bind(name.clone(), segment.term, assembled);
    }
    let extent = match assembled.duration() {
        Ok(extent) => extent,
        Err(error) => {
            resolver.report(
                Diagnostic::error(Code::UnknownName, "this kernel quote has no extent").at(span, error.to_string()),
            );
            return Segment::literal(empty_segment());
        }
    };
    Segment {
        term: assembled,
        extent,
        occurrences,
        tied: false,
    }
}

/// One fragment of a mobile, by name.
///
/// A mobile arranges *fragments* and nothing else: a motif takes arguments and
/// a bar is a measure, and neither is material a performance is invited to
/// reorder. The refusal names what was found so the mistake is one sentence.
fn elaborate_fragment(
    resolver: &mut Resolver,
    share: &mut Share,
    name: &str,
    cx: &ExpandCx<'_>,
    scope: Scope,
    span: SourceSpan,
    use_span: SourceSpan,
) -> Segment {
    if let Some(definition) = resolver.motifs.get(name)
        && definition.material != crate::resolve::Material::Fragment
    {
        resolver.report(
            Diagnostic::error(
                Code::Misplaced,
                format!("`{name}` is a {}, not a fragment", definition.material.word()),
            )
            .at(span, "a mobile arranges fragments")
            .help(format!("declare it as `fragment {name} {{ … }}`")),
        );
        return Segment::empty();
    }
    let Some(music) = cx.fragment(name) else {
        let known: Vec<&str> = resolver.motifs.keys().map(String::as_str).collect();
        resolver.report(
            Diagnostic::error(Code::UnknownName, format!("cannot find `{name}`"))
                .at(span, "not declared in this piece")
                .help(crate::resolve::suggest_name(name, &known)),
        );
        return Segment::empty();
    };
    let material = music.role.as_ref().map(|role| role.material);
    if material != Some(crate::resolve::Material::Fragment) {
        resolver.report(
            Diagnostic::error(
                Code::Misplaced,
                format!(
                    "`{name}` is {}, not a fragment",
                    material.map_or("a music value", crate::resolve::Material::word)
                ),
            )
            .at(span, "a mobile arranges fragments")
            .help(format!("declare it as `fragment {name} {{ … }}`")),
        );
        return Segment::empty();
    }
    // A name in a mobile's list is a use of the fragment it resolves to.
    if !cx.foreign() {
        resolver
            .references
            .record_use(crate::resolve::NameKind::Fragment, name, use_span);
    }
    instantiate_music(resolver, share, music, cx, scope, span)
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
fn specialize(resolver: &mut Resolver, call: &musa_language::ast::UseStmt, elaborated: &VoiceTrack) -> VoiceTrack {
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

    let occurrences: Vec<Occurrence<WrittenTime, ScoreFact>> = elaborated
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
    track_or_empty(elaborated.duration(), occurrences)
}

/// The derived time reversal (roadmap §5.4): `(d, E)` becomes
/// `(d, {(d−e, d−s, a)})`.
///
/// A plain function over an elaborated timeline, deliberately: the kernel
/// needs no reversal primitive to express it, which is the evidence
/// `docs/rules/kernel/08-open-questions.md` records for §34's smallest complete
/// basis.
///
/// Every mark stays with the note that carries it — a staccato is written on
/// a note, and reversing time does not move it. Ties need no repair: they
/// were merged when the enclosed items were elaborated, so what reverses is
/// Apply the kernel scale action and the matching written-duration action.
/// Both block and function spellings call this one implementation.
fn stretch_segment(share: &Share, segment: Segment, factor: Ratio<i64>) -> Segment {
    let elaborated = share.evaluate(segment.term);
    Segment::literal(
        elaborated
            .map_payloads(|payload| payload.stretched(factor))
            .scale(factor)
            .unwrap_or_else(|_| empty_segment()),
    )
}

fn retrograde_segment(share: &Share, segment: Segment) -> Segment {
    Segment::literal(reverse(&share.evaluate(segment.term)))
}

fn invert_segment(
    resolver: &mut Resolver,
    share: &Share,
    segment: Segment,
    axis: WrittenPitch,
    axis_text: &str,
) -> Segment {
    let elaborated = share.evaluate(segment.term);
    let refused = std::cell::RefCell::new(Vec::new());
    let inverted = elaborated.map_payloads(|payload| {
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
                format!("`{pitch}` is outside Musa's stored coordinate range when mirrored around `{axis_text}`"),
            )
            .at(at, "the exact integer result exceeds the implementation range")
            .help("mirror around a nearer pitch, or reduce the register displacement"),
        );
    }
    Segment::literal(inverted)
}

fn map_note_pitches_segment(
    resolver: &mut Resolver,
    share: &Share,
    segment: Segment,
    mapper: &crate::core::PitchFunction,
) -> Segment {
    let elaborated = share.evaluate(segment.term);
    let failed = std::cell::Cell::new(false);
    let mapped = elaborated.map_payloads(|payload| {
        map_note_pitch_fact(payload, |pitch| crate::core::apply_pitch_function(mapper, pitch)).unwrap_or_else(|| {
            failed.set(true);
            payload.clone()
        })
    });
    if failed.get() {
        resolver.report(
            Diagnostic::error(Code::ResourceLimit, "a pitch mapping exceeded the elaboration budget")
                .at(SourceSpan::default(), "while mapping sounding pitches"),
        );
    }
    Segment::literal(mapped)
}

/// The one exhaustive coverage table for the controlled traversal. Keeping
/// it at the fact boundary makes a newly-added `FactKind` a compile error
/// until its sounding-pitch policy is chosen deliberately.
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

/// an ordinary occurrence with an ordinary span.
fn reverse(timeline: &VoiceTrack) -> VoiceTrack {
    let extent = timeline.duration();
    let mut mirrored: Vec<Occurrence<WrittenTime, ScoreFact>> = timeline
        .occurrences()
        .iter()
        .map(|occurrence| {
            let span = occurrence.span();
            let start = Position::new(extent.as_ratio() - span.end().as_ratio());
            let end = Position::new(extent.as_ratio() - span.start().as_ratio());
            let mirrored = Span::new(start, end).unwrap_or(Span::ZERO);
            Occurrence::new(mirrored, occurrence.payload().clone())
        })
        .collect();
    // Stable by start, so the members of a chord stay adjacent and in the
    // order the projection expects.
    mirrored.sort_by_key(|occurrence| (occurrence.span().start(), occurrence.span().end()));
    track_or_empty(extent, mirrored)
}

/// A timeline over `extent`, or the empty segment when the occurrences do
/// not fit it (unreachable for elaborated music; never a panic).
/// A written-time amount from an exact rational, clamped at zero.
///
/// Elaboration derives lengths from notated durations, repeat counts, and
/// tuplet ratios, all nonnegative by construction. A negative one would be a
/// bug upstream rather than a piece of music, and answering it with zero keeps
/// one bad measure from taking the rest of the piece with it — the same
/// judgment [`track_or_empty`] already makes about an out-of-bounds fact.
fn beats(amount: num_rational::Ratio<i64>) -> Duration<WrittenTime> {
    Duration::new(amount).unwrap_or(Duration::ZERO)
}

fn track_or_empty(extent: Duration<WrittenTime>, occurrences: Vec<Occurrence<WrittenTime, ScoreFact>>) -> VoiceTrack {
    track(extent, occurrences).unwrap_or_else(|_| empty_segment())
}

/// The empty segment `(0, ∅)` — contributes nothing to the sequence.
fn empty_segment() -> VoiceTrack {
    empty(Duration::ZERO)
}

/// The span `[0, d)` for a notated duration. Non-negative by construction
/// (durations parse from positive literals); `Span::ZERO` is the dead
/// fallback.
fn span_of_duration(duration: &NotatedDuration) -> Span<WrittenTime> {
    Span::new(Position::ZERO, Position::new(duration.value.as_ratio())).unwrap_or(Span::ZERO)
}

/// A segment holding one occurrence over `[0, d)`. Bounds hold by
/// construction; the empty segment is the dead fallback.
fn single(duration: &NotatedDuration, payload: ScoreFact) -> VoiceTrack {
    let span = span_of_duration(duration);
    track_or_empty(span.duration(), vec![Occurrence::new(span, payload)])
}

/// A point fact at the cursor: an occurrence of zero extent in a segment of
/// zero extent, so sequencing places it exactly where it was written.
fn point(fact: ScoreFact) -> VoiceTrack {
    track_or_empty(Duration::ZERO, vec![Occurrence::new(Span::ZERO, fact)])
}

/// The origin for an event under this expansion context (mirrors the direct
/// lowerer: motif applications point at the call site).
fn origin_of(cx: &ExpandCx<'_>, span: SourceSpan) -> Origin {
    Origin {
        source_span: cx.source_span(span),
        definition_span: span,
        declaration: cx.declaration(),
        expansion_path: cx.path().to_vec(),
    }
}

/// Apply the transposition stack (shared semantics with the direct lowerer).
///
/// The blame span is passed rather than a node, because a pitch that a
/// function built has no node of its own to point at — only the site that
/// asked for it.
fn apply_intervals(
    resolver: &mut Resolver,
    pitch: WrittenPitch,
    cx: &ExpandCx<'_>,
    blame: SourceSpan,
) -> Option<WrittenPitch> {
    let mut current = pitch;
    for interval in cx.intervals() {
        let Some(next) = current.transpose(*interval) else {
            resolver.report(
                Diagnostic::error(
                    Code::OutOfRange,
                    format!("`{current}` is outside Musa's stored coordinate range after this transposition"),
                )
                .at(blame, "the exact integer result exceeds the implementation range")
                .help("use a smaller interval or reduce the register displacement"),
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
fn merge_ties(resolver: &mut Resolver, timeline: VoiceTrack) -> VoiceTrack {
    if !timeline.occurrences().iter().any(|it| it.payload().tied) {
        return timeline;
    }
    let extent = timeline.duration();
    let statements = statements(timeline.occurrences());
    let mut merged: Vec<Vec<Occurrence<WrittenTime, ScoreFact>>> = Vec::with_capacity(statements.len());
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
    let occurrences: Vec<Occurrence<WrittenTime, ScoreFact>> = merged.into_iter().flatten().collect();
    track_or_empty(extent, occurrences)
}

/// The occurrences grouped into *statements*: one written note or rest, or
/// the pitches of one chord, which share a span and an origin.
///
/// Region and point facts are statements of one, and never merge: only a
/// notehead can be tied.
fn statements(occurrences: &[Occurrence<WrittenTime, ScoreFact>]) -> Vec<Vec<Occurrence<WrittenTime, ScoreFact>>> {
    let mut grouped: Vec<Vec<Occurrence<WrittenTime, ScoreFact>>> = Vec::with_capacity(occurrences.len());
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
fn same_sound(left: &[Occurrence<WrittenTime, ScoreFact>], right: &[Occurrence<WrittenTime, ScoreFact>]) -> bool {
    let pitches = |statement: &[Occurrence<WrittenTime, ScoreFact>]| -> Option<Vec<WrittenPitch>> {
        statement.iter().map(|it| it.payload().pitch_of()).collect()
    };
    match (pitches(left), pitches(right)) {
        (Some(left), Some(right)) => !left.is_empty() && left == right,
        _ => false,
    }
}

/// Extend `previous` through `statement`: one occurrence per pitch, spanning
/// both, spelled as the noteheads the composer wrote.
fn join(previous: &mut [Occurrence<WrittenTime, ScoreFact>], statement: &[Occurrence<WrittenTime, ScoreFact>]) {
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
fn check_dangling_tie(resolver: &mut Resolver, timeline: VoiceTrack) -> VoiceTrack {
    if !timeline.occurrences().iter().any(|it| it.payload().tied) {
        return timeline;
    }
    let extent = timeline.duration();
    let mut occurrences: Vec<Occurrence<WrittenTime, ScoreFact>> = timeline.occurrences().to_vec();
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
    track_or_empty(extent, occurrences)
}

/// Clear a tie that could not be honoured, so nothing downstream sees it.
fn untie(occurrence: &mut Occurrence<WrittenTime, ScoreFact>) {
    let mut fact = occurrence.payload().clone();
    fact.tied = false;
    *occurrence = Occurrence::new(occurrence.span(), fact);
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

/// The normalized human-display text of a source's piece timeline, for golden
/// snapshots (docs/rules/kernel/05 N5). Semantic hashing uses separate framed N6
/// bytes. `None` when the
/// source does not elaborate cleanly.
///
/// One timeline, not one per part: a compilation has exactly
/// one temporal object, and the normal form is the text of that object —
/// key, meter, form markers and chord symbols included.
#[doc(hidden)]
pub fn kernel_normal_form(source: &SourceDocument, realization: &crate::Realization) -> Option<String> {
    let (_, term, _) = piece_term(source, realization)?;
    Some(musa_kernel::evaluate_marked(term, instantiate).to_string())
}

/// The piece as a **term** (docs/rules/kernel/10): its name, and an `over` of one
/// literal per voice plus one for the piece-wide context.
///
/// The shared bodies are `let`-bound around the whole `over`, because a motif
/// called from two voices is one body and its binding has to dominate both.
/// Bindings nothing references — a level that had to be evaluated spent its
/// sharing — are dropped, so the printed term names only what it uses.
pub(crate) fn piece_term(
    source: &SourceDocument,
    realization: &crate::Realization,
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
            "piece".to_owned(),
            crate::template::Kind::Piece,
            None,
            source.name(),
        )
    });
    let piece = PieceDecl::from_root(&root).or_else(|| made.as_ref().and_then(crate::template::Instance::piece))?;
    let mut snapshot = ScoreSnapshot::default();
    let enclosing = made.as_ref().map(crate::template::Instance::template);
    let sites = collect_voice_sites(&mut resolver, &mut templates, &piece, enclosing, source.name());
    let mut bindings: Vec<crate::core::Binding> = Vec::new();
    if let Some(instance) = &made {
        let arguments = crate::core::check_for_kernel(&mut resolver, &root, None, instance.holders())?;
        bindings.extend(instance.bindings(&arguments));
    }
    bindings.extend(sites.values().flat_map(crate::template::Instance::holders));
    // This interchange helper historically has no import-source parameter.
    // Preserve that contract: local material elaborates, and a use whose
    // declaration only an unavailable import could supply is the empty error
    // segment the pre-core helper produced. Full compilation always uses the
    // checked build closure above.
    let core = crate::core::check_for_kernel(&mut resolver, &root, Some(piece.syntax()), bindings)?;
    let expansion = Expansion {
        root: root.clone(),
        libraries: None,
        sites: &sites,
        prefix: made.as_ref().map(crate::template::Instance::step).into_iter().collect(),
    };
    resolve::lower_header(&mut resolver, &piece, &mut snapshot, &core);
    let score = piece.score()?;
    let mut share = Share::default();
    resolve::register_bars(&mut resolver, &mut snapshot, &score);
    let mut lanes = Vec::new();
    let mut declared = Vec::new();
    for (part_index, part) in score.parts().iter().enumerate() {
        let part_id = u32::try_from(part_index).unwrap_or(u32::MAX);
        declared.push((part_id, resolve::part_context(&mut resolver, part, snapshot.profiles())));
        for (index, item) in part.items().into_iter().enumerate() {
            let Some(site) = VoiceSite::read(&mut resolver, &expansion, &core, &item) else {
                continue;
            };
            let voice_key = resolver.declare(crate::resolve::DeclInfo::Voice);
            let declaration = resolve::ordinal(&resolver, voice_key);
            lanes.push(elaborate_voice(
                &mut resolver,
                &mut share,
                &site.voice,
                declaration,
                part_id,
                u32::try_from(index).unwrap_or(u32::MAX),
                site.program(&core),
                site.path.clone(),
            ));
        }
    }
    // The stack's extent without building the stack: D3 says it is the
    // maximum of the parts', and the context facts need it before they exist.
    let extent = lanes.iter().map(|lane| lane.extent).max().unwrap_or(Duration::ZERO);
    let bars = resolve_meters(&mut resolver);
    check_keys(&mut resolver, &bars);
    let context = context_facts(&mut resolver, &piece, &score, &declared, &bars, extent);
    let lane_occurrences = lanes
        .iter()
        .fold(0u64, |count, lane| count.saturating_add(lane.occurrences));
    let context_occurrences = u64::try_from(context.occurrences().len()).unwrap_or(u64::MAX);
    if !share.reserve_output(
        "elaborating the piece timeline",
        lane_occurrences.saturating_add(context_occurrences),
        resolve::trimmed_span(score.syntax()),
    ) {
        return None;
    }
    let parts: Vec<musa_kernel::Term<WrittenTime, ScoreFact>> = lanes
        .into_iter()
        .map(|lane| lane.term)
        .chain(std::iter::once(musa_kernel::Term::literal(context)))
        .collect();
    let term = share.close(musa_kernel::Term::together(parts).ok()?);
    term.check().ok()?;
    Some((piece.name().unwrap_or_default(), term, resolver.decisions))
}
