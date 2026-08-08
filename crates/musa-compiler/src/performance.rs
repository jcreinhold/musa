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

use crate::origin::Origin;
use crate::pitch::WrittenPitch;
use crate::profile::ArticulationRealization;
use crate::score::{EventId, PartId, ScoreEvent, ScoreEventKind, ScoreSnapshot};
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
/// One segment per written tempo. Each segment carries the exact frame at
/// which it begins, accumulated as a rational: the rounding to whole frames
/// happens once, at the position being asked about, so a tempo change never
/// accumulates the drift that rounding each segment's start would.
#[derive(Clone, Debug, PartialEq)]
pub struct IntegratedTempoMap {
    points: Vec<TempoPoint>,
    sample_rate: u32,
}

/// One tempo segment: from `position` (whole notes), at `frames_per_whole`
/// frames per whole note, starting `frame_offset` frames into the piece.
/// Both rates are exact — the map's whole job is to stay exact until the
/// answer is a frame number.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TempoPoint {
    position: MusicalTime,
    frames_per_whole: Ratio<i64>,
    seconds_per_whole: Ratio<i64>,
    frame_offset: Ratio<i64>,
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
    /// Build the map from a snapshot's tempo declarations.
    pub fn new(snapshot: &ScoreSnapshot, options: &PerformanceOptions) -> Self {
        let tempo = snapshot.tempo();
        let rate = Ratio::from_integer(i64::from(options.sample_rate.max(1)));
        let segment = |position: MusicalTime, beat: Ratio<i64>, bpm: u32, frame_offset: Ratio<i64>| {
            let seconds = seconds_per_whole(beat, bpm);
            TempoPoint {
                position,
                frames_per_whole: seconds * rate,
                seconds_per_whole: seconds,
                frame_offset,
            }
        };
        let mut points = vec![segment(MusicalTime::ZERO, tempo.beat, tempo.bpm, Ratio::ZERO)];
        for change in &tempo.changes {
            // Where the previous segment has carried the music to by the time
            // this one starts. Changes are in playing order (the elaborator
            // sorts them), so `last` is always the segment being left.
            let offset = points.last().map_or(Ratio::ZERO, |previous| {
                previous.frame_offset
                    + (change.at.as_ratio() - previous.position.as_ratio()) * previous.frames_per_whole
            });
            points.push(segment(change.at, change.beat, change.bpm, offset));
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
    pub fn segments(&self) -> Vec<TempoSegment> {
        self.points
            .iter()
            .map(|point| TempoSegment {
                position: point.position,
                frame: ratio_to_f64(point.frame_offset).round().max(0.0) as u64,
                seconds_per_quarter: ratio_to_f64(point.seconds_per_whole) / 4.0,
            })
            .collect()
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
        let frames = point.frame_offset + (position.as_ratio() - point.position.as_ratio()) * point.frames_per_whole;
        // One rounding, at the end: the segment offsets above are exact.
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

/// The scheduled performance of a score.
#[derive(Clone, Debug, PartialEq)]
pub struct PerformancePlan {
    tempo: IntegratedTempoMap,
    meters: Vec<MeterChange>,
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
    let mut lanes = Vec::new();
    let mut next_instance = 0u32;
    for (_, part) in score.parts().iter() {
        let profile = score.profiles().for_part(part.name());
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
                    gate: realization.gate,
                    attack: ratio_to_f32(realization.attack),
                    amplitude: ratio_to_f32(amplitude),
                };
                lower_event(options, &tempo, event, &interpreted, &mut events, &mut next_instance);
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
            meter,
        })
        .collect();
    Ok(PerformancePlan { tempo, meters, lanes })
}

/// What the part's profile makes of one event, resolved once per event.
struct Interpreted {
    /// Fraction of the written value that actually sounds.
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

/// Exact ratio → the float the DSP edge needs. This is the boundary the
/// roadmap allows floats to appear at, and the only one.
fn ratio_to_f32(value: Ratio<i64>) -> f32 {
    *value.numer() as f32 / *value.denom() as f32
}

/// Lower one score event: notes and chord tones become on/off pairs; rests
/// schedule nothing (absence is silence; course correction §2).
fn lower_event(
    options: &PerformanceOptions,
    tempo: &IntegratedTempoMap,
    event: &ScoreEvent,
    interpreted: &Interpreted,
    events: &mut Vec<PerformanceEvent>,
    next_instance: &mut u32,
) {
    let pitches: &[WrittenPitch] = match &event.kind {
        ScoreEventKind::Note { pitch } => std::slice::from_ref(pitch),
        ScoreEventKind::Chord { pitches } => pitches,
        ScoreEventKind::Rest => &[],
    };
    let written = event.notated_duration.value;
    let notated_end = event.onset + written;
    let sounded_end = event.onset + crate::time::MusicalDuration::new(written.as_ratio() * interpreted.gate);
    let on_frame = tempo.frames(event.onset);
    let off_frame = tempo.frames(sounded_end);
    let notated_off = tempo.frames(notated_end);
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
