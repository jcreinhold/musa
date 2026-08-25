//! Performance resolver (roadmap §6.4, §15.3;
//! docs/rules/events/06-surface-elaboration.md).
//!
//! The neutral core that integrates the tempo map and resolves a
//! `ScoreSnapshot` into exact, instrument-independent gesture tracks.
//!
//! Tempo is a monotone map written-time → second applied to symbolic positions
//! (§22) — it never rewrites the symbolic track, and "stretch" (an event track
//! time action) is not "tempo" (a performance map). Symbolic stays in beats
//! through this boundary. Frame choice and tuning happen later, in checked
//! audio preparation; the optional `frames` query is only an edge projection.
//!
//! Neutrality is the point (roadmap §2): a written A4 becomes a frequency
//! only through the tuning service, and symbolic dynamics are not
//! velocities. Interpretation enters here and only here: the part's profile
//! (`profile.rs`) turns the written marks into a gate, an amplitude, and an
//! attack request. A part with no profile is resolved exactly as it was
//! before profiles existed — full gate, neutral amplitude.

// Time accumulation uses `MusicalTime`/`MusicalDuration` operators, total
// for musa's magnitudes (see `time.rs`); the workspace arithmetic lint is
// allowed at module scope for that reason.
#![allow(clippy::arithmetic_side_effects)]

use crate::groove::Groove;
use crate::origin::Origin;
use crate::pitch::WrittenPitch;
use crate::profile::{ArticulationRealization, PerformanceProfile};
use crate::score::{EventId, Meter, PartId, ScoreEvent, ScoreEventKind, ScoreSnapshot};
use crate::time::MusicalTime;
use musa_events::{Canonical, Duration, EventTrack, Occurrence, PerformedTime, PhysicalTime, Position, Span, track};
use num_rational::Ratio;
use std::fmt::Write as _;
use std::sync::Arc;

/// Equal temperament with a configurable concert A (roadmap §8.1: the
/// concrete default now, a service boundary later).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tuning {
    /// Frequency of A4 in Hz.
    pub concert_a: f64,
}

impl Default for Tuning {
    fn default() -> Self {
        Self { concert_a: 440.0 }
    }
}

impl Tuning {
    /// The frequency of a written pitch in 12-TET.
    pub fn frequency(&self, pitch: &WrittenPitch) -> f64 {
        let chromatic = f64::from(pitch.letter.natural_semitone()) + f64::from(pitch.accidental.0);
        let midi = 12.0f64.mul_add(f64::from(pitch.octave + 1), chromatic);
        self.concert_a * ((midi - 69.0) / 12.0).exp2()
    }
}

/// Options for performance resolver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerformanceOptions {
    /// Frames per second of the target render.
    pub sample_rate: u32,
    /// The tuning service.
    pub tuning: Tuning,
}

impl Default for PerformanceOptions {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            tuning: Tuning::default(),
        }
    }
}

/// A piecewise-monotone tempo map (docs/rules/events/06-surface-elaboration.md).
///
/// One segment per written tempo, each carrying the exact number of seconds
/// elapsed before it begins, accumulated as a rational: the rounding to whole
/// frames happens once, at the position being asked about, so a tempo change
/// never accumulates the drift that rounding each segment's start would.
#[derive(Clone, Debug, PartialEq)]
pub struct IntegratedTempoMap {
    points: Vec<TempoPoint>,
}

/// One tempo segment: from `position` (whole notes), at `seconds_per_whole`
/// seconds per whole note, `seconds_offset` seconds into the piece — and,
/// when the marking was gradual, on its way to another rate.
#[derive(Clone, Debug, PartialEq)]
struct TempoPoint {
    position: MusicalTime,
    seconds_per_whole: Ratio<i64>,
    ramp: Option<RampRate>,
    seconds_offset: Ratio<i64>,
}

/// A gradual change as the map integrates it: where the rate arrives, how far
/// it takes to get there, and how the change is spread across that reach.
///
/// In **seconds per whole note**, never in beats per minute. Interpolating
/// bpm makes each beat's duration a reciprocal, which leaves the rationals
/// and so leaves §4; interpolating duration keeps every intermediate value
/// exact. It is also what a listener hears as even, which is the rare case of
/// the musically right answer and the representable one being the same.
#[derive(Clone, Debug, PartialEq)]
struct RampRate {
    to: Ratio<i64>,
    over: Ratio<i64>,
    shape: musa_events::Progress,
}

impl TempoPoint {
    /// Seconds elapsed from this segment's start to `whole_notes` into it.
    ///
    /// Total for any distance: past the end of a ramp the rate is simply the
    /// one it arrived at, which is what "the tempo after a *rit.* is the
    /// tempo the *rit.* reached" means arithmetically. A ramp cut short by
    /// the next marking is never asked past the cut, so intersection clamps
    /// it with no rule of its own.
    fn elapsed(&self, whole_notes: Ratio<i64>) -> Ratio<i64> {
        let Some(ramp) = self.ramp.as_ref() else {
            return whole_notes * self.seconds_per_whole;
        };
        let inside = whole_notes.min(ramp.over);
        let after = (whole_notes - ramp.over).max(Ratio::ZERO);
        // A ramp of no reach is the jump it degenerates to.
        let local = if ramp.over == Ratio::ZERO {
            Ratio::ONE
        } else {
            inside / ramp.over
        };
        ramp.over * self.integral(ramp, local) + after * ramp.to
    }

    /// The rate at normalized local time `u` inside a ramp.
    fn rate_at(&self, ramp: &RampRate, u: Ratio<i64>) -> Ratio<i64> {
        self.seconds_per_whole + (ramp.to - self.seconds_per_whole) * ramp.shape.at(u)
    }

    /// `∫₀^u rate(t) dt`, exactly.
    ///
    /// The rate is linear in `t` on each of the shape's pieces — `Progress`
    /// is piecewise-linear and the rate is an affine function of it — so the
    /// trapezoid rule is not an approximation here, it is the integral.
    fn integral(&self, ramp: &RampRate, u: Ratio<i64>) -> Ratio<i64> {
        let mut total = Ratio::ZERO;
        for pair in ramp.shape.points().windows(2) {
            let [(u0, v0), (u1, v1)] = pair else { continue };
            if u <= *u0 {
                break;
            }
            let end = u.min(*u1);
            // `u1 > u0` by construction, so the division is defined.
            let reached = *v0 + (*v1 - *v0) * ((end - *u0) / (*u1 - *u0));
            let (from, to) = (
                self.seconds_per_whole + (ramp.to - self.seconds_per_whole) * *v0,
                self.seconds_per_whole + (ramp.to - self.seconds_per_whole) * reached,
            );
            total += (end - *u0) * (from + to) / Ratio::from_integer(2);
        }
        total
    }
}

/// Seconds per whole note at `bpm` beats of `beat` whole notes each.
///
/// A whole note holds `1/beat` beats, so it lasts `60 / (beat * bpm)`
/// seconds. Both inputs come from the grammar, which cannot write a zero
/// beat unit; a zero bpm is guarded so the map stays total.
fn seconds_per_whole(beat: Ratio<i64>, bpm: u32) -> Ratio<i64> {
    let bpm = i64::from(bpm.max(1));
    let beats_per_whole = beat.recip();
    Ratio::new(60, bpm) * beats_per_whole
}

impl IntegratedTempoMap {
    /// Integrate the piece's tempo *markings* into a written-time → second map.
    ///
    /// This is where the two things called tempo meet and stay apart (course
    /// correction §22). The markings are notation: they have places, they are
    /// printed, and some of them are only words. The map is a function, it is
    /// not in the snapshot, and it is built here — from the markings that
    /// carry a metronome and from no others.
    ///
    /// `tempo "Andante";` therefore contributes a printed word and no
    /// segment, which is the behaviour that proves the split. A piece with no
    /// metronome mark anywhere performs at a quarter = 120: the default lives
    /// here rather than in the snapshot, because it is a fact about playing
    /// an unmarked page, not a fact about the page.
    /// `scope` is polytempo, and it is only an argument: `Tempo` inherits by
    /// `Override` (`scope.rs`), so a part that states its own marking reads
    /// its own and every other scope reads the piece's.
    pub fn new(snapshot: &ScoreSnapshot, scope: crate::Scope) -> Self {
        let mut marks: Vec<(MusicalTime, &crate::score::TempoMarking)> = snapshot
            .tempos()
            .changes(scope)
            .filter(|(_, marking)| marking.metronome.is_some())
            .collect();
        let opening = crate::score::TempoMarking::default();
        // A piece whose first marking is a word alone still has to start
        // somewhere, and so does a piece with no marking at all.
        if marks.first().is_none_or(|(at, _)| *at != MusicalTime::ZERO) {
            marks.insert(0, (MusicalTime::ZERO, &opening));
        }
        let mut points: Vec<TempoPoint> = Vec::new();
        for (at, marking) in marks {
            let mark = marking.metronome.unwrap_or_default();
            let opening_rate = seconds_per_whole(mark.beat, mark.bpm);
            // Where the previous segment has carried the music to by the time
            // this one starts. Changes arrive in playing order, so `last` is
            // always the segment being left.
            let seconds_offset = points.last().map_or(Ratio::ZERO, |previous| {
                previous.seconds_offset + previous.elapsed(at.as_ratio() - previous.position.as_ratio())
            });
            points.push(TempoPoint {
                position: at,
                seconds_per_whole: opening_rate,
                // A ramp with nowhere to arrive is a printed word: it says
                // *rit.* and leaves the speed to the performer, so the map
                // never hears about it.
                ramp: marking.ramp.as_ref().and_then(|ramp| {
                    Some(RampRate {
                        to: seconds_per_whole(mark.beat, ramp.to?),
                        over: ramp.over.as_ratio(),
                        shape: ramp.shape.clone(),
                    })
                }),
                seconds_offset,
            });
        }
        Self { points }
    }

    /// Seconds per quarter note at the start of the piece — what a metrical
    /// MIDI file's tempo meta-event states.
    pub fn seconds_per_quarter(&self) -> f64 {
        self.points
            .first()
            .map_or(0.5, |point| ratio_to_f64(point.seconds_per_whole) / 4.0)
    }

    /// The tempo segments, in playing order: what an exporter needs to write
    /// a tempo change into a file that counts in beats rather than frames.
    ///
    /// A gradual change has no exact form in any of those files, so it is
    /// **sampled** here, into `steps_per_whole` constant segments per whole
    /// note of its reach. The density is the caller's argument rather than a
    /// constant of the map, because the shape is normative and the sampling
    /// is the consumer's policy (docs/rules/events/07) — the same rule a hairpin's
    /// `Progress` is read under, and the reason both are one type.
    pub fn segments(&self, steps_per_whole: u32) -> Vec<TempoSegment> {
        let mut segments = Vec::with_capacity(self.points.len());
        for (index, point) in self.points.iter().enumerate() {
            let next = self.points.get(index.saturating_add(1)).map(|point| point.position);
            let Some(ramp) = point.ramp.as_ref() else {
                segments.push(self.segment_at(point.position, point.seconds_per_whole));
                continue;
            };
            // Cut at the next marking: past it, this segment says nothing.
            let reach = next.map_or(ramp.over, |next| {
                (next.as_ratio() - point.position.as_ratio()).min(ramp.over)
            });
            let steps = (reach * Ratio::from_integer(i64::from(steps_per_whole.max(1))))
                .ceil()
                .to_integer()
                .max(1);
            for step in 0..steps {
                let along = reach * Ratio::new(step, steps);
                let local = if ramp.over == Ratio::ZERO {
                    Ratio::ONE
                } else {
                    along / ramp.over
                };
                segments.push(self.segment_at(
                    MusicalTime::new(point.position.as_ratio() + along),
                    point.rate_at(ramp, local),
                ));
            }
            // The rate the ramp arrived at, stated once where it arrives —
            // unless the next marking got there first and states its own.
            let ends = point.position.as_ratio() + ramp.over;
            if next.is_none_or(|next| ends < next.as_ratio()) {
                segments.push(self.segment_at(MusicalTime::new(ends), ramp.to));
            }
        }
        segments
    }

    /// One exported segment: its exact musical and physical positions and
    /// the rate in force there.
    fn segment_at(&self, position: MusicalTime, seconds_per_whole: Ratio<i64>) -> TempoSegment {
        TempoSegment {
            position,
            physical: self.physical(Position::new(position.as_ratio())),
            seconds_per_quarter: seconds_per_whole / 4,
        }
    }

    /// The absolute frame of a symbolic position (monotone; §22).
    pub fn frames(&self, position: MusicalTime, sample_rate: u32) -> u64 {
        let seconds = self.physical_seconds(position);
        // One rounding, at the end: segment offsets above are exact.
        let frames = seconds * Ratio::from_integer(i64::from(sample_rate.max(1)));
        ratio_to_f64(frames).round().max(0.0) as u64
    }

    /// Exact physical seconds at a position after performance-time warping.
    ///
    /// This is the time-map answer consumed by checked scheduling. It does
    /// not round to a frame and therefore remains independent of the audio
    /// format chosen later.
    pub fn physical(&self, position: Position<PerformedTime>) -> Position<PhysicalTime> {
        Position::new(self.physical_seconds(MusicalTime::new(position.as_ratio())))
    }

    fn physical_seconds(&self, position: MusicalTime) -> Ratio<i64> {
        let Some(point) = self
            .points
            .iter()
            .rev()
            .find(|point| point.position <= position)
            .or_else(|| self.points.first())
        else {
            return Ratio::ZERO;
        };
        point.seconds_offset + point.elapsed(position.as_ratio() - point.position.as_ratio())
    }
}

/// One tempo segment as an exporter sees it: where it starts, in exact
/// symbolic and physical time, and how fast it goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TempoSegment {
    /// The symbolic position the segment starts at.
    pub position: MusicalTime,
    /// Exact physical seconds where the segment starts.
    pub physical: Position<PhysicalTime>,
    /// Exact seconds per quarter note inside the segment.
    pub seconds_per_quarter: Ratio<i64>,
}

/// The one place a musical rational becomes a float.
fn ratio_to_f64(value: Ratio<i64>) -> f64 {
    *value.numer() as f64 / *value.denom() as f64
}

/// One exact instrument-independent instruction in performed time.
///
/// Pitch stays written and expression stays rational here. Tuning and
/// conversion to floating-point DSP parameters belong to audio preparation,
/// after checked scheduling has fixed the occurrence's frame boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gesture {
    /// Stable source-declared identity within the finite performance plan.
    instance: u64,
    /// Written pitch after score transposition, before an instrument tunes it.
    pitch: WrittenPitch,
    /// Exact abstract loudness; one is neutral.
    amplitude: Ratio<i64>,
    /// Exact checked source gesture framing. Empty only on the retained
    /// pre-177 differential oracle used by compatibility tests.
    source_exact: Arc<[u8]>,
}

impl Gesture {
    /// Stable source-declared identity within this finite plan.
    pub const fn instance(&self) -> u64 {
        self.instance
    }

    /// Written pitch retained until tuning.
    pub const fn pitch(&self) -> WrittenPitch {
        self.pitch
    }

    /// Exact abstract expression projection used by compatibility consumers.
    pub const fn amplitude(&self) -> Ratio<i64> {
        self.amplitude
    }

    /// Exact checked source framing of the opaque gesture payload.
    pub fn exact_source_bytes(&self) -> &[u8] {
        &self.source_exact
    }
}

impl Canonical for Gesture {
    const OWNER_TYPE_ID: &'static str = "musa.score.Gesture";
    const QUOTIENT_VERSION: u32 = 2;

    /// Every stored field participates. Constructors and integers have an
    /// explicit encoding; arbitrary source strings are length-framed. This is
    /// deliberately a versioned identity key, never an audio parameter codec.
    fn canonical_key(&self) -> String {
        if !self.source_exact.is_empty() {
            let mut key = format!("source={}:{};", self.source_exact.len(), self.instance);
            for byte in self.source_exact.iter() {
                let _ = write!(key, "{byte:02x}");
            }
            return key;
        }
        let mut key = String::new();
        let _ = write!(
            key,
            "pitch={},{},{};instance={};",
            self.pitch.letter.as_char(),
            self.pitch.accidental.0,
            self.pitch.octave,
            self.instance
        );
        write_ratio(&mut key, "amplitude", self.amplitude);
        key
    }
}

fn write_ratio(key: &mut String, name: &str, value: Ratio<i64>) {
    let _ = write!(key, "{name}={}/{};", value.numer(), value.denom());
}

#[cfg(test)]
mod gesture_identity_laws {
    use musa_events::Canonical as _;
    use num_rational::Ratio;

    use super::Gesture;
    use crate::{Accidental, Letter, WrittenPitch};

    fn gesture(axis: &str) -> Gesture {
        Gesture {
            instance: 0,
            pitch: WrittenPitch {
                letter: Letter::C,
                accidental: Accidental::NATURAL,
                octave: 4,
            },
            amplitude: Ratio::ONE,
            source_exact: format!("axis={axis}").into_bytes().into(),
        }
    }

    #[test]
    fn framed_gesture_identity_does_not_parse_payload_delimiters() {
        let left = gesture("c4|1:source");
        let right = gesture("c4|1:source|");
        let first = left.canonical_key();
        assert_eq!(first, left.canonical_key());
        assert_ne!(left.canonical_key(), right.canonical_key());
    }
}

/// One part's exact gesture track and the map that gives performed positions
/// physical meaning. Parts remain separate because polytempo means they can
/// carry different maps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GestureLineage {
    instance: u64,
    event: EventId,
    written_on: MusicalTime,
    written_off: MusicalTime,
    origin: Origin,
}

impl GestureLineage {
    /// Stable source-declared identity of the gesture this entry describes.
    pub const fn instance(&self) -> u64 {
        self.instance
    }

    /// Written score event realized by the gesture.
    pub const fn event(&self) -> EventId {
        self.event
    }

    /// Written onset before performance-time warping.
    pub const fn written_on(&self) -> MusicalTime {
        self.written_on
    }

    /// Written end before performance-time warping.
    pub const fn written_off(&self) -> MusicalTime {
        self.written_off
    }

    /// Complete source and expansion provenance of the written event.
    pub const fn origin(&self) -> &Origin {
        &self.origin
    }
}

/// Temporary keyed compatibility data retained until prompt 178.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GestureCompatibility {
    instance: u64,
    attack_seconds: Ratio<i64>,
}

impl GestureCompatibility {
    /// Stable gesture identity this temporary projection accompanies.
    pub const fn instance(&self) -> u64 {
        self.instance
    }

    /// Legacy requested attack in exact physical seconds.
    pub const fn attack_seconds(&self) -> Ratio<i64> {
        self.attack_seconds
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GestureLane {
    part: PartId,
    name: String,
    track: EventTrack<PerformedTime, Gesture>,
    lineage: Vec<GestureLineage>,
    compatibility: Vec<GestureCompatibility>,
    tempo: IntegratedTempoMap,
}

impl GestureLane {
    /// The part this lane realizes.
    pub const fn part(&self) -> PartId {
        self.part
    }

    /// The source part name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Exact finite performed gestures.
    pub const fn track(&self) -> &EventTrack<PerformedTime, Gesture> {
        &self.track
    }

    /// Immutable written support and provenance for one gesture identity.
    pub fn lineage(&self, instance: u64) -> Option<&GestureLineage> {
        self.lineage
            .binary_search_by_key(&instance, GestureLineage::instance)
            .ok()
            .and_then(|index| self.lineage.get(index))
    }

    /// Temporary physical-attack projection keyed by gesture identity.
    pub fn compatibility(&self, instance: u64) -> Option<&GestureCompatibility> {
        self.compatibility
            .binary_search_by_key(&instance, GestureCompatibility::instance)
            .ok()
            .and_then(|index| self.compatibility.get(index))
    }

    /// Answer one finite scheduler query in exact physical seconds.
    pub fn physical(&self, position: Position<PerformedTime>) -> Position<PhysicalTime> {
        self.tempo.physical(position)
    }
}

/// Exact performance preparation before frame scheduling or tuning.
#[derive(Clone, Debug, PartialEq)]
pub struct GesturePlan {
    tempo: IntegratedTempoMap,
    meters: Vec<MeterChange>,
    keys: Vec<KeyChange>,
    lanes: Vec<GestureLane>,
    polytempo: bool,
}

impl GesturePlan {
    /// The piece-scoped exact tempo map used by single-tempo edge formats.
    pub fn tempo(&self) -> &IntegratedTempoMap {
        &self.tempo
    }

    /// Whether at least one part carries a tempo map distinct from the piece.
    pub const fn is_polytempo(&self) -> bool {
        self.polytempo
    }

    /// Piece-scoped meter changes at exact written positions.
    pub fn meters(&self) -> &[MeterChange] {
        &self.meters
    }

    /// Piece-scoped key changes at exact written positions.
    pub fn keys(&self) -> &[KeyChange] {
        &self.keys
    }

    /// One exact lane per part, in source order.
    pub fn lanes(&self) -> &[GestureLane] {
        &self.lanes
    }
}

/// A time signature and the exact written position where it takes effect.
///
/// Meter is notation, and a performance does not hear it — which is exactly
/// why it is carried here rather than derived: MIDI is an *edge* format that
/// writes a time-signature meta event, and the only alternative would be for
/// the exporter to hold a second score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeterChange {
    /// Where it starts in written time.
    pub at: MusicalTime,
    /// What it is.
    pub meter: crate::score::Meter,
}

/// A key signature and the exact written position where it takes effect.
///
/// Carried for the same reason as [`MeterChange`]: SMF writes a key-signature
/// meta event, and the exporter must not have to hold a second score to know
/// when to write one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyChange {
    /// Where it starts in written time.
    pub at: MusicalTime,
    /// What it is.
    pub key: crate::score::Key,
}

/// A failure to lower a score for performance. Reserved: the current
/// grammar lowers everything; the type exists so the facade is stable.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PerformanceError {
    /// A construct the performance layer cannot schedule.
    #[error("unsupported performance construct: {0}")]
    Unsupported(String),
}

/// One complete notation value presented to `std::performance.interpret`.
///
/// This is a lossless host projection of facts the score already computed;
/// it contains no interpreted gate, loudness, technique, or control. The
/// compiler serializes it as an ordinary `NotationView` and source owns the
/// answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerformanceView {
    /// Stable source gesture identity within this finite preparation.
    instance: u64,
    /// Written pitch before tuning.
    pitch: WrittenPitch,
    /// Prevailing written dynamic, when any.
    dynamic: Option<crate::score::DynamicMark>,
    /// Written note marks in source order.
    marks: Vec<crate::Mark>,
    /// Hairpin target and exact reached progress at this event.
    hairpin: Option<(crate::score::DynamicMark, Ratio<i64>)>,
}

impl PerformanceView {
    /// Stable identity assigned to the requested source gesture.
    pub const fn instance(&self) -> u64 {
        self.instance
    }

    /// Written pitch before tuning.
    pub const fn pitch(&self) -> WrittenPitch {
        self.pitch
    }

    /// Prevailing written dynamic, when one has appeared.
    pub const fn dynamic(&self) -> Option<crate::score::DynamicMark> {
        self.dynamic
    }

    /// Written note marks in source order.
    pub fn marks(&self) -> &[crate::Mark] {
        &self.marks
    }

    /// Hairpin target and reached local progress, when under a hairpin.
    pub const fn hairpin(&self) -> Option<(crate::score::DynamicMark, Ratio<i64>)> {
        self.hairpin
    }
}

/// One part's finite source-performance request batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerformanceRequests {
    /// Part whose occurrences receive the answers.
    part: PartId,
    /// Parsed declarations only; interpretation remains source-owned.
    profile: Option<PerformanceProfile>,
    /// Views in the exact order gesture lowering consumes them.
    views: Vec<PerformanceView>,
}

impl PerformanceRequests {
    /// Part whose occurrences receive this batch's answers.
    pub const fn part(&self) -> PartId {
        self.part
    }

    /// Parsed declarations presented to source without host interpretation.
    pub const fn profile(&self) -> Option<&PerformanceProfile> {
        self.profile.as_ref()
    }

    /// Notation views in the exact order assigned stable identities.
    pub fn views(&self) -> &[PerformanceView] {
        &self.views
    }
}

/// Project every notation input the source performance policy receives.
///
/// The order is grace gestures first and then the principal's written
/// pitches, matching the mechanical track bridge. A changed order is caught
/// by the result-count and gesture-id checks at that boundary.
#[must_use]
pub fn performance_requests(score: &ScoreSnapshot) -> Vec<PerformanceRequests> {
    let marks = Interpretation::collect(score);
    let curves = hairpin_curves(score);
    let graces = grace_index(score);
    let mut next_instance = 0_u64;
    let mut batches = Vec::new();
    for (part_id, part) in score.parts().iter() {
        let mut views = Vec::new();
        for (_, voice) in part.voices() {
            let mut dynamic = None;
            for event in voice.events() {
                if let Some(mark) = marks.dynamics.get(&event.id) {
                    dynamic = Some(*mark);
                }
                let hairpin = curves.get(&event.id).map(|curve| (curve.target, curve.fraction));
                for grace in graces.get(&event.id).map_or(&[][..], Vec::as_slice) {
                    views.push(PerformanceView {
                        instance: next_instance,
                        pitch: grace.pitch,
                        dynamic,
                        marks: grace.articulations.clone(),
                        hairpin,
                    });
                    next_instance = next_instance.saturating_add(1);
                }
                let pitches: &[WrittenPitch] = match &event.kind {
                    ScoreEventKind::Note { pitch } => std::slice::from_ref(pitch),
                    ScoreEventKind::Chord { pitches } => pitches,
                    ScoreEventKind::Rest => &[],
                };
                for pitch in pitches {
                    views.push(PerformanceView {
                        instance: next_instance,
                        pitch: *pitch,
                        dynamic,
                        marks: marks.articulations_of(event.id).to_vec(),
                        hairpin,
                    });
                    next_instance = next_instance.saturating_add(1);
                }
                // Reaching the end of a written hairpin makes its target the
                // prevailing notation fact for following events. This is a
                // projection of the score's written state, not a host choice
                // about the target's expression value; source policy still
                // supplies that value.
                if let Some(curve) = curves.get(&event.id)
                    && curve.fraction == Ratio::ONE
                {
                    dynamic = Some(curve.target);
                }
            }
        }
        batches.push(PerformanceRequests {
            part: part_id,
            profile: score.profiles().for_part(part.name()).cloned(),
            views,
        });
    }
    batches
}

#[derive(Clone, Debug)]
struct SourceReading {
    instance: u64,
    pitch: WrittenPitch,
    expression: Ratio<i64>,
    gate: Ratio<i64>,
    attack: Ratio<i64>,
    hold: Ratio<i64>,
    exact_gesture: Arc<[u8]>,
}

#[derive(Clone, Debug)]
struct PreparedGesture {
    instance: u64,
    pitch: WrittenPitch,
    expression: Ratio<i64>,
    gate: Ratio<i64>,
    attack: Ratio<i64>,
    exact_gesture: Arc<[u8]>,
}

impl SourceReading {
    fn prepare(
        self,
        expected_instance: u64,
        expected_pitch: WrittenPitch,
    ) -> Result<PreparedGesture, PerformanceError> {
        if self.instance != expected_instance || self.pitch != expected_pitch {
            return Err(source_shape("the requested gesture identity and written pitch"));
        }
        Ok(PreparedGesture {
            instance: self.instance,
            pitch: self.pitch,
            expression: self.expression,
            gate: self.gate * self.hold,
            attack: self.attack,
            exact_gesture: self.exact_gesture,
        })
    }
}

fn source_readings(artifacts: &[musa_calculus::CheckedSource]) -> Result<Vec<Vec<SourceReading>>, PerformanceError> {
    artifacts.iter().map(source_lane).collect()
}

fn source_lane(artifact: &musa_calculus::CheckedSource) -> Result<Vec<SourceReading>, PerformanceError> {
    if artifact.schema().name() != "std.performance.PerformanceInterpretationArtifact"
        || artifact.schema().version() != 1
        || !artifact.has_valid_framing()
    {
        return Err(PerformanceError::Unsupported(
            "the performance bridge received the wrong checked-source schema".to_owned(),
        ));
    }
    let [_, results] = source_fields::<2>(artifact.root(), "PerformanceInterpretationArtifact")?;
    source_list(results, |result| {
        let [gesture, gate, attack, hold] = source_fields::<4>(result, "ProfileResult")?;
        let [instance, pitch, controls, _] = source_fields::<4>(gesture, "NoteGesture")?;
        let [instance] = source_fields::<1>(instance, "GestureId")?;
        Ok(SourceReading {
            instance: source_nat(instance)?,
            pitch: source_pitch(pitch)?,
            expression: source_expression(controls)?,
            gate: source_ratio(gate)?,
            attack: source_ratio(attack)?,
            hold: source_ratio(hold)?,
            exact_gesture: gesture.exact_bytes().into(),
        })
    })
}

fn source_expression(controls: musa_calculus::SourceDatum<'_>) -> Result<Ratio<i64>, PerformanceError> {
    let controls = source_list(controls, |control| {
        let [_, key, value] = source_fields::<3>(control, "Control")?;
        let [_, namespace, name, _, _] = source_fields::<5>(key, "Key")?;
        if source_text(namespace)? == "std.performance" && source_text(name)? == "expression" {
            let [value] = source_fields::<1>(value, "NormalizedValue")?;
            Ok(Some(source_ratio(value)?))
        } else {
            Ok(None)
        }
    })?;
    let mut expression = controls.into_iter().flatten();
    let value = expression
        .next()
        .ok_or_else(|| source_shape("one standard expression control"))?;
    if expression.next().is_some() {
        return Err(source_shape("one standard expression control"));
    }
    Ok(value)
}

fn source_fields<'a, const COUNT: usize>(
    datum: musa_calculus::SourceDatum<'a>,
    expected: &str,
) -> Result<[musa_calculus::SourceDatum<'a>; COUNT], PerformanceError> {
    let Some(musa_calculus::SourceDatumKind::Case { constructor }) = datum.kind() else {
        return Err(source_shape(expected));
    };
    if constructor.rsplit('.').next() != Some(expected) {
        return Err(source_shape(expected));
    }
    let fields = datum
        .fields()
        .ok_or_else(|| source_shape(expected))?
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| source_shape(expected))?
        .try_into()
        .map_err(|_| source_shape(expected))?;
    Ok(fields)
}

fn source_list<T>(
    mut datum: musa_calculus::SourceDatum<'_>,
    mut read: impl FnMut(musa_calculus::SourceDatum<'_>) -> Result<T, PerformanceError>,
) -> Result<Vec<T>, PerformanceError> {
    let mut values = Vec::new();
    loop {
        let Some(musa_calculus::SourceDatumKind::Case { constructor }) = datum.kind() else {
            return Err(source_shape("List"));
        };
        match constructor.rsplit('.').next() {
            Some("Empty") => return Ok(values),
            Some("Cons") => {
                let [head, tail] = source_fields::<2>(datum, "Cons")?;
                values.push(read(head)?);
                datum = tail;
            }
            _ => return Err(source_shape("List")),
        }
    }
}

fn source_nat(datum: musa_calculus::SourceDatum<'_>) -> Result<u64, PerformanceError> {
    match datum.kind() {
        Some(musa_calculus::SourceDatumKind::Count { family, count }) if family.ends_with("Nat") => Ok(count),
        _ => Err(source_shape("Nat")),
    }
}

fn source_ratio(datum: musa_calculus::SourceDatum<'_>) -> Result<Ratio<i64>, PerformanceError> {
    let Some(musa_calculus::SourceDatumKind::Literal { type_name, bytes }) = datum.kind() else {
        return Err(source_shape("Ratio"));
    };
    if !type_name.ends_with("Ratio") || bytes.len() != 16 {
        return Err(source_shape("Ratio"));
    }
    let (numerator, denominator) = bytes.split_at_checked(8).ok_or_else(|| source_shape("Ratio"))?;
    let numerator = i64::from_be_bytes(numerator.try_into().map_err(|_| source_shape("Ratio"))?);
    let denominator = i64::from_be_bytes(denominator.try_into().map_err(|_| source_shape("Ratio"))?);
    if denominator == 0 {
        return Err(source_shape("Ratio"));
    }
    Ok(Ratio::new(numerator, denominator))
}

fn source_text(datum: musa_calculus::SourceDatum<'_>) -> Result<&str, PerformanceError> {
    let Some(musa_calculus::SourceDatumKind::Literal { type_name, bytes }) = datum.kind() else {
        return Err(source_shape("Text"));
    };
    if !type_name.ends_with("Text") {
        return Err(source_shape("Text"));
    }
    std::str::from_utf8(bytes).map_err(|_| source_shape("Text"))
}

fn source_pitch(datum: musa_calculus::SourceDatum<'_>) -> Result<WrittenPitch, PerformanceError> {
    let Some(musa_calculus::SourceDatumKind::Literal { type_name, bytes }) = datum.kind() else {
        return Err(source_shape("Pitch"));
    };
    let [
        letter,
        accidental_a,
        accidental_b,
        accidental_c,
        accidental_d,
        octave_a,
        octave_b,
        octave_c,
        octave_d,
    ] = bytes
    else {
        return Err(source_shape("Pitch"));
    };
    if !type_name.ends_with("Pitch") {
        return Err(source_shape("Pitch"));
    }
    Ok(WrittenPitch {
        letter: crate::Letter::from_steps(i8::try_from(*letter).map_err(|_| source_shape("Pitch"))?)
            .ok_or_else(|| source_shape("Pitch"))?,
        accidental: crate::Accidental(i32::from_be_bytes([
            *accidental_a,
            *accidental_b,
            *accidental_c,
            *accidental_d,
        ])),
        octave: i32::from_be_bytes([*octave_a, *octave_b, *octave_c, *octave_d]),
    })
}

fn source_shape(expected: &str) -> PerformanceError {
    PerformanceError::Unsupported(format!(
        "the checked source performance result is not a `{expected}` in schema version 1"
    ))
}

/// Lower a score into exact, instrument-independent performed gesture tracks.
///
/// This retained pre-177 oracle fixes groove, grace, articulation, and dynamic
/// interpretation in Rust for differential tests. Production compilation uses
/// checked `std::performance` results through [`lower_gestures_from_checked`].
/// No frame, frequency, or floating-point
/// audio value is chosen here.
///
/// # Errors
/// [`PerformanceError`] if the resulting finite performed track violates its
/// exact bounds.
pub fn lower_gestures(score: &ScoreSnapshot) -> Result<GesturePlan, PerformanceError> {
    lower_gestures_with_source(score, None)
}

fn lower_gestures_with_source(
    score: &ScoreSnapshot,
    readings: Option<Vec<Vec<SourceReading>>>,
) -> Result<GesturePlan, PerformanceError> {
    let reference = IntegratedTempoMap::new(score, crate::Scope::Piece);
    let marks = Interpretation::collect(score);
    let curves = hairpin_curves(score);
    let graces = grace_index(score);
    let mut lanes = Vec::new();
    let mut polytempo = false;
    let mut next_instance = 0_u64;
    if readings
        .as_ref()
        .is_some_and(|lanes| lanes.len() != score.parts().len())
    {
        return Err(source_shape("one interpretation artifact per gesture lane"));
    }
    let mut source_lanes = readings
        .map(|lanes| {
            lanes
                .into_iter()
                .map(|lane| {
                    let expected = lane.len();
                    let by_instance: std::collections::HashMap<_, _> =
                        lane.into_iter().map(|reading| (reading.instance, reading)).collect();
                    if by_instance.len() == expected {
                        Ok(by_instance)
                    } else {
                        Err(source_shape("one result per distinct gesture identity"))
                    }
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    for (lane_index, (id, part)) in score.parts().iter().enumerate() {
        let mut source = match source_lanes.as_mut() {
            Some(lanes) => Some(
                lanes
                    .get_mut(lane_index)
                    .ok_or_else(|| source_shape("one interpretation artifact per gesture lane"))?,
            ),
            None => None,
        };
        let profile = score.profiles().for_part(part.name());
        let scope = crate::Scope::Part { part: id.0 };
        let tempo = IntegratedTempoMap::new(score, scope);
        polytempo |= tempo != reference;
        let clock = Clock {
            meters: score.meters(),
            scope,
            groove: profile.map_or(Groove::STRAIGHT, PerformanceProfile::groove),
        };
        let policy = profile.map_or(crate::GracePolicy::DEFAULT, PerformanceProfile::grace);
        let mut pending = Vec::new();
        for (_, voice) in part.voices() {
            let mut dynamic = None;
            let mut curve_from: Option<Ratio<i64>> = None;
            let mut floor = MusicalTime::ZERO;
            let mut previous = 0..0;
            for event in voice.events() {
                if let Some(mark) = marks.dynamics.get(&event.id) {
                    dynamic = Some(*mark);
                }
                let legacy = if source.is_none() {
                    let realization = profile.map_or(ArticulationRealization::NEUTRAL, |profile| {
                        profile.realize(marks.articulations_of(event.id))
                    });
                    let level = dynamic
                        .zip(profile)
                        .and_then(|(mark, profile)| profile.amplitude(mark))
                        .unwrap_or(Ratio::ONE);
                    let amplitude = match curves.get(&event.id) {
                        None => {
                            curve_from = None;
                            level
                        }
                        Some(curve) => {
                            let from = *curve_from.get_or_insert(level);
                            let to = profile
                                .and_then(|profile| profile.amplitude(curve.target))
                                .unwrap_or(Ratio::ONE);
                            let reached = from + (to - from) * curve.fraction;
                            if curve.fraction == Ratio::ONE {
                                dynamic = Some(curve.target);
                                curve_from = None;
                            }
                            reached
                        }
                    };
                    Some(Interpreted {
                        gate: realization.gate * realization.hold,
                        attack: realization.attack,
                        amplitude,
                    })
                } else {
                    None
                };
                let mut leaning = Vec::new();
                for grace in graces.get(&event.id).map_or(&[][..], Vec::as_slice) {
                    let prepared = take_prepared(source.as_deref_mut(), next_instance, grace.pitch, || {
                        let realized = profile.map_or(ArticulationRealization::NEUTRAL, |profile| {
                            profile.realize(&grace.articulations)
                        });
                        Interpreted {
                            gate: realized.gate * realized.hold,
                            attack: legacy.as_ref().map_or(Ratio::ZERO, |value| value.attack),
                            amplitude: legacy.as_ref().map_or(Ratio::ONE, |value| value.amplitude),
                        }
                    })?;
                    next_instance = next_instance.saturating_add(1);
                    leaning.push(prepared);
                }
                let pitches: &[WrittenPitch] = match &event.kind {
                    ScoreEventKind::Note { pitch } => std::slice::from_ref(pitch),
                    ScoreEventKind::Chord { pitches } => pitches,
                    ScoreEventKind::Rest => &[],
                };
                let mut principals = Vec::with_capacity(pitches.len());
                for pitch in pitches {
                    let prepared = take_prepared(source.as_deref_mut(), next_instance, *pitch, || {
                        legacy.as_ref().map_or(
                            Interpreted {
                                gate: Ratio::ONE,
                                attack: Ratio::ZERO,
                                amplitude: Ratio::ONE,
                            },
                            |value| Interpreted {
                                gate: value.gate,
                                attack: value.attack,
                                amplitude: value.amplitude,
                            },
                        )
                    })?;
                    next_instance = next_instance.saturating_add(1);
                    principals.push(prepared);
                }
                let lowered = lower_gesture_event(&clock, event, &leaning, &principals, policy, floor, &mut pending);
                if let Some(at) = lowered.anticipated {
                    let performed = clock.performed(at);
                    for index in previous.clone() {
                        if let Some(gesture) = pending.get_mut(index) {
                            gesture.end = gesture.end.min(performed);
                        }
                    }
                }
                if !matches!(event.kind, ScoreEventKind::Rest) {
                    floor =
                        event.onset + crate::time::MusicalDuration::new(event.notated_duration.value.as_ratio() / 2);
                }
                previous = lowered.pushed;
            }
        }
        if source.as_ref().is_some_and(|remaining| !remaining.is_empty()) {
            return Err(source_shape("one consumed result for every requested gesture"));
        }
        let ambient = clock.performed(MusicalTime::ZERO + part.span());
        let end = pending
            .iter()
            .map(|gesture| gesture.end)
            .max()
            .unwrap_or(ambient)
            .max(ambient);
        let duration =
            Duration::new(end.as_ratio()).map_err(|error| PerformanceError::Unsupported(error.to_string()))?;
        let mut lineage = Vec::with_capacity(pending.len());
        let mut compatibility = Vec::with_capacity(pending.len());
        let occurrences = pending
            .into_iter()
            .map(|gesture| {
                lineage.push(gesture.lineage);
                compatibility.push(gesture.compatibility);
                Span::new(gesture.start, gesture.end)
                    .map(|span| Occurrence::new(span, gesture.payload))
                    .map_err(|error| PerformanceError::Unsupported(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        lineage.sort_by_key(GestureLineage::instance);
        compatibility.sort_by_key(GestureCompatibility::instance);
        let track = track(duration, occurrences).map_err(|error| PerformanceError::Unsupported(error.to_string()))?;
        lanes.push(GestureLane {
            part: part.id(),
            name: part.name().to_owned(),
            track,
            lineage,
            compatibility,
            tempo,
        });
    }
    let meters = score
        .meters()
        .changes(crate::Scope::Piece)
        .map(|(at, meter)| MeterChange { at, meter: *meter })
        .collect();
    let keys = score
        .keys()
        .changes(crate::Scope::Piece)
        .map(|(at, key)| KeyChange { at, key: *key })
        .collect();
    Ok(GesturePlan {
        tempo: reference,
        meters,
        keys,
        lanes,
        polytempo,
    })
}

/// Build the performed track from checked `std::performance` results.
///
/// The retained lowering supplies coordinate conversion, grace placement, and
/// provenance. Every projected musical field comes from the checked source
/// answer; the old Rust interpretation is not evaluated on this path.
///
/// # Errors
///
/// Returns [`PerformanceError`] for a wrong/malformed artifact, a request/result
/// mismatch, or an invalid exact track.
pub fn lower_gestures_from_checked(
    score: &ScoreSnapshot,
    artifacts: &[musa_calculus::CheckedSource],
) -> Result<GesturePlan, PerformanceError> {
    let readings = source_readings(artifacts)?;
    lower_gestures_with_source(score, Some(readings))
}

fn take_prepared(
    source: Option<&mut std::collections::HashMap<u64, SourceReading>>,
    instance: u64,
    pitch: WrittenPitch,
    legacy: impl FnOnce() -> Interpreted,
) -> Result<PreparedGesture, PerformanceError> {
    if let Some(source) = source {
        return source
            .remove(&instance)
            .ok_or_else(|| source_shape("a result for every requested gesture"))?
            .prepare(instance, pitch);
    }
    let legacy = legacy();
    Ok(PreparedGesture {
        instance,
        pitch,
        expression: legacy.amplitude,
        gate: legacy.gate,
        attack: legacy.attack,
        exact_gesture: Arc::from([]),
    })
}

/// Written time to exact performed time, for one part.
///
/// The composition order is the whole point (docs/rules/events/06-surface-elaboration.md): the
/// groove is a written-time → written-time warp and tempo is written-time → second, so the groove
/// goes **first**. Composed the other way a shuffle would be specified in
/// seconds and would straighten out as the band sped up.
///
/// It is one operation rather than two exposed ones because a caller that
/// reached for `tempo.frames` directly would silently drop the groove, and
/// nothing in the output would say so.
struct Clock<'a> {
    meters: &'a crate::ContextTrack<Meter>,
    /// The part this clock is for: its meter names the groove's cell.
    scope: crate::Scope,
    groove: Groove,
}

impl Clock<'_> {
    /// The exact performed position after this part's groove warp.
    fn performed(&self, at: MusicalTime) -> Position<PerformedTime> {
        let meter = self.meters.at(self.scope, at).copied().unwrap_or_default();
        Position::new(self.groove.warp(meter, at).as_ratio())
    }
}

/// What the part's profile makes of one event, resolved once per event.
struct Interpreted {
    /// Fraction of the written value that actually sounds, with the profile's
    /// hold already folded in: both are multipliers on the written value and
    /// nothing downstream could tell them apart.
    gate: Ratio<i64>,
    /// Requested attack in seconds.
    attack: Ratio<i64>,
    /// Loudness in `0..=1`.
    amplitude: Ratio<i64>,
}

struct PendingGesture {
    start: Position<PerformedTime>,
    end: Position<PerformedTime>,
    payload: Gesture,
    lineage: GestureLineage,
    compatibility: GestureCompatibility,
}

/// Lower one event with the shared grace-stealing and interpretation rules,
/// stopping at performed positions before frames and tuning are chosen.
#[allow(clippy::too_many_arguments)]
fn lower_gesture_event(
    clock: &Clock<'_>,
    event: &ScoreEvent,
    leaning: &[PreparedGesture],
    principals: &[PreparedGesture],
    policy: crate::GracePolicy,
    floor: MusicalTime,
    gestures: &mut Vec<PendingGesture>,
) -> Lowered {
    let written = event.notated_duration.value;
    let notated_end = event.onset + written;
    let stolen = steal(event, written, leaning.len(), policy, floor);
    let mut at = stolen.graces_start_at;
    for prepared in leaning {
        let sounds = crate::time::MusicalDuration::new(stolen.each.as_ratio() * prepared.gate);
        gestures.push(PendingGesture {
            start: clock.performed(at),
            end: clock.performed(at + sounds),
            payload: Gesture {
                instance: prepared.instance,
                pitch: prepared.pitch,
                amplitude: prepared.expression,
                source_exact: Arc::clone(&prepared.exact_gesture),
            },
            lineage: GestureLineage {
                instance: prepared.instance,
                event: event.id,
                written_on: event.onset,
                written_off: notated_end,
                origin: event.origin.clone(),
            },
            compatibility: GestureCompatibility {
                instance: prepared.instance,
                attack_seconds: prepared.attack,
            },
        });
        at = at + stolen.each;
    }
    let sounded_start = stolen.principal_starts_at;
    let first = gestures.len();
    for prepared in principals {
        let sounded_end =
            sounded_start + crate::time::MusicalDuration::new(stolen.principal_sounds.as_ratio() * prepared.gate);
        gestures.push(PendingGesture {
            start: clock.performed(sounded_start),
            end: clock.performed(sounded_end),
            payload: Gesture {
                instance: prepared.instance,
                pitch: prepared.pitch,
                amplitude: prepared.expression,
                source_exact: Arc::clone(&prepared.exact_gesture),
            },
            lineage: GestureLineage {
                instance: prepared.instance,
                event: event.id,
                written_on: event.onset,
                written_off: notated_end,
                origin: event.origin.clone(),
            },
            compatibility: GestureCompatibility {
                instance: prepared.instance,
                attack_seconds: prepared.attack,
            },
        });
    }
    Lowered {
        pushed: first..gestures.len(),
        anticipated: stolen.anticipated,
    }
}

/// The score's marks indexed by the event they belong to, so interpretation
/// is a lookup per event rather than a scan per event.
struct Interpretation {
    dynamics: std::collections::HashMap<EventId, crate::score::DynamicMark>,
    articulations: std::collections::HashMap<EventId, Vec<crate::Mark>>,
}

impl Interpretation {
    fn collect(score: &ScoreSnapshot) -> Self {
        let mut dynamics = std::collections::HashMap::new();
        for marking in score.annotations().dynamics() {
            dynamics.insert(marking.at, marking.mark);
        }
        let mut articulations: std::collections::HashMap<_, Vec<_>> = std::collections::HashMap::new();
        for marking in score.annotations().articulations() {
            articulations.entry(marking.at).or_default().push(marking.mark);
        }
        Self {
            dynamics,
            articulations,
        }
    }

    fn articulations_of(&self, event: EventId) -> &[crate::Mark] {
        self.articulations.get(&event).map_or(&[], Vec::as_slice)
    }
}

/// How far one event is along the hairpin it falls under, and the mark that
/// hairpin arrives at.
///
/// `fraction` is the *shape's* value, not the sampling position: the policy
/// below picks `u`, the event track's `Progress` says what fraction of the distance
/// `u` has covered, and this is that answer. A hairpin's last event has
/// `fraction == 1` and therefore leaves the target in force.
#[derive(Clone, Copy, Debug)]
struct Reached {
    target: crate::score::DynamicMark,
    fraction: Ratio<i64>,
}

/// Index a voice's events by the hairpin they fall under, sampling each
/// hairpin's shape once per event.
///
/// **Shape versus sampling policy** (docs/rules/events/07). The shape — how the
/// growth is distributed across the region — is a fact about the piece: it
/// lives in the track as a `Progress`, it serializes, and every conforming
/// consumer must honour it. *Where to sample it* is this layer's choice, and
/// this layer chooses **once per notated event, at `u = index / (count − 1)`**.
///
/// That choice is deliberate: a hairpin is written around notes, so each event
/// under it takes an equal share of the distance and the last one arrives
/// exactly at the written mark. Sampling by time instead — `u = (onset −
/// start) / width` — would make the arrival depend on the rhythm, which is not
/// what the sign says. A consumer that prefers time-sampling is conforming;
/// it will simply sound different between the endpoints, and identical at
/// them.
fn hairpin_curves(score: &ScoreSnapshot) -> std::collections::HashMap<EventId, Reached> {
    let mut curves = std::collections::HashMap::new();
    for hairpin in score.annotations().hairpins() {
        let events = score.events_in(hairpin.from, hairpin.to);
        let Some(last) = events.len().checked_sub(1) else {
            continue;
        };
        for (step, event) in events.iter().enumerate() {
            // A lone event under a hairpin is already at the far end.
            let u = match (i64::try_from(step), i64::try_from(last)) {
                (Ok(step), Ok(last)) if last > 0 => Ratio::new(step, last),
                _ => Ratio::ONE,
            };
            curves.insert(
                event.id,
                Reached {
                    target: hairpin.target,
                    fraction: hairpin.shape.at(u),
                },
            );
        }
    }
    curves
}

/// Index the grace notes by the event they lean on, in written order.
///
/// The snapshot stores them in one flat lane keyed by principal, so this is
/// the same bulk-index-once shape as [`Interpretation::collect`]: one pass
/// here rather than a scan of every grace per event.
///
/// The sort is by `index`, the ordering the payload carries (docs/rules/events/05
/// N2). Normalization sorts occurrences by span then payload key, and every
/// grace in a group shares a span — so `grace { c5 d5 }` and
/// `grace { d5 c5 }` are told apart by nothing else. Reading the lane's
/// order instead would make the sound depend on projection order, which is
/// exactly the thing `index` exists to stop.
fn grace_index(score: &ScoreSnapshot) -> std::collections::HashMap<EventId, Vec<&crate::score::GraceNote>> {
    let mut index: std::collections::HashMap<_, Vec<&crate::score::GraceNote>> = std::collections::HashMap::new();
    for grace in score.annotations().graces() {
        index.entry(grace.at).or_default().push(grace);
    }
    for group in index.values_mut() {
        group.sort_by_key(|grace| grace.index);
    }
    index
}

/// What the voice loop needs to know about an event it has just lowered.
struct Lowered {
    /// Where in `events` the *principal's* on/off pairs went, so the next
    /// event's graces can shorten them. The graces this event pushed are
    /// deliberately outside the range: a grace is not something a later grace
    /// takes time from.
    pushed: std::ops::Range<usize>,
    /// When this event's graces began, if they were taken from the note
    /// before. `None` when they were taken from the principal, which is the
    /// case that costs the previous note nothing.
    anticipated: Option<MusicalTime>,
}

/// How a grace group's time is paid for.
///
/// Every field is exact musical time, decided before a single frame is
/// computed: the steal is a fact about the written values, and rounding it
/// through the clock first would make the principal's start depend on the
/// tempo.
struct Stolen {
    /// Where the first grace note sounds.
    graces_start_at: MusicalTime,
    /// The slot each grace note gets. Equal shares: the profile states one
    /// duration, and a group of three is three of them.
    each: crate::time::MusicalDuration,
    /// Where the principal actually sounds.
    principal_starts_at: MusicalTime,
    /// How much of the written value the principal has left.
    principal_sounds: crate::time::MusicalDuration,
    /// Set when the time came from the previous note (see [`Lowered`]).
    anticipated: Option<MusicalTime>,
}

/// Decide who pays for a grace group, and how much.
///
/// Two rules bound the answer, and both exist so that a profile cannot write
/// a performance that contradicts the page:
///
/// - Graces taken from the **principal** may take at most half its written
///   value. A profile asking for more gets equal shares of that half rather
///   than a note that starts after it ends.
/// - Graces taken from the **previous** note may reach back at most to
///   `floor` — the caller's bound, which is the midpoint of the note before.
///   Asking for more is read as taking from the principal instead, because a
///   grace that swallows the whole preceding note is not what the sign means.
///   This is also what happens at the start of a voice, where there is
///   nothing behind the beat to take from.
fn steal(
    event: &ScoreEvent,
    written: crate::time::MusicalDuration,
    count: usize,
    policy: crate::GracePolicy,
    floor: MusicalTime,
) -> Stolen {
    let plain = Stolen {
        graces_start_at: event.onset,
        each: crate::time::MusicalDuration::ZERO,
        principal_starts_at: event.onset,
        principal_sounds: written,
        anticipated: None,
    };
    let Ok(count) = i64::try_from(count) else {
        return plain;
    };
    if count == 0 || policy.steal <= Ratio::ZERO {
        return plain;
    }
    let asked = policy.steal * count;
    if policy.from == crate::StealFrom::Previous {
        // The comparison is on the ratio, not on a `MusicalTime`: the
        // constructor clamps a negative time to zero, so a grace reaching back
        // past the start of the piece would arrive here looking legal and
        // sound on top of the note it is supposed to precede.
        let back = event.onset.as_ratio() - asked;
        if back >= floor.as_ratio() {
            return Stolen {
                graces_start_at: MusicalTime::new(back),
                each: crate::time::MusicalDuration::new(policy.steal),
                anticipated: Some(MusicalTime::new(back)),
                ..plain
            };
        }
    }
    let room = written.as_ratio() / 2;
    let each = if asked > room { room / count } else { policy.steal };
    let total = crate::time::MusicalDuration::new(each * count);
    Stolen {
        graces_start_at: event.onset,
        each: crate::time::MusicalDuration::new(each),
        principal_starts_at: event.onset + total,
        principal_sounds: crate::time::MusicalDuration::new(written.as_ratio() - total.as_ratio()),
        anticipated: None,
    }
}

#[cfg(test)]
// The shapes here are written out literally, so a `None` is a typo in the test
// rather than a condition the test could meaningfully handle.
#[expect(
    clippy::expect_used,
    reason = "statically valid inputs; a failure is a bug in the test"
)]
mod ramp_shape_laws {
    use num_rational::Ratio;

    use super::{RampRate, TempoPoint};
    use crate::time::MusicalTime;

    fn r(numerator: i64, denominator: i64) -> Ratio<i64> {
        Ratio::new(numerator, denominator)
    }

    /// A ramp whose shape is not a straight line, which is the case the
    /// grammar cannot yet write and the integration is nonetheless written
    /// for.
    ///
    /// Four whole notes from four seconds each to eight, but with the change
    /// held back: a quarter of the way there at the halfway point, the rest
    /// crowded into the second half. That is a *rit.* that seems to give way
    /// suddenly, and a musician would hear the difference immediately.
    ///
    /// The arithmetic, by hand: the rate runs 4 → 5 over the first half and
    /// 5 → 8 over the second, so the integral in local time is
    /// `½·(4+5)/2 + ½·(5+8)/2 = 9/4 + 13/4 = 11/2`, and the ramp lasts
    /// `4 · 11/2 = 22` seconds — against 24 for the straight line. Exact, and
    /// different, which is the whole claim: the shape is normative, and a
    /// consumer that ignored it would be two seconds out.
    #[test]
    fn a_shaped_ramp_integrates_along_its_shape() {
        let shape = musa_events::Progress::piecewise([(r(0, 1), r(0, 1)), (r(1, 2), r(1, 4)), (r(1, 1), r(1, 1))])
            .expect("a curve");
        let point = TempoPoint {
            position: MusicalTime::ZERO,
            seconds_per_whole: Ratio::from_integer(4),
            ramp: Some(RampRate {
                to: Ratio::from_integer(8),
                over: Ratio::from_integer(4),
                shape,
            }),
            seconds_offset: Ratio::ZERO,
        };
        assert_eq!(point.elapsed(Ratio::from_integer(2)), Ratio::from_integer(9));
        assert_eq!(point.elapsed(Ratio::from_integer(4)), Ratio::from_integer(22));
        // Past the reach it is simply the rate it arrived at.
        assert_eq!(point.elapsed(Ratio::from_integer(5)), Ratio::from_integer(30));
    }

    /// The straight line through the same endpoints, for the comparison the
    /// test above rests on.
    #[test]
    fn the_straight_line_is_the_average_of_the_endpoints() {
        let point = TempoPoint {
            position: MusicalTime::ZERO,
            seconds_per_whole: Ratio::from_integer(4),
            ramp: Some(RampRate {
                to: Ratio::from_integer(8),
                over: Ratio::from_integer(4),
                shape: musa_events::Progress::linear(),
            }),
            seconds_offset: Ratio::ZERO,
        };
        assert_eq!(point.elapsed(Ratio::from_integer(4)), Ratio::from_integer(24));
    }
}
