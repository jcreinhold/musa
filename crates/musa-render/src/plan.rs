//! The `NotationPlan` (roadmap §12.1): measures, voice lanes, beaming, and
//! tie decomposition derived from a `ScoreSnapshot`. Semantic, not
//! typographic: no line breaks, spacing, or engraving decisions.
//!
//! Duration decomposition and measure splitting use exact rational
//! arithmetic; the workspace arithmetic lint is allowed at module scope (see
//! `musa-compiler/src/time.rs` for the totality argument).
#![allow(clippy::arithmetic_side_effects)]

use std::collections::{HashMap, HashSet};

use musa_compiler::{
    ArticulationMark, Clef, DynamicMark, EventId, KeyMap, MeterMap, Mode, MusicalDuration, MusicalTime,
    NotatedDuration, Part, ScoreEvent, ScoreEventKind, ScoreSnapshot, Voice, VoiceId, WrittenPitch,
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
    slurs: Vec<SlurRange>,
}

/// A slur that begins in this lane's measure, by the events it joins.
///
/// The items carry the same fact as start/stop flags, which is what an
/// inline syntax like `LilyPond`'s wants; a backend that anchors a slur by
/// identity needs both ends at once, and the far end is often in a later
/// measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlurRange {
    /// The first slurred event.
    pub from: EventId,
    /// The last slurred event.
    pub to: EventId,
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

    /// Slurs beginning in this measure, in onset order.
    pub fn slurs(&self) -> &[SlurRange] {
        &self.slurs
    }
}

/// A beam group identifier within a lane's measure (the beat index).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BeamGroup(pub u32);

/// Which side of the staff a symbol is printed on.
///
/// The plan decides this so the backends cannot disagree about it: a slur
/// that arched above in MEI and below in `LilyPond` would be two different
/// engravings of one score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Above the staff.
    Above,
    /// Below the staff.
    Below,
}

/// A tuplet bracket over a run of items: `num` written values in the time of
/// `den` of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TupletMark {
    /// How many written values the group holds.
    pub num: u32,
    /// How many of those values it lasts.
    pub den: u32,
    /// The bracket opens at this item.
    pub start: bool,
    /// The bracket closes at this item.
    pub stop: bool,
}

/// Where one item sits inside a spanning mark: the span may open here, close
/// here, both (a one-item span), or neither.
#[derive(Clone, Copy, Default)]
struct Edges {
    start: bool,
    stop: bool,
}

/// One word per item in the plan snapshots, which are the review surface for
/// this module: two nested `bool` fields per span would bury the change under
/// the shape that carries it.
impl std::fmt::Debug for Edges {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match (self.start, self.stop) {
            (false, false) => "none",
            (true, false) => "start",
            (false, true) => "stop",
            (true, true) => "start+stop",
        })
    }
}

/// One notated symbol: a note, rest, or chord with one spelled duration.
/// Cross-measure events appear once per piece; all pieces of one event share
/// its `EventId` and are chained by tie flags.
#[derive(Clone, Debug)]
pub struct NotatedItem {
    event: EventId,
    kind: NotatedKind,
    onset_in_measure: MusicalDuration,
    duration: NotatedDuration,
    tie: Edges,
    beam: Option<BeamGroup>,
    tuplet: Option<TupletMark>,
    slur: Edges,
    dynamic: Option<DynamicMark>,
    articulations: Vec<ArticulationMark>,
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
        self.tie.start
    }

    /// A tie ends here (the event began in an earlier piece).
    pub fn tie_stop(&self) -> bool {
        self.tie.stop
    }

    /// The beam group, when this item beams with its beat neighbors.
    pub fn beam(&self) -> Option<BeamGroup> {
        self.beam
    }

    /// The tuplet bracket this item belongs to, if any.
    pub fn tuplet(&self) -> Option<TupletMark> {
        self.tuplet
    }

    /// A slur begins at this item.
    pub fn slur_start(&self) -> bool {
        self.slur.start
    }

    /// A slur ends at this item.
    pub fn slur_stop(&self) -> bool {
        self.slur.stop
    }

    /// The dynamic marking printed at this item, if any.
    pub fn dynamic(&self) -> Option<DynamicMark> {
        self.dynamic
    }

    /// Articulations printed on this item, in written order.
    pub fn articulations(&self) -> &[ArticulationMark] {
        &self.articulations
    }
}

/// Where slurs are printed. One rule, because the plan has no stem
/// directions to reason from and a convention beats an inconsistency.
pub const SLUR_PLACEMENT: Placement = Placement::Above;

/// Where articulations are printed.
pub const ARTICULATION_PLACEMENT: Placement = Placement::Above;

/// Where dynamic markings are printed.
pub const DYNAMIC_PLACEMENT: Placement = Placement::Below;

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
    let marks = Marks::collect(score);
    let mut staves = Vec::new();
    for (_, part) in score.parts.iter() {
        staves.push(plan_staff(part, score.meter_map, measure_len, key, &marks)?);
    }
    Ok(NotationPlan { staves })
}

/// The annotation store, indexed the way planning reads it: by the event a
/// symbol belongs to.
#[derive(Default)]
struct Marks {
    tuplets: HashMap<EventId, TupletMark>,
    slur_ends: HashMap<EventId, EventId>,
    slur_stops: HashSet<EventId>,
    dynamics: HashMap<EventId, DynamicMark>,
    articulations: HashMap<EventId, Vec<ArticulationMark>>,
}

impl Marks {
    fn collect(score: &ScoreSnapshot) -> Self {
        let annotations = &score.annotations;
        let mut marks = Self::default();
        for slur in annotations.slurs() {
            marks.slur_ends.insert(slur.from, slur.to);
            marks.slur_stops.insert(slur.to);
        }
        for dynamic in annotations.dynamics() {
            marks.dynamics.insert(dynamic.at, dynamic.mark);
        }
        for articulation in annotations.articulations() {
            marks
                .articulations
                .entry(articulation.at)
                .or_default()
                .push(articulation.mark);
        }
        // Event ids run consecutively within a voice, so a group's members
        // are exactly the ids between its ends (musa-compiler's `identify`).
        for tuplet in annotations.tuplets() {
            for raw in tuplet.from.0..=tuplet.to.0 {
                marks.tuplets.insert(
                    EventId(raw),
                    TupletMark {
                        num: tuplet.num,
                        den: tuplet.den,
                        start: raw == tuplet.from.0,
                        stop: raw == tuplet.to.0,
                    },
                );
            }
        }
        marks
    }

    /// What a tuplet does to a written value: a `3/2` triplet eighth sounds
    /// `1/12` and is printed as the `1/8` it was written as.
    fn symbol_scale(&self, event: EventId) -> Ratio<i64> {
        self.tuplets.get(&event).map_or(Ratio::ONE, |tuplet| {
            Ratio::new(i64::from(tuplet.num), i64::from(tuplet.den))
        })
    }
}

fn key_signature(key: KeyMap) -> KeySignature {
    KeySignature {
        fifths: key.fifths(),
        mode: key.mode,
    }
}

fn plan_staff(
    part: &Part,
    meter: MeterMap,
    measure_len: Ratio<i64>,
    key: Option<KeySignature>,
    marks: &Marks,
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
            let lane = plan_lane(voice, meter, measure_len, start, end, marks)?;
            lanes.push(VoiceLane {
                voice: *voice_id,
                name,
                items: lane.items,
                slurs: lane.slurs,
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
    marks: &Marks,
) -> Result<Lane, crate::NotationError> {
    let mut items = Vec::new();
    for event in &voice.events {
        let event_end = event.onset + event.notated_duration.value;
        if event_end <= start || event.onset >= end {
            continue;
        }
        let scale = marks.symbol_scale(event.id);
        // The noteheads this event needs here: the pieces the composer wrote,
        // each clipped to this measure and then split into standard values.
        let mut pieces: Vec<(MusicalTime, NotatedDuration)> = Vec::new();
        let mut cursor = event.onset;
        for written in &event.notated_duration.pieces {
            let piece_start = cursor;
            let piece_end = cursor + *written;
            cursor = piece_end;
            if piece_end <= start || piece_start >= end {
                continue;
            }
            let clipped_start = piece_start.max(start);
            let clipped_end = piece_end.min(end);
            let sounding = MusicalDuration::new(clipped_end.as_ratio() - clipped_start.as_ratio());
            let symbol = MusicalDuration::new(sounding.as_ratio() * scale);
            let mut at = clipped_start;
            for value in decompose(event, symbol)? {
                let sounds = MusicalDuration::new(value.value.as_ratio() / scale);
                pieces.push((at, value));
                at = at + sounds;
            }
        }
        let piece_count = pieces.len();
        let continues_after = event_end > end;
        let started_before = event.onset < start;
        let tuplet = marks.tuplets.get(&event.id).copied();
        for (piece_index, (at, piece)) in pieces.into_iter().enumerate() {
            let is_first = piece_index == 0 && !started_before;
            let is_last = piece_index + 1 == piece_count && !continues_after;
            let tied = !matches!(event.kind, ScoreEventKind::Rest);
            let tie = if tied {
                Edges {
                    start: !is_last,
                    stop: !is_first,
                }
            } else {
                Edges::default()
            };
            // A symbol that spells one event only once carries what was
            // written on that event: an accent on the first notehead of a
            // tied pair, not on both.
            items.push(NotatedItem {
                event: event.id,
                kind: kind_of(event),
                onset_in_measure: MusicalDuration::new(at.as_ratio() - start.as_ratio()),
                duration: piece,
                tie,
                beam: None,
                tuplet: tuplet.map(|tuplet| TupletMark {
                    start: tuplet.start && is_first,
                    stop: tuplet.stop && is_last,
                    ..tuplet
                }),
                slur: Edges {
                    start: is_first && marks.slur_ends.contains_key(&event.id),
                    stop: is_last && marks.slur_stops.contains(&event.id),
                },
                dynamic: if is_first {
                    marks.dynamics.get(&event.id).copied()
                } else {
                    None
                },
                articulations: if is_first {
                    marks.articulations.get(&event.id).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                },
            });
        }
    }
    assign_beams(&mut items, meter, measure_len);
    let slurs = items
        .iter()
        .filter(|item| item.slur.start)
        .filter_map(|item| {
            marks.slur_ends.get(&item.event).map(|to| SlurRange {
                from: item.event,
                to: *to,
            })
        })
        .collect();
    Ok(Lane { items, slurs })
}

/// One lane's plan for one measure, before it is named.
struct Lane {
    items: Vec<NotatedItem>,
    slurs: Vec<SlurRange>,
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
        // Whether a symbol beams is a question about the symbol: a triplet
        // eighth beams because it is written as an eighth. Where it *ends*
        // is a question about time, and inside a tuplet the two differ.
        let symbol = item.duration.value.as_ratio();
        let short = symbol <= eighth;
        let pitched = !matches!(item.kind, NotatedKind::Rest);
        if !short || !pitched {
            continue;
        }
        let sounding = item.tuplet.map_or(symbol, |tuplet| {
            symbol * Ratio::new(i64::from(tuplet.den), i64::from(tuplet.num))
        });
        let onset = item.onset_in_measure.as_ratio();
        let item_end = onset + sounding;
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
        pieces.push(NotatedDuration::single(
            MusicalDuration::new(piece),
            event.notated_duration.spelling.clone(),
        ));
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
