//! The `NotationPlan` (roadmap §12.1): measures, voice lanes, beaming, and
//! tie decomposition derived from a `ScoreSnapshot`. Semantic, not
//! typographic: no line breaks, spacing, or engraving decisions.
//!
//! Duration decomposition and measure splitting use exact rational
//! arithmetic; the workspace arithmetic lint is allowed at module scope (see
//! `musa-compiler/src/time.rs` for the totality argument).
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{
    Clef, EventId, KeyMap, MeterMap, Mode, MusicalDuration, MusicalTime, NotatedDuration, Part, ScoreEvent,
    ScoreEventKind, ScoreSnapshot, Voice, VoiceId, WrittenPitch,
};
use num_rational::Ratio;

/// Options for notation planning. Empty until a real choice exists (beaming
/// preferences, part extraction); the type fixes the facade shape.
#[derive(Clone, Debug, Default)]
pub struct NotationOptions {}

/// The backend-neutral plan for one score.
#[derive(Clone, Debug)]
pub struct NotationPlan {
    staves: Vec<StaffPlan>,
}

impl NotationPlan {
    /// One staff per part, in source order.
    pub fn staves(&self) -> &[StaffPlan] {
        &self.staves
    }
}

/// A key signature element: fifths plus mode. Affects the signature only,
/// never pitch spelling (§6.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeySignature {
    /// Sharps (positive) or flats (negative) on the circle of fifths.
    pub fifths: i8,
    /// Major or minor.
    pub mode: Mode,
}

/// The plan for one staff (one part).
#[derive(Clone, Debug)]
pub struct StaffPlan {
    name: String,
    clef: Option<Clef>,
    key: Option<KeySignature>,
    time_signature: (u32, u32),
    measures: Vec<MeasurePlan>,
}

impl StaffPlan {
    /// The part name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The written clef, if declared.
    pub fn clef(&self) -> Option<Clef> {
        self.clef
    }

    /// The key signature, if the piece declares a key.
    pub fn key_signature(&self) -> Option<KeySignature> {
        self.key
    }

    /// The time signature as `(numerator, denominator)`.
    pub fn time_signature(&self) -> (u32, u32) {
        self.time_signature
    }

    /// Measures in order, starting at number 1.
    pub fn measures(&self) -> &[MeasurePlan] {
        &self.measures
    }
}

/// One measure of one staff.
#[derive(Clone, Debug)]
pub struct MeasurePlan {
    number: u32,
    lanes: Vec<VoiceLane>,
}

impl MeasurePlan {
    /// The 1-based measure number.
    pub fn number(&self) -> u32 {
        self.number
    }

    /// Voice lanes sounding in this measure (one per voice of the part).
    pub fn lanes(&self) -> &[VoiceLane] {
        &self.lanes
    }
}

/// One voice's notated content inside one measure.
#[derive(Clone, Debug)]
pub struct VoiceLane {
    voice: VoiceId,
    name: String,
    items: Vec<NotatedItem>,
}

impl VoiceLane {
    /// The voice identity in the part.
    pub fn voice(&self) -> VoiceId {
        self.voice
    }

    /// The voice name from the source.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Notated items in onset order.
    pub fn items(&self) -> &[NotatedItem] {
        &self.items
    }
}

/// A beam group identifier within a lane's measure (the beat index).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BeamGroup(pub u32);

/// One notated symbol: a note, rest, or chord with one spelled duration.
/// Cross-measure events appear once per piece; all pieces of one event share
/// its `EventId` and are chained by tie flags.
#[derive(Clone, Debug)]
pub struct NotatedItem {
    event: EventId,
    kind: NotatedKind,
    onset_in_measure: MusicalDuration,
    duration: NotatedDuration,
    tie_start: bool,
    tie_stop: bool,
    beam: Option<BeamGroup>,
}

impl NotatedItem {
    /// The score event this symbol (or tie piece) came from.
    pub fn event(&self) -> EventId {
        self.event
    }

    /// Note, rest, or chord content.
    pub fn kind(&self) -> &NotatedKind {
        &self.kind
    }

    /// The onset within the measure, in whole notes.
    pub fn onset_in_measure(&self) -> MusicalDuration {
        self.onset_in_measure
    }

    /// This piece's spelled duration.
    pub fn duration(&self) -> &NotatedDuration {
        &self.duration
    }

    /// A tie starts here (the event continues in the next piece).
    pub fn tie_start(&self) -> bool {
        self.tie_start
    }

    /// A tie ends here (the event began in an earlier piece).
    pub fn tie_stop(&self) -> bool {
        self.tie_stop
    }

    /// The beam group, when this item beams with its beat neighbors.
    pub fn beam(&self) -> Option<BeamGroup> {
        self.beam
    }
}

/// The content of a notated item.
#[derive(Clone, Debug)]
pub enum NotatedKind {
    /// A single written pitch.
    Note {
        /// The spelled pitch, verbatim from the score.
        pitch: WrittenPitch,
    },
    /// A rest.
    Rest,
    /// A chord.
    Chord {
        /// The spelled pitches, in source order.
        pitches: Vec<WrittenPitch>,
    },
}

/// Plan notation for a compiled score.
///
/// # Errors
/// Returns [`NotationError`] when a duration cannot be spelled with standard
/// values and ties inside one measure.
pub fn plan_notation(score: &ScoreSnapshot, _options: &NotationOptions) -> Result<NotationPlan, crate::NotationError> {
    let measure_len = score.meter_map.measure_len().as_ratio();
    let key = score.key_map.map(key_signature);
    let mut staves = Vec::new();
    for (_, part) in score.parts.iter() {
        staves.push(plan_staff(part, score.meter_map, measure_len, key)?);
    }
    Ok(NotationPlan { staves })
}

fn key_signature(key: KeyMap) -> KeySignature {
    let letter_fifths: i8 = match key.tonic.letter {
        musa_compiler::Letter::C => 0,
        musa_compiler::Letter::G => 1,
        musa_compiler::Letter::D => 2,
        musa_compiler::Letter::A => 3,
        musa_compiler::Letter::E => 4,
        musa_compiler::Letter::B => 5,
        musa_compiler::Letter::F => -1,
    };
    let major = letter_fifths.saturating_add(key.tonic.accidental.0.saturating_mul(7));
    let fifths = match key.mode {
        Mode::Major => major,
        // A minor key's signature is its relative major's: three fifths down.
        Mode::Minor => major.saturating_sub(3),
    };
    KeySignature { fifths, mode: key.mode }
}

fn plan_staff(
    part: &Part,
    meter: MeterMap,
    measure_len: Ratio<i64>,
    key: Option<KeySignature>,
) -> Result<StaffPlan, crate::NotationError> {
    let span = part.voices.values().map(Voice::span).max().unwrap_or_default();
    let measure_count = if measure_len == Ratio::ZERO {
        1
    } else {
        let count = span.as_ratio() / measure_len;
        let count = if *count.denom() == 1 {
            *count.numer()
        } else {
            count.numer() / count.denom() + 1
        };
        u32::try_from(count.max(1)).unwrap_or(u32::MAX)
    };

    let mut measures = Vec::new();
    for index in 0..measure_count {
        let start = MusicalTime::new(measure_len * Ratio::from_integer(i64::from(index)));
        let end = MusicalTime::new(measure_len * Ratio::from_integer(i64::from(index.saturating_add(1))));
        let mut lanes = Vec::new();
        for (voice_id, voice) in &part.voices {
            let name = part.voice_names.get(voice_id).cloned().unwrap_or_default();
            let items = plan_lane(voice, meter, measure_len, start, end)?;
            lanes.push(VoiceLane {
                voice: *voice_id,
                name,
                items,
            });
        }
        measures.push(MeasurePlan {
            number: index.saturating_add(1),
            lanes,
        });
    }
    Ok(StaffPlan {
        name: part.name.clone(),
        clef: part.clef,
        key,
        time_signature: (meter.numerator, meter.denominator),
        measures,
    })
}

/// The beat unit for beaming: compound meters (`6/8`, `9/8`, `12/8`) beam in
/// groups of three eighths; simple meters beam per notated beat.
fn beam_unit(meter: MeterMap) -> Ratio<i64> {
    if meter.denominator == 8 && meter.numerator.is_multiple_of(3) && meter.numerator > 3 {
        Ratio::new(3, 8)
    } else {
        Ratio::new(1, i64::from(meter.denominator))
    }
}

/// Notate one voice's events inside one measure.
fn plan_lane(
    voice: &Voice,
    meter: MeterMap,
    measure_len: Ratio<i64>,
    start: MusicalTime,
    end: MusicalTime,
) -> Result<Vec<NotatedItem>, crate::NotationError> {
    let mut items = Vec::new();
    for event in &voice.events {
        let event_end = event.onset + event.notated_duration.value;
        if event_end <= start || event.onset >= end {
            continue;
        }
        let piece_start = event.onset.max(start);
        let piece_end = event_end.min(end);
        let piece_len = MusicalDuration::new(piece_end.as_ratio() - piece_start.as_ratio());
        let onset_in_measure = MusicalDuration::new(piece_start.as_ratio() - start.as_ratio());
        let pieces = decompose(event, piece_len)?;
        let piece_count = pieces.len();
        let continues_after = event_end > end;
        let started_before = event.onset < start;
        for (piece_index, piece) in pieces.into_iter().enumerate() {
            let is_first = piece_index == 0 && !started_before;
            let is_last = piece_index + 1 == piece_count && !continues_after;
            let tied = !matches!(event.kind, ScoreEventKind::Rest);
            let (tie_start, tie_stop) = if tied { (!is_last, !is_first) } else { (false, false) };
            items.push(NotatedItem {
                event: event.id,
                kind: kind_of(event),
                onset_in_measure,
                duration: piece,
                tie_start,
                tie_stop,
                beam: None,
            });
        }
    }
    assign_beams(&mut items, meter, measure_len);
    Ok(items)
}

fn kind_of(event: &ScoreEvent) -> NotatedKind {
    match &event.kind {
        ScoreEventKind::Note { pitch } => NotatedKind::Note { pitch: *pitch },
        ScoreEventKind::Rest => NotatedKind::Rest,
        ScoreEventKind::Chord { pitches } => NotatedKind::Chord {
            pitches: pitches.clone(),
        },
    }
}

/// Beam consecutive eighth-and-shorter items that stay inside one beat.
fn assign_beams(items: &mut [NotatedItem], meter: MeterMap, measure_len: Ratio<i64>) {
    let unit = beam_unit(meter);
    let eighth = Ratio::new(1, 8);
    for item in items.iter_mut() {
        let short = item.duration.value.as_ratio() <= eighth;
        let pitched = !matches!(item.kind, NotatedKind::Rest);
        if !short || !pitched {
            continue;
        }
        let onset = item.onset_in_measure.as_ratio();
        let item_end = onset + item.duration.value.as_ratio();
        if item_end > measure_len {
            continue;
        }
        let beat = onset / unit;
        let beat_index = beat.numer() / beat.denom();
        let beat_end = Ratio::from_integer(beat_index.saturating_add(1)) * unit;
        if item_end <= beat_end {
            item.beam = Some(BeamGroup(u32::try_from(beat_index).unwrap_or(u32::MAX)));
        }
    }
}

/// Decompose a duration inside one measure into standard note values with
/// ties: powers of two (`1/2`, `1/4`, …) and dotted values (`3/4`, `3/8`, …).
/// Anything whose reduced denominator is not a power of two needs tuplets.
fn decompose(event: &ScoreEvent, duration: MusicalDuration) -> Result<Vec<NotatedDuration>, crate::NotationError> {
    let value = duration.as_ratio();
    let denominator = *value.denom();
    if denominator <= 0 || (denominator & (denominator - 1)) != 0 {
        return Err(crate::NotationError::UnspellableDuration {
            event: event.id,
            duration: event.notated_duration.spelling.clone(),
        });
    }
    let mut remaining = value;
    let mut pieces = Vec::new();
    while remaining > Ratio::ZERO {
        let piece = largest_value(remaining);
        pieces.push(NotatedDuration {
            value: MusicalDuration::new(piece),
            spelling: event.notated_duration.spelling.clone(),
        });
        remaining -= piece;
    }
    Ok(pieces)
}

/// The largest standard value (plain or dotted) not exceeding `limit`.
fn largest_value(limit: Ratio<i64>) -> Ratio<i64> {
    // Whole note and smaller: 1, 3/4, 1/2, 3/8, 1/4, 3/16, 1/8, …
    let mut plain = Ratio::from_integer(1);
    while plain > limit {
        plain /= 2;
    }
    let dotted = plain * Ratio::new(3, 2);
    if dotted <= limit { dotted } else { plain }
}
