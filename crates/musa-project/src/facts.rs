//! Everything the interface displays about a score, stated once, in words.
//!
//! `docs/rules/desktop/03-interaction.md` §7 is an exhaustive list of what the
//! frontend may compute, and it contains no musical facts at all: not bar
//! numbers, not beats, not pitch names, not provenance. So they are computed
//! here, once per successful compile, from the compiled score — and the
//! frontend renders strings and numbers it was handed.
//!
//! Everything in this module is display-ready. `Fraction` carries the exact
//! rational so the interface can typeset a real fraction
//! (`01-visual-language.md` §3) rather than a decimal.

use musa_compiler::{
    ExpansionStep, IntegratedTempoMap, Interval, Mode, MusicalTime, Origin, PerformanceOptions, PitchClass, Scope,
    ScoreEventKind, ScoreSnapshot, WrittenPitch,
};
use musa_language::HeaderField;
use serde::Serialize;
use serde::ser::SerializeStruct;

/// An exact rational, as a fraction rather than a decimal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Fraction {
    /// Numerator.
    pub numerator: i64,
    /// Denominator; never zero.
    pub denominator: i64,
}

impl Fraction {
    pub(crate) fn from_ratio(ratio: num_rational::Ratio<i64>) -> Self {
        Self {
            numerator: *ratio.numer(),
            denominator: *ratio.denom(),
        }
    }
}

/// What kind of thing an event is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    /// A single pitch.
    Note,
    /// Silence.
    Rest,
    /// Simultaneous pitches.
    Chord,
}

/// Where an event came from (`docs/rules/desktop/04-provenance.md` §3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginFacts {
    /// False when the user typed this note; true when the compiler produced
    /// it by expanding something.
    pub generated: bool,
    /// The expansion path, outside in, one step per act of expansion. Empty
    /// for authored events.
    pub path: Vec<StepFact>,
    /// Which note of its occurrence this is, 1-based — the `▸ note 3` tail
    /// of the inspector's Origin row. Absent for authored events.
    pub note_index: Option<u32>,
    /// The occurrence that produced it, when generated — the key into
    /// `ScoreFacts::occurrences`.
    pub occurrence: Option<String>,
    /// 1-based line in the source that (transitively) produced the event.
    pub line: u32,
    /// The source byte range, for revealing it in the drawer.
    pub span: crate::diagnostic::Span,
    /// The statement that spells this event: the note inside the `motif`
    /// body when generated, the same as `span` when authored. This is what
    /// an edit-definition edit rewrites (`04-provenance.md` §4).
    pub definition_span: crate::diagnostic::Span,
    /// Every place in the source this event's derivation reaches, in the
    /// order it reaches them (`02-derivation-diagrams.md` §6).
    ///
    /// The expansion path above says what happened; this says where. A note
    /// instantiated from a shared body reaches two places at each use — the
    /// body it instantiates and the site that instantiated it — and two uses
    /// of one body reach the same body and different sites, which is exactly
    /// what a list-shaped path could not tell apart.
    pub supported_by: Vec<crate::diagnostic::Span>,
    /// The decision this event was played under, as an index into
    /// [`ScoreFacts::decisions`] — the fourth step of the Origin chain.
    ///
    /// Absent for the overwhelming majority of notes, because the
    /// overwhelming majority of pieces decide nothing. Resolved here rather
    /// than in a frontend because finding it means asking which decision site
    /// *contains* this note, which is the core reasoning about the source
    /// (`03-interaction.md` §7).
    pub decision: Option<usize>,
}

/// One step of an expansion path, as the Origin row reads it.
///
/// A step used to be a bare string, which was enough while every step was a
/// motif application or a transform block and the row's only control was
/// "select what this produced". The elaboration language made steps into
/// *places*: a template instance is written at a `make`, an assertion at its
/// `assert`, a kernel quotation at the splice that put the material there,
/// and a composer following provenance wants to open each one. A segment
/// therefore carries where it is written, and what kind of thing it is, so
/// the interface can word and style it without reading the label back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepFact {
    /// What to print: `sigh()`, `transpose down P5`, `make Upper`.
    pub label: String,
    /// What kind of step it is.
    pub kind: StepKind,
    /// Where it is written, for revealing it in the source column.
    ///
    /// Absent for a step that has no written site of its own: a `repeat`
    /// iteration is a *count*, and the block it counts is already the step
    /// beside it. A frontend that invented a span for one would be pointing
    /// at something that is not there.
    pub span: Option<crate::diagnostic::Span>,
}

/// What one expansion step is.
///
/// Named rather than inferred from the label, because the label is prose that
/// the core spells and the interface must not parse (`03-interaction.md` §7).
/// Exhaustive, and matched exhaustively: a new step in the language arrives
/// as a compile error in every consumer rather than as a silent "other".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepKind {
    /// A motif or score function was applied here. The unit a composer
    /// selects, and the innermost step of most paths.
    Occurrence,
    /// A declaration template was expanded at a `make` site. Named by the
    /// instance rather than the template: two instances are two places.
    Instance,
    /// A transform block was in force: `transpose`, `stretch`, `retrograde`,
    /// `invert`, a note-pitch map, a lexical scale, or a repeat iteration.
    Transform,
    /// An `assert` was in force and its claim held. Provenance only — an
    /// assertion produces no music, which is exactly why it is worth saying
    /// that it covered this passage.
    Assertion,
    /// The material entered through a kernel quotation, at this locus in the
    /// quoted term's own time.
    Splice,
    /// A `with` clause respelled this note of its occurrence.
    Specialization,
}

/// One expansion, and everything it produced
/// (`docs/rules/desktop/04-provenance.md` §2).
///
/// An occurrence is a single act of expansion: one `use` inside whatever
/// transform blocks enclose it. Two `use sigh()` statements are two
/// occurrences even though they read the same, and the same `use` inside a
/// `transpose` block is a third — which is exactly the distinction a composer
/// is asking about when they hold the lens.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OccurrenceFacts {
    /// Stable within one compile; the id events point at.
    pub id: String,
    /// The expansion path, outside in: `transpose down P5 ▸ sigh()`.
    pub path: Vec<StepFact>,
    /// The path as one line, for a margin bracket's label.
    pub label: String,
    /// The motif's name, when a motif produced this: `sigh`.
    pub motif: Option<String>,
    /// Where that motif is declared, for revealing it in the drawer.
    pub declaration: Option<crate::diagnostic::Span>,
    /// The `use` statement that ran.
    pub use_site: crate::diagnostic::Span,
    /// 1-based line of the `use` statement.
    pub line: u32,
    /// Every event this expansion produced, in score order.
    pub events: Vec<String>,
}

/// One event, as the inspector reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventFacts {
    /// The MEI `xml:id` — the same identity the engraved SVG carries
    /// so a click on the page finds this row.
    pub id: String,
    /// The part's name.
    pub part: String,
    /// The voice's name.
    pub voice: String,
    /// Note, rest, or chord.
    pub kind: EventKind,
    /// The written pitch, spelled: `A4`, `G♯4`. Absent for a rest; the first
    /// tone for a chord.
    pub pitch: Option<String>,
    /// Every pitch, for a chord.
    pub pitches: Vec<String>,
    /// The same pitches as the *language* spells them (`g#4`), not as a
    /// reader reads them (`G♯4`). An editable field has to round-trip
    /// through the source, and converting `♯` back to `s` in the frontend
    /// would be the frontend spelling music (`03-interaction.md` §7).
    pub pitch_spellings: Vec<String>,
    /// The notated duration's exact value in whole notes.
    pub duration: Fraction,
    /// How the duration is written in the source (`1/2`, `3/8`).
    pub duration_spelling: String,
    /// 1-based bar number.
    pub bar: u32,
    /// 1-based beat within the bar, exact.
    pub beat: Fraction,
    /// The key in force **here**, written out (`A minor`), or absent when
    /// nothing has said one.
    ///
    /// Per event rather than per piece because a piece modulates: the
    /// inspector's job is to say what is true where the composer is looking,
    /// and asking the frontend to find the latest change at or before this
    /// note would be the frontend reasoning about musical time
    /// (`03-interaction.md` §7).
    pub key: Option<String>,
    /// The clef this part is read in here, likewise.
    pub clef: Option<String>,
    /// The frame this event starts at, in the performance's sample rate —
    /// the same clock the engine reports positions in.
    ///
    /// Written time and sounding time are different layers (roadmap §2), and
    /// only the performance lowering knows the second. It is stated here so
    /// the interface can tint the sounding note and seek to a selection
    /// without computing anything temporal (`03-interaction.md` §7).
    pub onset_frames: u64,
    /// The frame it stops, at full notated gate.
    pub end_frames: u64,
    /// Provenance.
    pub origin: OriginFacts,
}

/// A voice in the parts list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceFacts {
    /// The voice's name.
    pub name: String,
    /// Whether any of its music was generated — the parts list dims voices
    /// with none while Origin view is held (`04-provenance.md` §2).
    pub generated: bool,
}

/// A part in the parts list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartFacts {
    /// The part's name.
    pub name: String,
    /// Its voices, in score order.
    pub voices: Vec<VoiceFacts>,
}

/// The score, as the interface's chrome reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreFacts {
    /// The piece's title.
    pub title: String,
    /// Beats per minute at the start, at the beat unit below.
    ///
    /// This is the transport's tempo, not the page's: a piece that states no
    /// metronome mark still plays at a speed, and this is the speed it plays
    /// at. Where the *markings* fall is the notation plan's business.
    pub tempo_bpm: u32,
    /// The starting tempo's beat unit as a fraction of a whole note (`1/4`).
    pub tempo_beat: Fraction,
    /// The key, written out: `A minor`. Absent when the piece declares none.
    pub key: Option<String>,
    /// Beats per bar.
    pub meter_count: u32,
    /// The beat unit's denominator.
    pub meter_unit: u32,
    /// Parts in score order.
    pub parts: Vec<PartFacts>,
    /// Every event, in score order, keyed by its engraved `xml:id`.
    pub events: Vec<EventFacts>,
    /// Every expansion in the piece, in the order they were first met.
    pub occurrences: Vec<OccurrenceFacts>,
    /// The piece's structure, in the order it is played: what the outline
    /// pane navigates by.
    pub outline: Vec<OutlineFacts>,
    /// Every statement the piece can make about itself, whether or not it
    /// makes it.
    pub header: Vec<HeaderFact>,
    /// Which performance this reading is, when the piece left anything to
    /// one.
    ///
    /// `None` is the determinate case, and it is what makes the interface's
    /// seed field appear and vanish rather than sit there in every piece
    /// that cannot use it.
    pub performance: Option<u64>,
    /// Every question the piece asked and the answer this performance gave,
    /// in the order the sites were reached. Empty for a determinate piece.
    pub decisions: Vec<DecisionFact>,
}

/// One decision, as the Origin view and the Settings panel read it.
///
/// The wording rules of `docs/rules/desktop/` are already applied: this carries
/// sentences a musician reads, not the machine's names for things. `path` is
/// the exception and is never shown — it is the key a pin is written against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionFact {
    /// The site's stable name, canonical. Opaque to the interface: it is what
    /// [`crate::ProjectCommand::Pin`] takes back, and nothing else.
    pub path: String,
    /// Where the question was asked, as a person reads it: `fill, first
    /// choice`.
    pub asked: String,
    /// The answer, likewise: `6 passes`, `held 3/8`, the fragments by name.
    pub answered: String,
    /// Whether the composer kept this one. An unpinned decision reads as the
    /// performance it came from; a pinned one reads "kept".
    pub pinned: bool,
    /// Where in the source the question is written — the first place, for a
    /// freedom the piece spells once per voice.
    pub span: crate::diagnostic::Span,
}

/// One of the piece's own facts, as the source spells it.
///
/// Every field is listed, including the ones the piece is silent about, and a
/// silent one carries `value: None`. That is deliberate: the interface shows
/// the empty rows too, and that list is where a composer discovers a piece can
/// name an arranger at all. A row that only appeared once it had something in
/// it could only be found by someone who already knew.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderFact {
    /// Which statement this is. On the wire it is the statement's own keyword
    /// — `"composer"`, `"tempo"` — which is both what the language calls it
    /// and what the edit command's field names are.
    pub field: HeaderField,
    /// What the source says, in the source's own spelling — `quarter = 72`,
    /// not `♩ = 72`. This is exactly what [`EditCommand::SetHeader`] takes
    /// back, so reading a field and writing it unchanged is the identity.
    ///
    /// [`EditCommand::SetHeader`]: crate::EditCommand::SetHeader
    pub value: Option<String>,
}

/// Written by hand rather than derived, because the wire spelling of `field`
/// is the language's keyword and `HeaderField` belongs to another crate — so
/// the mapping cannot be an attribute on the enum, and putting it here keeps
/// the whole wire shape of a row in one place.
impl Serialize for HeaderFact {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut row = serializer.serialize_struct("HeaderFact", 2)?;
        row.serialize_field("field", self.field.word())?;
        row.serialize_field("value", &self.value)?;
        row.end()
    }
}

/// What kind of structural marker an outline entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutlineKind {
    /// A form marker: `section "Exposition" at 1:1;`.
    Section,
    /// A named phrase over a run of notes.
    Phrase,
}

/// One row of the structural outline (roadmap §8.2's annotations, read as
/// navigation).
///
/// A section and a phrase are anchored differently in the score — one to a
/// time, one to a run of events — but a composer looking for "the
/// development" wants the same thing from both: a place to jump to. The row
/// carries the event to reveal, so the interface scrolls to a notehead rather
/// than guessing at a coordinate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineFacts {
    /// Section or phrase.
    pub kind: OutlineKind,
    /// The name as it was written.
    pub name: String,
    /// The bar it begins in.
    pub bar: u32,
    /// The beat within that bar, 1-based.
    pub beat: Fraction,
    /// The event to reveal, when the score has one there. A section written
    /// past the last note of every part has none.
    pub event: Option<String>,
    /// When it is reached, in the frames the engine reports positions in.
    pub onset_frames: u64,
    /// When the passage it names ends: the next marker of its kind for a
    /// section, the phrase's last note for a phrase. What makes "the
    /// selection is inside this" a question with an answer.
    pub end_frames: u64,
    /// 1-based line of the statement that wrote it.
    pub line: u32,
    /// Its source range, for revealing it in the drawer.
    pub span: crate::diagnostic::Span,
}

impl ScoreFacts {
    /// Derive the facts from a compiled score and the source it came from.
    ///
    /// `source` is needed only to turn byte offsets into line numbers — the
    /// interface shows a composer a line, not an offset.
    pub(crate) fn derive(
        score: &ScoreSnapshot,
        source: &str,
        taken: &[musa_compiler::DecisionRecord],
        derivation: Option<&musa_compiler::Derivation>,
    ) -> Self {
        let lines = LineIndex::new(source);
        let decisions: Vec<DecisionFact> = taken
            .iter()
            .map(|record| DecisionFact {
                path: record.path().canonical(),
                asked: record.path().describe(),
                answered: record.answered().to_owned(),
                pinned: record.pinned(),
                span: record
                    .sites()
                    .first()
                    .map_or(crate::diagnostic::Span { start: 0, end: 0 }, |site| {
                        crate::diagnostic::Span {
                            start: site.start,
                            end: site.end,
                        }
                    }),
            })
            .collect();
        // The same tempo integration the performance lowering uses, at the
        // same options, so a frame here is the frame the engine will report.
        let tempo = IntegratedTempoMap::new(score, Scope::Piece, &PerformanceOptions::default());
        // Unfolded: the fact index reports where a moment *sounds*, which is
        // the coordinate the snapshot's own barlines are built over. The
        // piece's, for the piece-wide markers below; an event's bar number is
        // its own part's, which under polymeter is a different number.
        let bars = score.bars(Scope::Piece);

        let mut parts = Vec::new();
        let mut events = Vec::new();
        // Occurrences, and the expansion paths that identify them. Identity is
        // the path itself — call-site spans and transform arguments included —
        // so two `use sigh()` statements are two occurrences even though they
        // read the same on the page.
        let mut occurrences: Vec<OccurrenceFacts> = Vec::new();
        let mut paths: Vec<Vec<ExpansionStep>> = Vec::new();
        for (id, part) in score.parts().iter() {
            let bars = score.bars(Scope::Part { part: id.0 });
            let mut voices = Vec::new();
            for (voice_id, voice) in part.voices() {
                let name = part
                    .voice_name(voice_id)
                    .map_or_else(|| voice_id.0.to_string(), ToString::to_string);
                let mut generated = false;
                for event in voice.events() {
                    let id = format!("event-{:x}", event.id.0);
                    let mut origin = origin_facts(&event.origin, &lines, source);
                    origin.supported_by = derivation.map_or_else(Vec::new, |graph| {
                        graph
                            .sources_of(event.id)
                            .into_iter()
                            .map(|span| crate::diagnostic::Span {
                                start: span.start,
                                end: span.end,
                            })
                            .collect()
                    });
                    origin.decision = decided_under(taken, event.origin.source_span);
                    if origin.generated {
                        let at = match paths.iter().position(|known| *known == event.origin.expansion_path) {
                            Some(at) => at,
                            None => {
                                paths.push(event.origin.expansion_path.clone());
                                occurrences.push(occurrence_facts(
                                    occurrences.len(),
                                    &origin.path,
                                    &event.origin.expansion_path,
                                    score,
                                    &lines,
                                    source,
                                ));
                                occurrences.len().saturating_sub(1)
                            }
                        };
                        if let Some(occurrence) = occurrences.get_mut(at) {
                            occurrence.events.push(id.clone());
                            // Which note of this expansion it is — the `▸ note 3`
                            // tail of the inspector's Origin row.
                            origin.note_index = u32::try_from(occurrence.events.len()).ok();
                            origin.occurrence = Some(occurrence.id.clone());
                        }
                    }
                    generated |= origin.generated;
                    let onset = event.onset.as_ratio();
                    let position = bars.at(musa_compiler::MusicalTime::new(onset));
                    let (bar, beat_in_bar) = (position.measure, position.beat);
                    let pitches = pitches_of(&event.kind);
                    // Exact rational arithmetic on musical time, which is not
                    // the integer arithmetic the lint is about.
                    #[expect(clippy::arithmetic_side_effects, reason = "exact rational musical time")]
                    let end = event.onset + event.notated_duration.value;
                    events.push(EventFacts {
                        id,
                        part: part.name().to_string(),
                        voice: name.clone(),
                        kind: kind_of(&event.kind),
                        pitch: pitches.first().cloned(),
                        pitches,
                        pitch_spellings: spellings_of(&event.kind),
                        duration: Fraction::from_ratio(event.notated_duration.value.as_ratio()),
                        duration_spelling: event.notated_duration.spelling.clone(),
                        bar,
                        beat: Fraction::from_ratio(beat_in_bar),
                        key: score
                            .key_at(
                                musa_compiler::Scope::Part { part: part.id().0 },
                                musa_compiler::MusicalTime::new(onset),
                            )
                            .map(|key| format!("{} {}", pitch_class(key.tonic()), mode(key.mode()))),
                        clef: score
                            .clef_at(part.id(), musa_compiler::MusicalTime::new(onset))
                            .map(|clef| clef.name().to_owned()),
                        onset_frames: tempo.frames(event.onset),
                        end_frames: tempo.frames(end),
                        origin,
                    });
                }
                voices.push(VoiceFacts { name, generated });
            }
            parts.push(PartFacts {
                name: part.name().to_string(),
                voices,
            });
        }

        let outline = outline_facts(score, &events, &lines, &bars, &tempo);
        // The same fallback performance uses: a piece with no metronome mark
        // is played at the default speed rather than not played.
        let opening = score
            .tempo_at(Scope::Piece, MusicalTime::ZERO)
            .and_then(|marking| marking.metronome)
            .unwrap_or_default();
        Self {
            title: score.title().to_string(),
            tempo_bpm: opening.bpm,
            tempo_beat: Fraction::from_ratio(opening.beat),
            key: score
                .key_at(Scope::Piece, MusicalTime::ZERO)
                .map(|key| format!("{} {}", pitch_class(key.tonic()), mode(key.mode()))),
            meter_count: score.meter_at(Scope::Piece, MusicalTime::ZERO).numerator(),
            meter_unit: score.meter_at(Scope::Piece, MusicalTime::ZERO).denominator(),
            parts,
            events,
            occurrences,
            outline,
            performance: score.performance(),
            decisions,
            // Read back off the source rather than off the compiled score:
            // what a field shows has to be what a field writes, and the
            // compiled score has already normalized `quarter = 72` into a
            // beat and a number.
            header: HeaderField::ALL
                .iter()
                .map(|field| HeaderFact {
                    field: *field,
                    value: musa_language::read_header(source, *field),
                })
                .collect(),
        }
    }
}

/// The outline, in the order the piece reaches its markers.
///
/// Sections are anchored to a time and phrases to their first event, so each
/// is resolved to the other coordinate here: the interface gets one list with
/// one shape, sorted the way a reader reads.
fn outline_facts(
    score: &ScoreSnapshot,
    events: &[EventFacts],
    lines: &LineIndex,
    bars: &musa_compiler::BarLines,
    tempo: &IntegratedTempoMap,
) -> Vec<OutlineFacts> {
    let mut rows: Vec<(u64, OutlineFacts)> = Vec::new();
    // A section runs until the next one; the last runs to the end of the
    // piece, which is the end of its last event.
    let ending = events.iter().map(|event| event.end_frames).max().unwrap_or_default();
    let starts: Vec<u64> = score
        .annotations()
        .sections()
        .iter()
        .map(|section| tempo.frames(section.at))
        .collect();
    for (index, section) in score.annotations().sections().iter().enumerate() {
        let position = bars.at(section.at);
        let (bar, beat_in_bar) = (position.measure, position.beat);
        let frames = tempo.frames(section.at);
        // The notehead a reader would look at: the first one that has not
        // already gone by when the marker is reached.
        let event = events
            .iter()
            .find(|event| event.onset_frames >= frames)
            .map(|event| event.id.clone());
        rows.push((
            frames,
            OutlineFacts {
                kind: OutlineKind::Section,
                name: section.name.clone(),
                bar,
                beat: Fraction::from_ratio(beat_in_bar),
                event,
                onset_frames: frames,
                end_frames: starts.get(index.saturating_add(1)).copied().unwrap_or(ending),
                line: lines.line_of(section.origin.source_span.start),
                span: crate::diagnostic::Span {
                    start: section.origin.source_span.start,
                    end: section.origin.source_span.end,
                },
            },
        ));
    }
    for phrase in score.annotations().phrases() {
        let id = format!("event-{:x}", phrase.from.0);
        let Some(event) = events.iter().find(|event| event.id == id) else {
            continue;
        };
        let last = format!("event-{:x}", phrase.to.0);
        let stops = events
            .iter()
            .find(|event| event.id == last)
            .map_or(event.end_frames, |event| event.end_frames);
        rows.push((
            event.onset_frames,
            OutlineFacts {
                kind: OutlineKind::Phrase,
                name: phrase.name.clone(),
                bar: event.bar,
                beat: event.beat,
                event: Some(event.id.clone()),
                onset_frames: event.onset_frames,
                end_frames: stops,
                line: lines.line_of(phrase.origin.source_span.start),
                span: crate::diagnostic::Span {
                    start: phrase.origin.source_span.start,
                    end: phrase.origin.source_span.end,
                },
            },
        ));
    }
    // A section names the passage a phrase inside it belongs to, so it comes
    // first when they start together.
    rows.sort_by_key(|(frames, row)| (*frames, row.kind == OutlineKind::Phrase));
    rows.into_iter().map(|(_, row)| row).collect()
}

/// One occurrence's row, built the first time an event from it is met.
fn occurrence_facts(
    at: usize,
    path: &[StepFact],
    steps: &[ExpansionStep],
    score: &ScoreSnapshot,
    lines: &LineIndex,
    source: &str,
) -> OccurrenceFacts {
    // The motif application is the innermost step, and therefore the last:
    // `transpose down P5 ▸ sigh()` is a `use` inside a transform block.
    let call_site = steps.iter().rev().find_map(|step| match *step {
        ExpansionStep::MotifApplication { call_site } => Some(call_site),
        ExpansionStep::RepeatIteration(_)
        | ExpansionStep::Transposition(_)
        | ExpansionStep::Stretch(_)
        | ExpansionStep::Retrograde
        | ExpansionStep::Inversion { .. }
        | ExpansionStep::MapNotePitches
        | ExpansionStep::ScaleContext { .. }
        | ExpansionStep::TemplateInstance { .. }
        | ExpansionStep::Assertion { .. }
        | ExpansionStep::Specialization { .. }
        | ExpansionStep::KernelSplice { .. } => None,
    });
    let motif = call_site.map(|span| motif_name(source, span.start, span.end));
    let declaration = motif.as_ref().and_then(|name| {
        score
            .motifs()
            .iter()
            .find(|declared| declared.name == *name)
            .map(|declared| crate::diagnostic::Span {
                start: declared.span.start,
                end: declared.span.end,
            })
    });
    let use_site = call_site.map_or(crate::diagnostic::Span { start: 0, end: 0 }, |span| {
        crate::diagnostic::Span {
            start: span.start,
            end: span.end,
        }
    });
    OccurrenceFacts {
        id: format!("occurrence-{at:x}"),
        path: path.to_vec(),
        label: path
            .iter()
            .map(|step| step.label.as_str())
            .collect::<Vec<_>>()
            .join(" \u{25b8} "),
        motif,
        declaration,
        use_site,
        line: lines.line_of(use_site.start),
        events: Vec::new(),
    }
}

/// `use sigh(a4);` → `sigh`.
fn motif_name(source: &str, start: u32, end: u32) -> String {
    let called = call_site_name(source, start, end);
    called
        .split_once('(')
        .map_or(called.as_str(), |(name, _)| name)
        .trim()
        .to_owned()
}

fn kind_of(kind: &ScoreEventKind) -> EventKind {
    match *kind {
        ScoreEventKind::Note { .. } => EventKind::Note,
        ScoreEventKind::Rest => EventKind::Rest,
        ScoreEventKind::Chord { .. } => EventKind::Chord,
    }
}

/// The pitches as the source spells them — `WrittenPitch`'s own `Display`,
/// which is the inverse of the parser it came from.
fn spellings_of(kind: &ScoreEventKind) -> Vec<String> {
    match kind {
        ScoreEventKind::Note { pitch } => vec![pitch.to_string()],
        ScoreEventKind::Rest => Vec::new(),
        ScoreEventKind::Chord { pitches } => pitches.iter().map(ToString::to_string).collect(),
    }
}

fn pitches_of(kind: &ScoreEventKind) -> Vec<String> {
    match kind {
        ScoreEventKind::Note { pitch } => vec![written(*pitch)],
        ScoreEventKind::Rest => Vec::new(),
        ScoreEventKind::Chord { pitches } => pitches.iter().map(|pitch| written(*pitch)).collect(),
    }
}

/// A pitch as a reader expects it — `A4`, `G♯4`, `B♭2` — rather than in the
/// language's ASCII spelling.
pub(crate) fn written(pitch: WrittenPitch) -> String {
    format!(
        "{}{}{}",
        pitch.letter.as_char().to_ascii_uppercase(),
        accidental(pitch.accidental.0),
        pitch.octave
    )
}

pub(crate) fn pitch_class(class: PitchClass) -> String {
    format!(
        "{}{}",
        class.letter.as_char().to_ascii_uppercase(),
        accidental(class.accidental.0)
    )
}

fn accidental(steps: i32) -> String {
    match steps {
        2 => "\u{1d12a}".to_owned(), // double sharp
        1 => "\u{266f}".to_owned(),  // sharp
        0 => String::new(),
        -1 => "\u{266d}".to_owned(),  // flat
        -2 => "\u{1d12b}".to_owned(), // double flat
        positive if positive > 0 => "\u{266f}".repeat(usize::try_from(positive).unwrap_or(usize::MAX)),
        negative => "\u{266d}".repeat(usize::try_from(negative.unsigned_abs()).unwrap_or(usize::MAX)),
    }
}

pub(crate) fn mode(mode: Mode) -> &'static str {
    match mode {
        Mode::Major => "major",
        Mode::Minor => "minor",
    }
}

fn origin_facts(origin: &Origin, lines: &LineIndex, source: &str) -> OriginFacts {
    OriginFacts {
        decision: None,
        supported_by: Vec::new(),
        generated: !origin.expansion_path.is_empty(),
        path: origin.expansion_path.iter().map(|it| step(it, source)).collect(),
        note_index: None,
        occurrence: None,
        line: lines.line_of(origin.source_span.start),
        span: crate::diagnostic::Span {
            start: origin.source_span.start,
            end: origin.source_span.end,
        },
        definition_span: crate::diagnostic::Span {
            start: origin.definition_span.start,
            end: origin.definition_span.end,
        },
    }
}

/// Which decision an event was played under: the innermost site whose source
/// spans it.
///
/// Containment rather than time, because a decision is *provenance* and the
/// Origin chain is a chain of source. A note inside a `repeat 4 to 16` was
/// written inside it; a note a mobile reordered was written inside the
/// `mobile` block; a freely-held note is its own site. Innermost wins, for the
/// reason the expansion path reads outside-in: the nearest question is the one
/// that produced this note.
fn decided_under(taken: &[musa_compiler::DecisionRecord], span: musa_compiler::SourceSpan) -> Option<usize> {
    taken
        .iter()
        .enumerate()
        .filter_map(|(index, record)| {
            let site = record
                .sites()
                .iter()
                .filter(|site| site.start <= span.start && span.end <= site.end)
                .min_by_key(|site| site.end.saturating_sub(site.start))?;
            Some((site.end.saturating_sub(site.start), index))
        })
        .min()
        .map(|(_, index)| index)
}

/// One expansion step, named the way the source names it.
///
/// A motif application records its call site rather than the motif's name, so
/// the name is read back out of the source at that span — `use sigh();`
/// becomes `sigh()`. Reading it here rather than in the frontend keeps the
/// rule that the interface renders what it is handed.
fn step(step: &ExpansionStep, source: &str) -> StepFact {
    let at = |span: musa_compiler::SourceSpan| {
        Some(crate::diagnostic::Span {
            start: span.start,
            end: span.end,
        })
    };
    match *step {
        ExpansionStep::MotifApplication { call_site } => StepFact {
            label: call_site_name(source, call_site.start, call_site.end),
            kind: StepKind::Occurrence,
            span: at(call_site),
        },
        // A transform block's own span is not on the step: the block is
        // written around the `use` beside it, and the occurrence's site
        // already opens the source there. Saying nothing beats pointing at
        // the wrong bracket.
        ExpansionStep::RepeatIteration(index) => transform(format!("repeat {}", index.saturating_add(1))),
        ExpansionStep::Transposition(interval) => transform(format!("transpose {}", interval_name(interval))),
        ExpansionStep::Stretch(factor) => transform(format!("stretch {}/{}", factor.numer(), factor.denom())),
        ExpansionStep::Retrograde => transform("retrograde".to_owned()),
        ExpansionStep::Inversion { ref axis } => transform(format!("invert around {axis}")),
        ExpansionStep::MapNotePitches => transform("map note pitches".to_owned()),
        ExpansionStep::ScaleContext { ref scale } => transform(format!("in {scale}")),
        // An assertion produces no music, so this step is the whole of what
        // it left behind — and the whole reason it is worth a segment is that
        // a composer can open the claim that covered this passage.
        ExpansionStep::Assertion { ref claim } => StepFact {
            label: format!("assert {claim}"),
            kind: StepKind::Assertion,
            span: None,
        },
        ExpansionStep::Specialization { override_site } => StepFact {
            label: "specialized".to_owned(),
            kind: StepKind::Specialization,
            span: at(override_site),
        },
        // The instance, not the template: two instances of one template are
        // two places, and Origin's job is to say which one this is.
        ExpansionStep::TemplateInstance { ref alias, site, .. } => StepFact {
            label: format!("make {alias}"),
            kind: StepKind::Instance,
            span: at(site),
        },
        // The locus, not the quote: a reader following a spliced note back
        // wants to know where in the assembled term it was put. The locus is
        // a *time*, not a place in the text, so there is nothing to open.
        ExpansionStep::KernelSplice { at: locus } => StepFact {
            label: format!("splice at {}/{}", locus.numer(), locus.denom()),
            kind: StepKind::Splice,
            span: None,
        },
    }
}

/// A transform step: prose, no site of its own.
fn transform(label: String) -> StepFact {
    StepFact {
        label,
        kind: StepKind::Transform,
        span: None,
    }
}

/// `use sigh();` → `sigh()`.
fn call_site_name(source: &str, start: u32, end: u32) -> String {
    let range = usize::try_from(start).unwrap_or(0)..usize::try_from(end).unwrap_or(0);
    let text = source.get(range).unwrap_or("").trim();
    let text = text.strip_prefix("use").unwrap_or(text).trim();
    // A specialized occurrence is still that motif's occurrence: the `with`
    // clause says what this one does differently, not what it is called.
    let text = text.split(" with").next().unwrap_or(text).trim();
    let text = text.strip_suffix(';').unwrap_or(text).trim();
    if text.is_empty() {
        "use".to_owned()
    } else {
        text.to_owned()
    }
}

fn interval_name(interval: Interval) -> String {
    let direction = if interval.semitones < 0 { "down" } else { "up" };
    let quality = match (interval.diatonic_steps.abs(), interval.semitones.abs()) {
        (0, 0) => "P1".to_owned(),
        (1, 1) => "m2".to_owned(),
        (1, 2) => "M2".to_owned(),
        (2, 3) => "m3".to_owned(),
        (2, 4) => "M3".to_owned(),
        (3, 5) => "P4".to_owned(),
        (4, 7) => "P5".to_owned(),
        (5, 8) => "m6".to_owned(),
        (5, 9) => "M6".to_owned(),
        (6, 10) => "m7".to_owned(),
        (6, 11) => "M7".to_owned(),
        (7, 12) => "P8".to_owned(),
        (steps, semitones) => format!("{steps}d{semitones}s"),
    };
    format!("{direction} {quality}")
}

/// What kind of thing a recorded name names.
///
/// A deliberate restatement of the compiler's `NameKind`, for the same
/// reason [`Severity`](crate::Severity) is one: the compiler's types stop
/// at this crate's boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NameKind {
    /// An immutable elaboration `let` binding.
    Value,
    /// A named elaboration function.
    Function,
    /// A `motif` declaration.
    Motif,
    /// A named `bar`.
    Bar,
    /// A `fragment` a mobile arranges.
    Fragment,
    /// A `part` in the score.
    Part,
    /// A `voice` in a part. Voices are declared, never used by name — their
    /// entries are declaration-only.
    Voice,
    /// A `patch` in the studio.
    Patch,
    /// A `signature` or a `structure`: static structure naming a group of
    /// declarations, never a value.
    Module,
    /// A `template`: named like a module, applied like a function by a `make`
    /// site.
    Template,
}

/// One named thing and everywhere it is spoken, for an editor's references
/// and rename.
///
/// Spans are the *name tokens'* spans: a rename rewrites exactly these
/// ranges, never a textual match. A name declared in an imported library has
/// `declaration: None` and an `external_declaration`, so editors can navigate
/// there without offering an incomplete one-document rename.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameFact {
    /// The name as written.
    pub name: String,
    /// What it names.
    pub kind: NameKind,
    /// Where the declaration's name token is, when it is in this document.
    pub declaration: Option<crate::diagnostic::Span>,
    /// Declaration in an imported source, when the name is not defined in
    /// the open document.
    pub external_declaration: Option<SourceLocation>,
    /// Every resolved use's name token, in the order the resolver met them.
    pub uses: Vec<crate::diagnostic::Span>,
}

/// A stable editor target in another source document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocation {
    /// Resolved filesystem path or readable virtual URI.
    pub uri: String,
    /// Byte range of the declaration name in that source.
    pub span: crate::diagnostic::Span,
}

impl NameKind {
    /// Restate one compiler kind in this crate's own vocabulary.
    pub(crate) fn from_compiler(kind: musa_compiler::NameKind) -> Self {
        match kind {
            musa_compiler::NameKind::Value => Self::Value,
            musa_compiler::NameKind::Function => Self::Function,
            musa_compiler::NameKind::Motif => Self::Motif,
            musa_compiler::NameKind::Bar => Self::Bar,
            musa_compiler::NameKind::Fragment => Self::Fragment,
            musa_compiler::NameKind::Part => Self::Part,
            musa_compiler::NameKind::Voice => Self::Voice,
            musa_compiler::NameKind::Patch => Self::Patch,
            musa_compiler::NameKind::Module => Self::Module,
            musa_compiler::NameKind::Template => Self::Template,
        }
    }
}

impl NameFact {
    /// Restate one compiler reference in this crate's own vocabulary.
    pub(crate) fn from_compiler(reference: &musa_compiler::NameReference) -> Self {
        let kind = NameKind::from_compiler(reference.kind);
        let span = |span: musa_compiler::SourceSpan| crate::diagnostic::Span {
            start: span.start,
            end: span.end,
        };
        Self {
            name: reference.name.clone(),
            kind,
            declaration: reference.declaration.map(span),
            external_declaration: reference.external_declaration.as_ref().map(|location| SourceLocation {
                uri: location.uri.clone(),
                span: span(location.span),
            }),
            uses: reference.uses.iter().map(|use_span| span(*use_span)).collect(),
        }
    }
}

/// A type as a reader meets it, with the line that tells it from its
/// look-alike.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeFact {
    /// As it is spelled in source: `NoteName`, `List<Pitch>`.
    pub name: String,
    /// The one line distinguishing this type from the one it is confused
    /// with — spelled `NoteName` against modulo-twelve `Pc12`, `Key` against
    /// `Scale`, `ChordClass` against `Voicing`. Absent for a compound type,
    /// which is distinguished by its shape.
    pub distinction: Option<String>,
}

/// One parameter of a callable declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterFact {
    /// The name it is called by, which is also the name it is passed by.
    pub name: String,
    /// The substring of [`ItemFact::signature`] this parameter occupies —
    /// what an editor highlights while the caller is writing it.
    pub label: String,
    /// Its type.
    pub ty: TypeFact,
}

/// What an editor says about one declaration.
///
/// A deliberate restatement of the compiler's `ItemDoc`, for the same reason
/// [`NameFact`] restates `NameReference`. The record answers the questions a
/// reader asks about a name — what is it, what does it take, what does it
/// mean, where is it written, may I edit it — and nothing else: no body, no
/// environment, no module table.
///
/// # Invariants
///
/// - `uri` is `None` exactly when `span` indexes the open document. A
///   consumer must never resolve a `Some(uri)` record's span against the
///   text it has open.
/// - `read_only` implies `uri` is `Some`: the open document is never
///   read-only. Bundled standard-library modules are read-only, and their
///   text is reached through
///   [`standard_library_source`](crate::standard_library_source).
/// - `result` is `Some` exactly when the declaration names a value; a
///   `signature` or `structure` names static structure and has none.
/// - every `label` in `parameters` appears verbatim in `signature`. The list
///   is empty for a value *and* for a nullary callable, which the signature
///   tells apart by writing the empty parameter list a caller must also write.
/// - Records are unique by `(name, kind)`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFact {
    /// The name as it is reached: `perfect_fifth`, or `Harmony.triad` when
    /// an import was qualified.
    pub name: String,
    /// What it names.
    pub kind: NameKind,
    /// The document it is declared in, when that is not the open one.
    pub uri: Option<String>,
    /// Its own name token, in that document.
    pub span: crate::diagnostic::Span,
    /// Whether an editor may write to it.
    pub read_only: bool,
    /// The comment block written directly above it, as one paragraph.
    pub summary: Option<String>,
    /// The declaration line without its body:
    /// `fn triad(root: NoteName) -> ChordClass`.
    pub signature: String,
    /// What it evaluates to.
    pub result: Option<TypeFact>,
    /// Its parameters, in order.
    pub parameters: Vec<ParameterFact>,
    /// What to write instead, when the declaration says it is deprecated.
    pub deprecation: Option<String>,
}

impl ItemFact {
    /// Restate one compiler record in this crate's own vocabulary.
    pub(crate) fn from_compiler(item: &musa_compiler::ItemDoc) -> Self {
        let ty = |note: &musa_compiler::TypeNote| TypeFact {
            name: note.name.clone(),
            distinction: note.distinction.map(str::to_owned),
        };
        Self {
            name: item.name.clone(),
            kind: NameKind::from_compiler(item.kind),
            uri: item.source.uri.clone(),
            span: crate::diagnostic::Span {
                start: item.source.span.start,
                end: item.source.span.end,
            },
            read_only: item.source.read_only,
            summary: item.summary.clone(),
            signature: item.signature.clone(),
            result: item.result.as_ref().map(&ty),
            parameters: item
                .parameters
                .iter()
                .map(|parameter| ParameterFact {
                    name: parameter.name.clone(),
                    label: parameter.label.clone(),
                    ty: ty(&parameter.ty),
                })
                .collect(),
            deprecation: item.deprecation.clone(),
        }
    }
}

/// Byte offset → 1-based line, built once per compile.
struct LineIndex {
    starts: Vec<u32>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(u32::try_from(offset).unwrap_or(u32::MAX).saturating_add(1));
            }
        }
        Self { starts }
    }

    fn line_of(&self, offset: u32) -> u32 {
        let index = self.starts.partition_point(|start| *start <= offset);
        u32::try_from(index).unwrap_or(1).max(1)
    }
}
