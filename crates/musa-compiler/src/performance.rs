//! Performance resolver (roadmap §6.4, §15.3; course correction §22): the
//! neutral core that integrates the tempo map and schedules a
//! `ScoreSnapshot` into frame-exact note-on/note-off events.
//!
//! Tempo is a monotone map `Beat → Second` applied to symbolic positions
//! (§22) — it never rewrites the symbolic timeline, and "stretch" (a kernel
//! time action) is not "tempo" (a performance map). Symbolic stays in beats
//! until this boundary; floats (frequency, seconds→frames) appear only here.
//!
//! Neutrality is the point (roadmap §2): a written A4 becomes a frequency
//! only through the tuning service, and symbolic dynamics are not
//! velocities. Interpretation enters here and only here: the part's profile
//! (`profile.rs`) turns the written marks into a gate, an amplitude, and an
//! attack request. A part with no profile is scheduled exactly as it was
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
use num_rational::Ratio;

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
        let midi = 12.0f64.mul_add(f64::from(pitch.octave + 1), f64::from(pitch.semitone()));
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

/// A piecewise-monotone tempo map (course correction §22).
///
/// One segment per written tempo, each carrying the exact number of seconds
/// elapsed before it begins, accumulated as a rational: the rounding to whole
/// frames happens once, at the position being asked about, so a tempo change
/// never accumulates the drift that rounding each segment's start would.
#[derive(Clone, Debug, PartialEq)]
pub struct IntegratedTempoMap {
    points: Vec<TempoPoint>,
    sample_rate: u32,
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
    shape: musa_kernel::Progress,
}

impl TempoPoint {
    /// Seconds elapsed from this segment's start to `whole_notes` into it.
    ///
    /// Total for any distance: past the end of a ramp the rate is simply the
    /// one it arrived at, which is what "the tempo after a *rit.* is the
    /// tempo the *rit.* reached" means arithmetically. A ramp cut short by
    /// the next marking is never asked past the cut, so intersection clamps
    /// it with no rule of its own (prompt 37).
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
    /// Integrate the piece's tempo *markings* into a `Beat → Second` map.
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
    pub fn new(snapshot: &ScoreSnapshot, options: &PerformanceOptions) -> Self {
        let mut marks: Vec<(MusicalTime, &crate::score::TempoMarking)> = snapshot
            .tempos()
            .changes(crate::Scope::Piece)
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
        Self {
            points,
            sample_rate: options.sample_rate,
        }
    }

    /// Seconds per quarter note at the start of the piece — what a metrical
    /// MIDI file's tempo meta-event states.
    pub fn seconds_per_quarter(&self) -> f64 {
        self.points
            .first()
            .map_or(0.5, |point| ratio_to_f64(point.seconds_per_whole) / 4.0)
    }

    /// The sample rate the frames were scheduled against.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The tempo segments, in playing order: what an exporter needs to write
    /// a tempo change into a file that counts in beats rather than frames.
    ///
    /// A gradual change has no exact form in any of those files, so it is
    /// **sampled** here, into `steps_per_whole` constant segments per whole
    /// note of its reach. The density is the caller's argument rather than a
    /// constant of the map, because the shape is normative and the sampling
    /// is the consumer's policy (docs/kernel/07) — the same rule a hairpin's
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

    /// One exported segment: a position, the frame it falls on, and the rate
    /// in force there.
    fn segment_at(&self, position: MusicalTime, seconds_per_whole: Ratio<i64>) -> TempoSegment {
        TempoSegment {
            position,
            frame: self.frames(position),
            seconds_per_quarter: ratio_to_f64(seconds_per_whole) / 4.0,
        }
    }

    /// The absolute frame of a symbolic position (monotone; §22).
    pub fn frames(&self, position: MusicalTime) -> u64 {
        let Some(point) = self
            .points
            .iter()
            .rev()
            .find(|point| point.position <= position)
            .or_else(|| self.points.first())
        else {
            return 0;
        };
        let seconds = point.seconds_offset + point.elapsed(position.as_ratio() - point.position.as_ratio());
        // One rounding, at the end: the segment offsets above are exact.
        let frames = seconds * Ratio::from_integer(i64::from(self.sample_rate.max(1)));
        ratio_to_f64(frames).round().max(0.0) as u64
    }
}

/// One tempo segment as an exporter sees it: where it starts, in both
/// symbolic and frame time, and how fast it goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TempoSegment {
    /// The symbolic position the segment starts at.
    pub position: MusicalTime,
    /// The absolute frame the segment starts at.
    pub frame: u64,
    /// Seconds per quarter note inside the segment.
    pub seconds_per_quarter: f64,
}

/// The one place a musical rational becomes a float.
fn ratio_to_f64(value: Ratio<i64>) -> f64 {
    *value.numer() as f64 / *value.denom() as f64
}

/// Identity of one sounding note instance, matching note-on to note-off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VoiceInstanceId(pub u32);

/// A parameter target for `Parameter` events (nothing produces them until
/// prompts 24–26; the type exists now so the event enum is stable).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterId(pub u32);

/// A sounding note: written pitch plus derived frequency, with provenance.
/// Frequency is derived at this boundary, never stored in the score.
///
/// The interpreted fields come from the part's profile (roadmap §6.4). With
/// no profile they are exactly neutral, which is what keeps a piece that
/// declares none sounding bit-for-bit as it did before profiles existed.
#[derive(Clone, Debug, PartialEq)]
pub struct PerformedNote {
    /// The written pitch (post-transposition; sounding = written until
    /// transposing instruments exist).
    pub pitch: WrittenPitch,
    /// Frequency in Hz from the tuning service.
    pub frequency: f64,
    /// The score event this note realizes.
    pub event: EventId,
    /// Why the event exists.
    pub origin: Origin,
    /// Interpreted loudness in `0..=1`, from the prevailing dynamic marking.
    /// Abstract, not decibels and not a MIDI velocity (§2); `1.0` is neutral.
    pub amplitude: f32,
    /// The attack time in seconds the profile asks for. A request carried to
    /// the instrument, not an envelope: prompt 30's parameter system decides
    /// what an instrument does with it.
    pub attack: f32,
    /// The frame the *written* value ends at, before the profile's gate.
    /// Notated duration ≠ performed duration (§2) and both are facts: score
    /// MIDI wants this one, performance MIDI wants the note-off's.
    pub notated_off: u64,
    /// The frame the note starts at *on the page*, before the part's groove.
    /// The same fact as `notated_off` in the other direction: a swung file is
    /// a performance, and a notation program reading a score-mode export must
    /// not be handed an interpretation to draw (prompt 69).
    pub notated_on: u64,
}

/// One scheduled performance event.
#[derive(Clone, Debug, PartialEq)]
pub enum PerformanceEvent {
    /// A note starts.
    NoteOn {
        /// Absolute frame.
        frame: u64,
        /// The sounding note.
        note: PerformedNote,
        /// Instance identity for the matching note-off.
        instance: VoiceInstanceId,
    },
    /// A note ends, at the profile's gate of the written value.
    NoteOff {
        /// Absolute frame.
        frame: u64,
        /// The instance ending.
        instance: VoiceInstanceId,
    },
    /// A parameter change (unused until prompts 24–26; present for enum
    /// stability).
    Parameter {
        /// Absolute frame.
        frame: u64,
        /// What changes.
        target: ParameterId,
        /// The new value.
        value: f32,
    },
}

impl PerformanceEvent {
    /// The event's frame.
    pub fn frame(&self) -> u64 {
        match self {
            Self::NoteOn { frame, .. } | Self::NoteOff { frame, .. } | Self::Parameter { frame, .. } => *frame,
        }
    }
}

/// One part's scheduled events (an instrument lane; parts ≠ synthesizers —
/// the lane is scheduled data, not a synth instance).
#[derive(Clone, Debug, PartialEq)]
pub struct PerformanceLane {
    part: PartId,
    name: String,
    events: Vec<PerformanceEvent>,
}

impl PerformanceLane {
    /// The part identity.
    pub fn part(&self) -> PartId {
        self.part
    }

    /// The part name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Events sorted by frame, then `EventId`, with ons before offs.
    pub fn events(&self) -> &[PerformanceEvent] {
        &self.events
    }
}

/// A time signature and the frame it takes effect at.
///
/// Meter is notation, and a performance does not hear it — which is exactly
/// why it is carried here rather than derived: MIDI is an *edge* format that
/// writes a time-signature meta event, and the only alternative would be for
/// the exporter to hold a second score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeterChange {
    /// Where it starts, in frames.
    pub frame: u64,
    /// What it is.
    pub meter: crate::score::Meter,
}

/// A key signature and the frame it takes effect at.
///
/// Carried for the same reason as [`MeterChange`]: SMF writes a key-signature
/// meta event, and the exporter must not have to hold a second score to know
/// when to write one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyChange {
    /// Where it starts, in frames.
    pub frame: u64,
    /// What it is.
    pub key: crate::score::Key,
}

/// The scheduled performance of a score.
#[derive(Clone, Debug, PartialEq)]
pub struct PerformancePlan {
    tempo: IntegratedTempoMap,
    meters: Vec<MeterChange>,
    keys: Vec<KeyChange>,
    lanes: Vec<PerformanceLane>,
}

impl PerformancePlan {
    /// The tempo map used for scheduling.
    pub fn tempo(&self) -> &IntegratedTempoMap {
        &self.tempo
    }

    /// Every meter the piece states, in playing order, beginning with the one
    /// it opens in.
    pub fn meters(&self) -> &[MeterChange] {
        &self.meters
    }

    /// Every key the piece states, in playing order.
    pub fn keys(&self) -> &[KeyChange] {
        &self.keys
    }

    /// One lane per part, in source order.
    pub fn lanes(&self) -> &[PerformanceLane] {
        &self.lanes
    }
}

/// A failure to lower a score for performance. Reserved: the current
/// grammar lowers everything; the type exists so the facade is stable.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PerformanceError {
    /// A construct the performance layer cannot schedule.
    #[error("unsupported performance construct: {0}")]
    Unsupported(String),
}

/// Lower a score into a frame-scheduled `PerformancePlan`.
///
/// # Errors
/// [`PerformanceError`] for unschedulable constructs (none in the current
/// grammar).
pub fn lower_performance(
    score: &ScoreSnapshot,
    options: &PerformanceOptions,
) -> Result<PerformancePlan, PerformanceError> {
    let tempo = IntegratedTempoMap::new(score, options);
    let marks = Interpretation::collect(score);
    // The hairpin index covers the whole piece: an event belongs to exactly
    // one voice, so one map keyed by event id serves every voice.
    let curves = hairpin_curves(score);
    let graces = grace_index(score);
    let mut lanes = Vec::new();
    let mut next_instance = 0u32;
    for (_, part) in score.parts().iter() {
        let profile = score.profiles().for_part(part.name());
        // The groove is the part's, because feel is an ensemble's sections
        // disagreeing on purpose: a swung horn over a straight bass is a
        // arrangement, not a mistake.
        let clock = Clock {
            tempo: &tempo,
            meters: score.meters(),
            groove: profile.map_or(Groove::STRAIGHT, PerformanceProfile::groove),
        };
        // What the part's reading makes of a grace note. Notation is silent on
        // this by design (roadmap §2): the page says *a grace note*, and
        // whether it lands on the beat or ahead of it is the performer's, so
        // it is the profile's here.
        let policy = profile.map_or(crate::GracePolicy::DEFAULT, PerformanceProfile::grace);
        let mut events = Vec::new();
        for (_, voice) in part.voices() {
            // The prevailing dynamic is per voice: a marking applies from its
            // event onward in the voice that wrote it, not across the part.
            //
            // This is the kernel's prevailing rule (docs/kernel/03 D11) applied
            // in bulk — one ordered pass over the voice, carrying the last
            // marking forward — and not one `Timeline::prevailing` call per
            // event, which would be O(events × markings). The two conventions
            // D11 fixes are honoured here: a marking on an event is in force
            // *at* that event (the assignment precedes the read below), and of
            // two markings at one instant the canonically later wins, because
            // `Marks::collect` inserts them in the annotation lane's order and
            // the last insertion keeps the key.
            let mut dynamic = None;
            // The loudness a hairpin grows from: whatever was in force at its
            // first note, which is what a hairpin means on the page.
            let mut curve_from: Option<Ratio<i64>> = None;
            // How far back a grace note may reach, and what it shortens when
            // it does. Both are per voice: a grace leans on the line it is
            // written in, not on whatever else the part happens to sound.
            let mut floor = MusicalTime::ZERO;
            let mut previous = 0..0;
            for event in voice.events() {
                if let Some(mark) = marks.dynamics.get(&event.id) {
                    dynamic = Some(*mark);
                }
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
                        // A hairpin arrives at its mark, and leaves it in
                        // force for what follows.
                        if curve.fraction == Ratio::ONE {
                            dynamic = Some(curve.target);
                            curve_from = None;
                        }
                        reached
                    }
                };
                let interpreted = Interpreted {
                    gate: realization.gate * realization.hold,
                    attack: ratio_to_f32(realization.attack),
                    amplitude: ratio_to_f32(amplitude),
                };
                // A grace note's own marks are read by the same profile that
                // reads the principal's: a staccato grace is short for the same
                // reason a staccato note is, and nothing about leaning on
                // another note changes that.
                let leaning: Vec<GraceSlot> = graces
                    .get(&event.id)
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .map(|grace| GraceSlot {
                        pitch: grace.pitch,
                        gate: profile.map_or(Ratio::ONE, |profile| {
                            let realized = profile.realize(&grace.articulations);
                            realized.gate * realized.hold
                        }),
                    })
                    .collect();
                let lowered = lower_event(
                    options,
                    &clock,
                    event,
                    &interpreted,
                    &leaning,
                    policy,
                    floor,
                    &mut events,
                    &mut next_instance,
                );
                // A grace taken from the note before is only honest if that
                // note actually gives the time up: the previous note-off is
                // pulled back to where the grace starts. `min` because a gate
                // may already have ended it sooner — a staccato note does not
                // get *longer* because the next note has a grace.
                if let Some(at) = lowered.anticipated {
                    let frame = clock.frames(at);
                    for index in previous.clone() {
                        if let Some(PerformanceEvent::NoteOff { frame: off, .. }) = events.get_mut(index) {
                            *off = (*off).min(frame);
                        }
                    }
                }
                // The bound the next event's graces may reach back to: the
                // midpoint of this note. Rests leave it where it was — silence
                // gives way freely, so a grace may take all of one.
                if !matches!(event.kind, ScoreEventKind::Rest) {
                    floor =
                        event.onset + crate::time::MusicalDuration::new(event.notated_duration.value.as_ratio() / 2);
                }
                previous = lowered.pushed;
            }
        }
        sort_events(&mut events);
        lanes.push(PerformanceLane {
            part: part.id(),
            name: part.name().to_string(),
            events,
        });
    }
    let meters = score
        .meters()
        .changes(crate::Scope::Piece)
        .map(|(at, meter)| MeterChange {
            frame: tempo.frames(at),
            meter: *meter,
        })
        .collect();
    let keys = score
        .keys()
        .changes(crate::Scope::Piece)
        .map(|(at, key)| KeyChange {
            frame: tempo.frames(at),
            key: *key,
        })
        .collect();
    Ok(PerformancePlan {
        tempo,
        meters,
        keys,
        lanes,
    })
}

/// Written time to frames, for one part.
///
/// The composition order is the whole point (course correction §22): the
/// groove is a `Beat → Beat` warp and tempo is `Beat → Second`, so the groove
/// goes **first**. Composed the other way a shuffle would be specified in
/// seconds and would straighten out as the band sped up.
///
/// It is one operation rather than two exposed ones because a caller that
/// reached for `tempo.frames` directly would silently drop the groove, and
/// nothing in the output would say so.
struct Clock<'a> {
    tempo: &'a IntegratedTempoMap,
    meters: &'a crate::ContextTrack<Meter>,
    groove: Groove,
}

impl Clock<'_> {
    /// The frame a written instant is played at.
    fn frames(&self, at: MusicalTime) -> u64 {
        let meter = self.meters.at(crate::Scope::Piece, at).copied().unwrap_or_default();
        self.tempo.frames(self.groove.warp(meter, at))
    }

    /// The frame a written instant sits at *on the page* — tempo, no groove.
    fn written_frames(&self, at: MusicalTime) -> u64 {
        self.tempo.frames(at)
    }
}

/// What the part's profile makes of one event, resolved once per event.
struct Interpreted {
    /// Fraction of the written value that actually sounds, with the profile's
    /// hold already folded in: both are multipliers on the written value and
    /// nothing downstream could tell them apart.
    gate: Ratio<i64>,
    /// Requested attack in seconds.
    attack: f32,
    /// Loudness in `0..=1`.
    amplitude: f32,
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
/// below picks `u`, the kernel's `Progress` says what fraction of the distance
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
/// **Shape versus sampling policy** (docs/kernel/07). The shape — how the
/// growth is distributed across the region — is a fact about the piece: it
/// lives in the timeline as a `Progress`, it serializes, and every conforming
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
/// The sort is by `index`, the ordering the payload carries (docs/kernel/05
/// N2). Normalization sorts occurrences by span then payload key, and every
/// grace in a group shares a span — so `grace { c5; d5; }` and
/// `grace { d5; c5; }` are told apart by nothing else. Reading the lane's
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

/// One grace note as the clock sees it: a pitch and how much of its stolen
/// slot it actually sounds.
struct GraceSlot {
    pitch: WrittenPitch,
    gate: Ratio<i64>,
}

/// Exact ratio → the float the DSP edge needs. This is the boundary the
/// roadmap allows floats to appear at, and the only one.
fn ratio_to_f32(value: Ratio<i64>) -> f32 {
    *value.numer() as f32 / *value.denom() as f32
}

/// Lower one score event: notes and chord tones become on/off pairs; rests
/// schedule nothing (absence is silence; course correction §2).
///
/// **Where a grace note's time comes from.** A grace is a *point* occurrence —
/// zero written duration — so performance is where it acquires one, and the
/// time has to come out of a neighbour. Which neighbour is `policy`, and the
/// asymmetry between the two answers is the point of keeping it out of the
/// notation: stealing from the principal delays it and leaves the bar intact;
/// stealing from the previous note leaves the principal exactly where the page
/// puts it. Either way `notated_on`/`notated_off` — what the editor highlights
/// — are computed from the written values and never move.
#[allow(clippy::too_many_arguments)]
fn lower_event(
    options: &PerformanceOptions,
    clock: &Clock<'_>,
    event: &ScoreEvent,
    interpreted: &Interpreted,
    leaning: &[GraceSlot],
    policy: crate::GracePolicy,
    floor: MusicalTime,
    events: &mut Vec<PerformanceEvent>,
    next_instance: &mut u32,
) -> Lowered {
    let pitches: &[WrittenPitch] = match &event.kind {
        ScoreEventKind::Note { pitch } => std::slice::from_ref(pitch),
        ScoreEventKind::Chord { pitches } => pitches,
        ScoreEventKind::Rest => &[],
    };
    let written = event.notated_duration.value;
    let notated_end = event.onset + written;
    let notated_off = clock.written_frames(notated_end);
    let notated_on = clock.written_frames(event.onset);
    let stolen = steal(event, written, leaning.len(), policy, floor);
    let mut at = stolen.graces_start_at;
    for slot in leaning {
        let instance = VoiceInstanceId(*next_instance);
        *next_instance = next_instance.saturating_add(1);
        let sounds = crate::time::MusicalDuration::new(stolen.each.as_ratio() * slot.gate);
        events.push(PerformanceEvent::NoteOn {
            frame: clock.frames(at),
            note: PerformedNote {
                pitch: slot.pitch,
                frequency: options.tuning.frequency(&slot.pitch),
                // A grace belongs to the note it leans on: selecting it in the
                // editor selects that note, and highlighting follows the
                // principal's written span because a grace has none of its own.
                event: event.id,
                origin: event.origin.clone(),
                amplitude: interpreted.amplitude,
                attack: interpreted.attack,
                notated_off,
                notated_on,
            },
            instance,
        });
        events.push(PerformanceEvent::NoteOff {
            frame: clock.frames(at + sounds),
            instance,
        });
        at = at + stolen.each;
    }
    let sounded_start = stolen.principal_starts_at;
    let sounded_end =
        sounded_start + crate::time::MusicalDuration::new(stolen.principal_sounds.as_ratio() * interpreted.gate);
    let on_frame = clock.frames(sounded_start);
    let off_frame = clock.frames(sounded_end);
    let first = events.len();
    for pitch in pitches {
        let instance = VoiceInstanceId(*next_instance);
        *next_instance = next_instance.saturating_add(1);
        let note = PerformedNote {
            pitch: *pitch,
            frequency: options.tuning.frequency(pitch),
            event: event.id,
            origin: event.origin.clone(),
            amplitude: interpreted.amplitude,
            attack: interpreted.attack,
            notated_off,
            notated_on,
        };
        events.push(PerformanceEvent::NoteOn {
            frame: on_frame,
            note,
            instance,
        });
        events.push(PerformanceEvent::NoteOff {
            frame: off_frame,
            instance,
        });
    }
    Lowered {
        pushed: first..events.len(),
        anticipated: stolen.anticipated,
    }
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

/// Deterministic order: frame, then offs before ons (a gate ending exactly
/// where another starts must close first, or same-frame transitions
/// overlap), then event identity, then instance (§6.4 stability).
fn sort_events(events: &mut [PerformanceEvent]) {
    fn key(event: &PerformanceEvent) -> (u64, u8, u64, u32) {
        match event {
            PerformanceEvent::NoteOff { frame, instance } => (*frame, 0, u64::MAX, instance.0),
            PerformanceEvent::NoteOn { frame, note, instance } => (*frame, 1, note.event.0, instance.0),
            PerformanceEvent::Parameter { frame, target, .. } => (*frame, 2, 0, target.0),
        }
    }
    events.sort_by_key(key);
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
        let shape = musa_kernel::Progress::piecewise([(r(0, 1), r(0, 1)), (r(1, 2), r(1, 4)), (r(1, 1), r(1, 1))])
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
                shape: musa_kernel::Progress::linear(),
            }),
            seconds_offset: Ratio::ZERO,
        };
        assert_eq!(point.elapsed(Ratio::from_integer(4)), Ratio::from_integer(24));
    }
}
