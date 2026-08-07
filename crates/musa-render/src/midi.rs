//! MIDI export (roadmap §12.5): an **edge format**, never canonical.
//!
//! MIDI numbers exist only in this file. The score keeps written pitch —
//! D♯ and E♭ are different notes there and the same number here, which is
//! exactly why the mapping happens at the boundary and not before it.
//!
//! Two modes, because a score and a performance are different documents:
//!
//! - [`MidiMode::Score`] is the neutral reading — every note lasts its
//!   *written* value and every note carries one velocity. It is what you hand
//!   to another engraver.
//! - [`MidiMode::Performance`] is the interpreted one — gates from the part's
//!   profile, velocity from the prevailing dynamic. It is what you hand to a
//!   sampler.
//!
//! Both read the same [`PerformancePlan`], which carries both facts per note
//! (§2: notated duration ≠ performed duration).

// Frame→tick conversion is exact for musa's magnitudes; the workspace
// arithmetic lint is allowed at module scope for that reason.
#![allow(clippy::arithmetic_side_effects)]

use std::collections::HashMap;

use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind};
use musa_compiler::{PerformanceEvent, PerformancePlan, VoiceInstanceId, WrittenPitch};

use crate::error::RenderError;

/// Which document the MIDI file is (§12.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MidiMode {
    /// Neutral: written durations, one velocity, tempo from the map.
    #[default]
    Score,
    /// Interpreted: profiled gates and dynamic-derived velocity.
    Performance,
}

/// Options for [`render_midi`].
#[derive(Clone, Copy, Debug)]
pub struct MidiOptions {
    /// Which document to write.
    pub mode: MidiMode,
    /// Ticks per quarter note in the written file.
    pub ticks_per_quarter: u16,
}

impl Default for MidiOptions {
    fn default() -> Self {
        Self {
            mode: MidiMode::default(),
            ticks_per_quarter: 480,
        }
    }
}

/// The velocity every note gets in [`MidiMode::Score`]: mezzo-forte, the
/// conventional neutral. Score MIDI states no opinion about loudness, and a
/// file full of velocity 127 is an opinion.
const NEUTRAL_VELOCITY: u8 = 80;

/// Render a performance plan as a Standard MIDI File.
///
/// # Errors
/// [`RenderError::Unsupported`] when a written pitch falls outside MIDI's
/// 0–127 range, which no other backend cares about.
pub fn render_midi(performance: &PerformancePlan, options: &MidiOptions) -> Result<Vec<u8>, RenderError> {
    let ticks = Ticks::new(performance, options.ticks_per_quarter);
    let mut smf = Smf::new(Header::new(
        Format::Parallel,
        Timing::Metrical(u15::new(options.ticks_per_quarter)),
    ));
    smf.tracks.push(tempo_track(performance, &ticks));
    for (index, lane) in performance.lanes().iter().enumerate() {
        // Channel 10 (index 9) is percussion by convention; skipping it keeps
        // a tenth part from being silently rewritten to a drum kit.
        let slot = index % 15;
        let channel = u4::new(u8::try_from(if slot >= 9 { slot + 1 } else { slot }).unwrap_or(0));
        smf.tracks.push(lane_track(lane, channel, &ticks, *options)?);
    }
    let mut bytes = Vec::new();
    smf.write(&mut bytes)
        .map_err(|error| RenderError::Xml(error.to_string()))?;
    Ok(bytes)
}

/// Frames → ticks, one segment per tempo.
///
/// The plan is scheduled in frames; the file is metrical, so ticks are beats
/// and the conversion factor changes wherever the tempo does. Each segment
/// carries the tick its first frame lands on, so a note after a tempo change
/// is placed against the tempo actually in force there rather than against
/// the tempo the piece started in.
struct Ticks {
    segments: Vec<TickSegment>,
}

#[derive(Clone, Copy)]
struct TickSegment {
    frame: u64,
    tick: u64,
    per_frame: f64,
}

impl Ticks {
    fn new(performance: &PerformancePlan, ticks_per_quarter: u16) -> Self {
        let rate = f64::from(performance.tempo().sample_rate());
        let mut segments: Vec<TickSegment> = Vec::new();
        for segment in performance.tempo().segments() {
            let frames_per_quarter = segment.seconds_per_quarter * rate;
            let per_frame = if frames_per_quarter > 0.0 {
                f64::from(ticks_per_quarter) / frames_per_quarter
            } else {
                0.0
            };
            let tick = segments.last().map_or(0, |previous: &TickSegment| {
                let frames = segment.frame.saturating_sub(previous.frame) as f64;
                previous
                    .tick
                    .saturating_add((frames * previous.per_frame).round().max(0.0) as u64)
            });
            segments.push(TickSegment {
                frame: segment.frame,
                tick,
                per_frame,
            });
        }
        Self { segments }
    }

    fn of(&self, frame: u64) -> u64 {
        let Some(segment) = self
            .segments
            .iter()
            .rev()
            .find(|segment| segment.frame <= frame)
            .or_else(|| self.segments.first())
        else {
            return 0;
        };
        let frames = frame.saturating_sub(segment.frame) as f64;
        segment
            .tick
            .saturating_add((frames * segment.per_frame).round().max(0.0) as u64)
    }
}

/// One tempo meta-event per written tempo, at the tick it takes effect.
fn tempo_track(performance: &PerformancePlan, ticks: &Ticks) -> Track<'static> {
    let mut events = Vec::new();
    let mut previous = 0u64;
    for segment in performance.tempo().segments() {
        let micros = (segment.seconds_per_quarter * 1_000_000.0).round().max(1.0) as u32;
        let tick = ticks.of(segment.frame);
        events.push(TrackEvent {
            delta: u28::new(u32::try_from(tick.saturating_sub(previous)).unwrap_or(u32::MAX)),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(micros.min(0x00FF_FFFF)))),
        });
        previous = tick;
    }
    events.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    events
}

/// One track per part, named, with its notes on one channel.
fn lane_track<'a>(
    lane: &musa_compiler::PerformanceLane,
    channel: u4,
    ticks: &Ticks,
    options: MidiOptions,
) -> Result<Track<'a>, RenderError> {
    // Absolute-tick messages first; deltas are a rendering of them.
    let mut absolute: Vec<(u64, u8, MidiMessage)> = Vec::new();
    let mut ends: HashMap<VoiceInstanceId, u64> = HashMap::new();
    for event in lane.events() {
        if let PerformanceEvent::NoteOff { frame, instance } = event {
            ends.insert(*instance, *frame);
        }
    }
    for event in lane.events() {
        let PerformanceEvent::NoteOn { frame, note, instance } = event else {
            continue;
        };
        let key = midi_key(note.pitch).ok_or_else(|| RenderError::Unsupported {
            event: note.event,
            what: format!("written pitch {} is outside MIDI's range", note.pitch),
        })?;
        let (off_frame, velocity) = match options.mode {
            MidiMode::Score => (note.notated_off, NEUTRAL_VELOCITY),
            MidiMode::Performance => (
                ends.get(instance).copied().unwrap_or(note.notated_off),
                velocity_of(note.amplitude),
            ),
        };
        absolute.push((
            ticks.of(*frame),
            // Note-on sorts after note-off at the same tick, so a repeated
            // pitch retriggers instead of being cut by its predecessor.
            1,
            MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(velocity),
            },
        ));
        absolute.push((
            ticks.of(off_frame.max(*frame)),
            0,
            MidiMessage::NoteOff {
                key: u7::new(key),
                vel: u7::new(0),
            },
        ));
    }
    absolute.sort_by_key(|(tick, order, message)| (*tick, *order, message_key(*message)));

    let mut track = Track::new();
    let mut previous = 0u64;
    for (tick, _, message) in absolute {
        track.push(TrackEvent {
            delta: u28::new(u32::try_from(tick.saturating_sub(previous)).unwrap_or(u32::MAX)),
            kind: TrackEventKind::Midi { channel, message },
        });
        previous = tick;
    }
    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    Ok(track)
}

/// A total order among simultaneous messages, so the bytes are deterministic.
fn message_key(message: MidiMessage) -> (u8, u8) {
    match message {
        MidiMessage::NoteOff { key, .. } => (0, key.as_int()),
        MidiMessage::NoteOn { key, .. } => (1, key.as_int()),
        MidiMessage::Aftertouch { .. }
        | MidiMessage::Controller { .. }
        | MidiMessage::ProgramChange { .. }
        | MidiMessage::ChannelAftertouch { .. }
        | MidiMessage::PitchBend { .. } => (2, 0),
    }
}

/// 12-TET written pitch → MIDI note number. This is the tuning service's
/// edge: middle C (`c4`) is 60.
fn midi_key(pitch: WrittenPitch) -> Option<u8> {
    let number = 12 * (i32::from(pitch.octave) + 1) + i32::from(pitch.semitone());
    u8::try_from(number).ok().filter(|key| *key <= 127)
}

/// Abstract amplitude → velocity. Linear and documented rather than clever:
/// a curve here would be a second interpretation layer on top of the profile
/// that already made the choice.
fn velocity_of(amplitude: f32) -> u8 {
    let scaled = (amplitude.clamp(0.0, 1.0) * 127.0).round() as u8;
    scaled.max(1)
}
