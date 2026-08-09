//! Everything the interface displays about a score, stated once, in words.
//!
//! `docs/interface/03-interaction.md` §7 is an exhaustive list of what the
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
    fn from_ratio(ratio: num_rational::Ratio<i64>) -> Self {
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

/// Where an event came from (`docs/interface/04-provenance.md` §3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginFacts {
    /// False when the user typed this note; true when the compiler produced
    /// it by expanding something.
    pub generated: bool,
    /// The expansion path, one display segment per step: `["sigh()",
    /// "transpose down P5"]`. Empty for authored events.
    pub path: Vec<String>,
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
}

/// One expansion, and everything it produced
/// (`docs/interface/04-provenance.md` §2).
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
    /// The expansion path, outside in: `["transpose down P5", "sigh()"]`.
    pub path: Vec<String>,
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
    /// (prompt 13's contract), so a click on the page finds this row.
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
    /// The same pitches as the *language* spells them (`gs4`), not as a
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
    /// Beats per minute, at the beat unit below.
    pub tempo_bpm: u32,
    /// The tempo's beat unit as a fraction of a whole note (`1/4`).
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
    /// makes it (prompt 54).
    pub header: Vec<HeaderFact>,
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
    pub(crate) fn derive(score: &ScoreSnapshot, source: &str) -> Self {
        let lines = LineIndex::new(source);
        // The same tempo integration the performance lowering uses, at the
        // same options, so a frame here is the frame the engine will report.
        let tempo = IntegratedTempoMap::new(score, &PerformanceOptions::default());
        // Unfolded: the fact index reports where a moment *sounds*, which is
        // the coordinate the snapshot's own barlines are built over.
        let bars = score.bars();

        let mut parts = Vec::new();
        let mut events = Vec::new();
        // Occurrences, and the expansion paths that identify them. Identity is
        // the path itself — call-site spans and transform arguments included —
        // so two `use sigh()` statements are two occurrences even though they
        // read the same on the page.
        let mut occurrences: Vec<OccurrenceFacts> = Vec::new();
        let mut paths: Vec<Vec<ExpansionStep>> = Vec::new();
        for (_, part) in score.parts().iter() {
            let mut voices = Vec::new();
            for (voice_id, voice) in part.voices() {
                let name = part
                    .voice_name(voice_id)
                    .map_or_else(|| voice_id.0.to_string(), ToString::to_string);
                let mut generated = false;
                for event in voice.events() {
                    let id = format!("event-{:x}", event.id.0);
                    let mut origin = origin_facts(&event.origin, &lines, source);
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
        Self {
            title: score.title().to_string(),
            tempo_bpm: score.tempo().bpm,
            tempo_beat: Fraction::from_ratio(score.tempo().beat),
            key: score
                .key_at(Scope::Piece, MusicalTime::ZERO)
                .map(|key| format!("{} {}", pitch_class(key.tonic()), mode(key.mode()))),
            meter_count: score.meter_at(Scope::Piece, MusicalTime::ZERO).numerator(),
            meter_unit: score.meter_at(Scope::Piece, MusicalTime::ZERO).denominator(),
            parts,
            events,
            occurrences,
            outline,
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
    path: &[String],
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
        | ExpansionStep::Specialization { .. } => None,
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
        label: path.join(" \u{25b8} "),
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
fn written(pitch: WrittenPitch) -> String {
    format!(
        "{}{}{}",
        pitch.letter.as_char().to_ascii_uppercase(),
        accidental(pitch.accidental.0),
        pitch.octave
    )
}

fn pitch_class(class: PitchClass) -> String {
    format!(
        "{}{}",
        class.letter.as_char().to_ascii_uppercase(),
        accidental(class.accidental.0)
    )
}

fn accidental(steps: i8) -> &'static str {
    match steps {
        2 => "\u{1d12a}", // double sharp
        1 => "\u{266f}",  // sharp
        0 => "",
        -1 => "\u{266d}",  // flat
        -2 => "\u{1d12b}", // double flat
        _ => "?",
    }
}

fn mode(mode: Mode) -> &'static str {
    match mode {
        Mode::Major => "major",
        Mode::Minor => "minor",
    }
}

fn origin_facts(origin: &Origin, lines: &LineIndex, source: &str) -> OriginFacts {
    OriginFacts {
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

/// One expansion step, named the way the source names it.
///
/// A motif application records its call site rather than the motif's name, so
/// the name is read back out of the source at that span — `use sigh();`
/// becomes `sigh()`. Reading it here rather than in the frontend keeps the
/// rule that the interface renders what it is handed.
fn step(step: &ExpansionStep, source: &str) -> String {
    match *step {
        ExpansionStep::MotifApplication { call_site } => call_site_name(source, call_site.start, call_site.end),
        ExpansionStep::RepeatIteration(index) => format!("repeat {}", index.saturating_add(1)),
        ExpansionStep::Transposition(interval) => format!("transpose {}", interval_name(interval)),
        ExpansionStep::Stretch(factor) => format!("stretch {}/{}", factor.numer(), factor.denom()),
        ExpansionStep::Retrograde => "retrograde".to_owned(),
        ExpansionStep::Inversion { ref axis } => format!("invert around {axis}"),
        ExpansionStep::Specialization { .. } => "specialized".to_owned(),
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

/// What kind of thing a recorded name names (prompt 78).
///
/// A deliberate restatement of the compiler's `NameKind`, for the same
/// reason [`Severity`](crate::Severity) is one: the compiler's types stop
/// at this crate's boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NameKind {
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
}

/// One named thing and everywhere it is spoken, for an editor's references
/// and rename (prompt 78).
///
/// Spans are the *name tokens'* spans: a rename rewrites exactly these
/// ranges, never a textual match. A name declared in an imported library has
/// `declaration: None` — its uses here are recorded, and cross-file rename
/// is impossible to ask for rather than silently wrong.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameFact {
    /// The name as written.
    pub name: String,
    /// What it names.
    pub kind: NameKind,
    /// Where the declaration's name token is, when it is in this document.
    pub declaration: Option<crate::diagnostic::Span>,
    /// Every resolved use's name token, in the order the resolver met them.
    pub uses: Vec<crate::diagnostic::Span>,
}

impl NameFact {
    /// Restate one compiler reference in this crate's own vocabulary.
    pub(crate) fn from_compiler(reference: &musa_compiler::NameReference) -> Self {
        let kind = match reference.kind {
            musa_compiler::NameKind::Motif => NameKind::Motif,
            musa_compiler::NameKind::Bar => NameKind::Bar,
            musa_compiler::NameKind::Fragment => NameKind::Fragment,
            musa_compiler::NameKind::Part => NameKind::Part,
            musa_compiler::NameKind::Voice => NameKind::Voice,
            musa_compiler::NameKind::Patch => NameKind::Patch,
        };
        let span = |span: musa_compiler::SourceSpan| crate::diagnostic::Span {
            start: span.start,
            end: span.end,
        };
        Self {
            name: reference.name.clone(),
            kind,
            declaration: reference.declaration.map(span),
            uses: reference.uses.iter().map(|use_span| span(*use_span)).collect(),
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
