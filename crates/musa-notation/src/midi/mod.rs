//! MIDI export (roadmap §12.5): an **edge format**, never canonical.
//!
//! MIDI numbers exist only in this module. The score keeps written pitch —
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
//! Both read the same exact [`GesturePlan`]. Its occurrence span is the
//! performed fact; its separate immutable lineage projection carries written
//! support (§2: notated duration ≠ performed duration). Frame scheduling
//! belongs only to audio.
//!
//! **Every decision lives in [`schedule`], and this file only writes a
//! container.** A Standard MIDI File is one reading of a [`MidiSchedule`]; a
//! live `CoreMIDI` projection is another. Neither may decide anything of its
//! own, which is what lets a differential law hold the two to each other
//! (`docs/rules/across-stages/06-daw-boundary.md` §2).

// Frame→tick conversion is exact for musa's magnitudes; the workspace
// arithmetic lint is allowed at module scope for that reason.
#![allow(clippy::arithmetic_side_effects)]

mod schedule;

use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind};
use musa_score::{EventId, GesturePlan};

use crate::error::RenderError;

pub use crate::midi::schedule::{
    MidiLoss, MidiMeta, MidiPart, MidiSchedule, ScheduledMessage, ScheduledMeta, midi_schedule,
};

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

/// One track of the written file, and which part it carries.
///
/// A consumer that has to say "this MIDI track is that part" should not have
/// to re-derive the channel rule from the file: the schedule decided it, so
/// the renderer reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiTrack {
    /// Position in the file, counting the tempo track as zero.
    pub index: usize,
    /// The part name, or `None` for the tempo track.
    pub part: Option<String>,
    /// The channel its notes are written on, or `None` for the tempo track.
    pub channel: Option<u8>,
}

/// One written note-on and the source event it came from.
///
/// This is the origin projection, not a second reading of the file: the
/// schedule already resolved every gesture's written lineage in order to
/// place the note, and this reports what it resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MidiOrigin {
    /// Which track of the file, counting the tempo track as zero.
    pub track: usize,
    /// The absolute tick the note-on lands on.
    pub tick: u32,
    /// The MIDI key number written.
    pub key: u8,
    /// The source event the gesture was written from.
    pub event: EventId,
}

/// A rendered Standard MIDI File and what the schedule decided about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedMidi {
    bytes: Vec<u8>,
    tracks: Vec<MidiTrack>,
    origins: Vec<MidiOrigin>,
}

impl RenderedMidi {
    /// The file.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Every track, in file order.
    pub fn tracks(&self) -> &[MidiTrack] {
        &self.tracks
    }

    /// Every written note-on and the source event behind it, ordered by
    /// track and then by tick — the order they appear in the file.
    pub fn origins(&self) -> &[MidiOrigin] {
        &self.origins
    }
}

/// Render a performance plan as a Standard MIDI File.
///
/// # Errors
/// [`RenderError::Unsupported`] when a written pitch falls outside MIDI's
/// 0–127 range, which no other backend cares about.
pub fn render_midi(performance: &GesturePlan, options: &MidiOptions) -> Result<Vec<u8>, RenderError> {
    render_midi_with_origins(performance, options).map(|rendered| rendered.bytes)
}

/// The same file, with the track and origin facts the schedule settled.
///
/// # Errors
/// The same as [`render_midi`].
pub fn render_midi_with_origins(performance: &GesturePlan, options: &MidiOptions) -> Result<RenderedMidi, RenderError> {
    write_schedule(&midi_schedule(performance, options)?)
}

/// Write one already-decided schedule as a Standard MIDI File.
///
/// Pure transport: every tick, channel, velocity, and ordering below was
/// settled by [`midi_schedule`], and nothing here reconsiders one.
///
/// # Errors
/// [`RenderError::Xml`] if the container cannot be serialized, which the
/// writer reports the same way every other backend reports a write failure.
pub fn write_schedule(schedule: &MidiSchedule) -> Result<RenderedMidi, RenderError> {
    let span = tracing::info_span!("write_schedule", parts = schedule.parts().len());
    let _entered = span.enter();
    let mut smf = Smf::new(Header::new(
        Format::Parallel,
        Timing::Metrical(u15::new(schedule.ticks_per_quarter())),
    ));
    smf.tracks.push(conductor_track(schedule));
    let mut tracks = vec![MidiTrack {
        index: 0,
        part: None,
        channel: None,
    }];
    let mut origins = Vec::new();
    for (index, part) in schedule.parts().iter().enumerate() {
        smf.tracks.push(part_track(schedule, index, part));
        tracks.push(MidiTrack {
            index: part.track,
            part: Some(part.name.clone()),
            channel: Some(part.channel),
        });
        origins.extend(
            schedule
                .messages()
                .iter()
                .filter(|message| message.part == index && message.is_note_on())
                .filter_map(|message| {
                    Some(MidiOrigin {
                        track: part.track,
                        tick: u32::try_from(message.tick).unwrap_or(u32::MAX),
                        key: message.data[0],
                        event: message.origin?,
                    })
                }),
        );
    }
    let mut bytes = Vec::new();
    smf.write(&mut bytes)
        .map_err(|error| RenderError::Xml(error.to_string()))?;
    tracing::debug!(
        bytes = bytes.len(),
        tracks = smf.tracks.len(),
        "wrote a standard MIDI file"
    );
    Ok(RenderedMidi { bytes, tracks, origins })
}

/// The conductor's track: every tempo and every meter, at the tick each takes
/// effect.
///
/// SMF has one of each for the whole file, which is what makes them a track
/// rather than a property of a part — and what makes polytempo lossy here.
fn conductor_track(schedule: &MidiSchedule) -> Track<'static> {
    let mut events = Vec::new();
    let mut previous = 0u64;
    for entry in schedule.meta() {
        let Some(message) = meta_message(entry.what) else {
            continue;
        };
        events.push(TrackEvent {
            delta: u28::new(u32::try_from(entry.tick.saturating_sub(previous)).unwrap_or(u32::MAX)),
            kind: TrackEventKind::Meta(message),
        });
        previous = entry.tick;
    }
    events.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    events
}

/// One conductor statement as SMF writes it, or `None` where SMF cannot.
fn meta_message(what: MidiMeta) -> Option<MetaMessage<'static>> {
    match what {
        MidiMeta::Tempo(micros) => Some(MetaMessage::Tempo(u24::new(micros))),
        MidiMeta::Meter(meter) => time_signature(meter),
        MidiMeta::Key(key) => Some(key_signature(key)),
    }
}

/// A meter as SMF writes one, or `None` when SMF cannot say it.
///
/// The denominator is written as a power of two, so a meter whose denominator
/// is not one — `4/6`, which real music does use — has no MIDI spelling at
/// all. Dropping it leaves the previous signature standing, which is wrong in
/// exactly the way MIDI is always wrong about notation; the page says the
/// truth.
fn time_signature(meter: musa_score::Meter) -> Option<MetaMessage<'static>> {
    // Unmeasured music has nothing to say here, and SMF has no way to say
    // "the barlines stop" — so the events simply stop, which is the honest
    // silence rather than a signature of no beats.
    if !meter.is_measured() {
        return None;
    }
    let denominator = meter.denominator();
    if !denominator.is_power_of_two() {
        return None;
    }
    let numerator = u8::try_from(meter.numerator()).ok()?;
    let power = u8::try_from(denominator.trailing_zeros()).ok()?;
    // 24 MIDI clocks to the quarter and 8 thirty-second notes to 24 clocks:
    // the conventional values, and the ones every sequencer writes.
    Some(MetaMessage::TimeSignature(numerator, power, 24, 8))
}

/// A key as SMF writes one: fifths, and whether it is minor.
///
/// Unlike the meter this is total — SMF's key signature is exactly musa's
/// (fifths on the circle, major or minor), which is the one place the two
/// formats agree completely.
fn key_signature(key: musa_score::Key) -> MetaMessage<'static> {
    MetaMessage::KeySignature(key.fifths(), key.mode() == musa_score::Mode::Minor)
}

/// One track per part, named, with its messages on one channel.
fn part_track<'a>(schedule: &'a MidiSchedule, index: usize, part: &'a MidiPart) -> Track<'a> {
    let mut track = Track::new();
    // The file says which part each track is. Logic and GarageBand show this
    // name in their track headers, and a bundle of tracks called `Track 2` is
    // a bundle nobody can mix.
    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::TrackName(part.name.as_bytes())),
    });
    let channel = u4::new(part.channel & 0x0F);
    let mut previous = 0u64;
    for entry in schedule.messages().iter().filter(|message| message.part == index) {
        let Some(message) = voice_message(entry) else {
            continue;
        };
        track.push(TrackEvent {
            delta: u28::new(u32::try_from(entry.tick.saturating_sub(previous)).unwrap_or(u32::MAX)),
            kind: TrackEventKind::Midi { channel, message },
        });
        previous = entry.tick;
    }
    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    track
}

/// One scheduled message in midly's vocabulary.
fn voice_message(message: &ScheduledMessage) -> Option<MidiMessage> {
    let key = u7::new(message.data[0] & 0x7F);
    let value = u7::new(message.data[1] & 0x7F);
    match message.status & 0xF0 {
        0x80 => Some(MidiMessage::NoteOff { key, vel: value }),
        0x90 => Some(MidiMessage::NoteOn { key, vel: value }),
        0xB0 => Some(MidiMessage::Controller { controller: key, value }),
        _ => None,
    }
}
