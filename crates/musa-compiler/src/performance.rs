//! Performance lowering (roadmap §6.4, §15.3; course correction §22): the
//! neutral core that integrates the tempo map and schedules a
//! `ScoreSnapshot` into frame-exact note-on/note-off events.
//!
//! Tempo is a monotone map `Beat → Second` applied to symbolic positions
//! (§22) — it never rewrites the symbolic timeline, and "stretch" (a kernel
//! time action) is not "tempo" (a performance map). Symbolic stays in beats
//! until this boundary; floats (frequency, seconds→frames) appear only here.
//!
//! Neutrality is the point (roadmap §2): a written A4 becomes a frequency
//! only through the tuning service; symbolic dynamics are not velocities;
//! notated durations are full gates until interpretation profiles
//! (prompt 23) shorten them.

// Time accumulation uses `MusicalTime`/`MusicalDuration` operators, total
// for musa's magnitudes (see `time.rs`); the workspace arithmetic lint is
// allowed at module scope for that reason.
#![allow(clippy::arithmetic_side_effects)]

use crate::origin::Origin;
use crate::pitch::WrittenPitch;
use crate::score::{EventId, PartId, ScoreEvent, ScoreEventKind, ScoreSnapshot};
use crate::time::MusicalTime;

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

/// Options for performance lowering.
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
/// Tempo points give beats-per-minute from a position onward. One point
/// today (the grammar has a single tempo); prompt 31's curves extend the
/// data, not the code.
#[derive(Clone, Debug, PartialEq)]
pub struct IntegratedTempoMap {
    points: Vec<TempoPoint>,
}

/// One tempo segment: from `position` (whole notes), `bpm` beats of
/// `beat`-fractions per minute, and the cumulative frame offset at
/// `position` (precomputed against the target sample rate).
#[derive(Clone, Copy, Debug, PartialEq)]
struct TempoPoint {
    position: MusicalTime,
    bpm: u32,
    beat: num_rational::Ratio<i64>,
    frame_offset: u64,
    sample_rate: u32,
}

impl TempoPoint {
    /// Seconds per whole note inside this segment.
    fn seconds_per_whole(&self) -> f64 {
        // `beat` is the beat unit as a fraction of a whole note; a whole
        // note holds `1/beat` beats.
        let beats_per_whole = 1.0 / (*self.beat.numer() as f64 / *self.beat.denom() as f64);
        60.0 * beats_per_whole / f64::from(self.bpm)
    }
}

impl IntegratedTempoMap {
    /// Build the map from a snapshot's tempo declaration.
    pub fn new(snapshot: &ScoreSnapshot, options: &PerformanceOptions) -> Self {
        let tempo = &snapshot.tempo_map;
        Self {
            points: vec![TempoPoint {
                position: MusicalTime::ZERO,
                bpm: tempo.bpm,
                beat: tempo.beat,
                frame_offset: 0,
                sample_rate: options.sample_rate,
            }],
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
        let whole_notes = position.as_ratio() - point.position.as_ratio();
        let whole_notes = *whole_notes.numer() as f64 / *whole_notes.denom() as f64;
        let frames = whole_notes * point.seconds_per_whole() * f64::from(point.sample_rate);
        point.frame_offset.saturating_add(frames.round().max(0.0) as u64)
    }
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
    /// A note ends (full notated gate until prompt 23 profiles).
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

/// The scheduled performance of a score.
#[derive(Clone, Debug, PartialEq)]
pub struct PerformancePlan {
    tempo: IntegratedTempoMap,
    lanes: Vec<PerformanceLane>,
}

impl PerformancePlan {
    /// The tempo map used for scheduling.
    pub fn tempo(&self) -> &IntegratedTempoMap {
        &self.tempo
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
    let mut lanes = Vec::new();
    let mut next_instance = 0u32;
    for (_, part) in score.parts.iter() {
        let mut events = Vec::new();
        for voice in part.voices.values() {
            for event in &voice.events {
                lower_event(score, options, &tempo, event, &mut events, &mut next_instance);
            }
        }
        sort_events(&mut events);
        lanes.push(PerformanceLane {
            part: part.id,
            name: part.name.clone(),
            events,
        });
    }
    Ok(PerformancePlan { tempo, lanes })
}

/// Lower one score event: notes and chord tones become on/off pairs; rests
/// schedule nothing (absence is silence; course correction §2).
fn lower_event(
    _score: &ScoreSnapshot,
    options: &PerformanceOptions,
    tempo: &IntegratedTempoMap,
    event: &ScoreEvent,
    events: &mut Vec<PerformanceEvent>,
    next_instance: &mut u32,
) {
    let pitches: &[WrittenPitch] = match &event.kind {
        ScoreEventKind::Note { pitch } => std::slice::from_ref(pitch),
        ScoreEventKind::Chord { pitches } => pitches,
        ScoreEventKind::Rest => &[],
    };
    let end = event.onset + event.notated_duration.value;
    let on_frame = tempo.frames(event.onset);
    let off_frame = tempo.frames(end);
    for pitch in pitches {
        let instance = VoiceInstanceId(*next_instance);
        *next_instance = next_instance.saturating_add(1);
        let note = PerformedNote {
            pitch: *pitch,
            frequency: options.tuning.frequency(pitch),
            event: event.id,
            origin: event.origin.clone(),
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
