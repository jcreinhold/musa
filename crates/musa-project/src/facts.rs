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
    ExpansionStep, IntegratedTempoMap, Interval, Mode, Origin, PerformanceOptions, PitchClass, ScoreEventKind,
    ScoreSnapshot, WrittenPitch,
};
use serde::Serialize;

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
    /// 1-based line in the source that (transitively) produced the event.
    pub line: u32,
    /// The source byte range, for revealing it in the drawer.
    pub span: crate::diagnostic::Span,
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
    /// The notated duration's exact value in whole notes.
    pub duration: Fraction,
    /// How the duration is written in the source (`1/2`, `3/8`).
    pub duration_spelling: String,
    /// 1-based bar number.
    pub bar: u32,
    /// 1-based beat within the bar, exact.
    pub beat: Fraction,
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
        let measure = score.meter_map.measure_len().as_ratio();
        let beat = num_rational::Ratio::new(1, i64::from(score.meter_map.denominator).max(1));

        let mut parts = Vec::new();
        let mut events = Vec::new();
        for (_, part) in score.parts.iter() {
            let mut voices = Vec::new();
            for (voice_id, voice) in &part.voices {
                let name = part
                    .voice_names
                    .get(voice_id)
                    .cloned()
                    .unwrap_or_else(|| voice_id.0.to_string());
                let mut generated = false;
                // How many notes of the current occurrence have been seen, so
                // the inspector can say "note 3 of sigh()".
                let mut occurrence: Option<(Vec<String>, u32)> = None;
                for event in &voice.events {
                    let mut origin = origin_facts(&event.origin, &lines, source);
                    if origin.generated {
                        let index = match occurrence.take() {
                            Some((path, count)) if path == origin.path => count.saturating_add(1),
                            _ => 1,
                        };
                        occurrence = Some((origin.path.clone(), index));
                        origin.note_index = Some(index);
                    } else {
                        occurrence = None;
                    }
                    generated |= origin.generated;
                    let onset = event.onset.as_ratio();
                    let (bar, beat_in_bar) = position(onset, measure, beat);
                    let pitches = pitches_of(&event.kind);
                    // Exact rational arithmetic on musical time, which is not
                    // the integer arithmetic the lint is about.
                    #[expect(clippy::arithmetic_side_effects, reason = "exact rational musical time")]
                    let end = event.onset + event.notated_duration.value;
                    events.push(EventFacts {
                        id: format!("event-{:x}", event.id.0),
                        part: part.name.clone(),
                        voice: name.clone(),
                        kind: kind_of(&event.kind),
                        pitch: pitches.first().cloned(),
                        pitches,
                        duration: Fraction::from_ratio(event.notated_duration.value.as_ratio()),
                        duration_spelling: event.notated_duration.spelling.clone(),
                        bar,
                        beat: Fraction::from_ratio(beat_in_bar),
                        onset_frames: tempo.frames(event.onset),
                        end_frames: tempo.frames(end),
                        origin,
                    });
                }
                voices.push(VoiceFacts { name, generated });
            }
            parts.push(PartFacts {
                name: part.name.clone(),
                voices,
            });
        }

        Self {
            title: score.title.clone(),
            tempo_bpm: score.tempo_map.bpm,
            tempo_beat: Fraction::from_ratio(score.tempo_map.beat),
            key: score
                .key_map
                .map(|key| format!("{} {}", pitch_class(key.tonic), mode(key.mode))),
            meter_count: score.meter_map.numerator,
            meter_unit: score.meter_map.denominator,
            parts,
            events,
        }
    }
}

/// Bar and beat, both 1-based, from an onset in whole notes.
///
/// Rational arithmetic on `Ratio<i64>` has no cheap checked form, and the
/// zero denominators that would make it misbehave are rejected on the line
/// above; the same allowance is made in `musa-compiler`'s time module.
#[allow(clippy::arithmetic_side_effects)]
fn position(
    onset: num_rational::Ratio<i64>,
    measure: num_rational::Ratio<i64>,
    beat: num_rational::Ratio<i64>,
) -> (u32, num_rational::Ratio<i64>) {
    if *measure.numer() == 0 || *beat.numer() == 0 {
        return (1, num_rational::Ratio::from_integer(1));
    }
    let bars = (onset / measure).floor().to_integer();
    let into_bar = onset - measure * num_rational::Ratio::from_integer(bars);
    let bar = u32::try_from(bars.saturating_add(1)).unwrap_or(1);
    (bar, into_bar / beat + num_rational::Ratio::from_integer(1))
}

fn kind_of(kind: &ScoreEventKind) -> EventKind {
    match *kind {
        ScoreEventKind::Note { .. } => EventKind::Note,
        ScoreEventKind::Rest => EventKind::Rest,
        ScoreEventKind::Chord { .. } => EventKind::Chord,
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
        line: lines.line_of(origin.source_span.start),
        span: crate::diagnostic::Span {
            start: origin.source_span.start,
            end: origin.source_span.end,
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
    }
}

/// `use sigh();` → `sigh()`.
fn call_site_name(source: &str, start: u32, end: u32) -> String {
    let range = usize::try_from(start).unwrap_or(0)..usize::try_from(end).unwrap_or(0);
    let text = source.get(range).unwrap_or("").trim();
    let text = text.strip_prefix("use").unwrap_or(text).trim();
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
