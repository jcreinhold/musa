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
//! Both read the same exact [`GesturePlan`]. Its occurrence span is the
//! performed fact; its separate immutable lineage projection carries written
//! support (§2: notated duration ≠ performed duration). Frame scheduling
//! belongs only to audio.

// Frame→tick conversion is exact for musa's magnitudes; the workspace
// arithmetic lint is allowed at module scope for that reason.
#![allow(clippy::arithmetic_side_effects)]

use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind};
use musa_events::{PerformedTime, Position};
use musa_score::{EventId, GestureLane, GesturePlan, MusicalTime, WrittenPitch};
use num_rational::Ratio;

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

/// One track of the written file, and which part it carries.
///
/// A consumer that has to say "this MIDI track is that part" should not have
/// to re-derive the channel rule from the file: the renderer decided it, so
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
/// renderer already resolved every gesture's written lineage in order to
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

/// A rendered Standard MIDI File and what the renderer decided about it.
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

/// The same file, with the track and origin facts the renderer settled while
/// writing it.
///
/// # Errors
/// The same as [`render_midi`].
pub fn render_midi_with_origins(performance: &GesturePlan, options: &MidiOptions) -> Result<RenderedMidi, RenderError> {
    // One track per lane plus the tempo track, and the lane count is the
    // channel-assignment story: past fifteen lanes, parts start sharing a
    // channel, and that is invisible in the file.
    let span = tracing::info_span!("render_midi", lanes = performance.lanes().len());
    let _entered = span.enter();
    let ticks = Ticks::new(performance, options.ticks_per_quarter);
    let mut smf = Smf::new(Header::new(
        Format::Parallel,
        Timing::Metrical(u15::new(options.ticks_per_quarter)),
    ));
    smf.tracks.push(tempo_track(performance, &ticks));
    let mut tracks = vec![MidiTrack {
        index: 0,
        part: None,
        channel: None,
    }];
    let mut origins = Vec::new();
    for (index, lane) in performance.lanes().iter().enumerate() {
        // Channel 10 (index 9) is percussion by convention; skipping it keeps
        // a tenth part from being silently rewritten to a drum kit.
        let slot = index % 15;
        let channel = u4::new(u8::try_from(if slot >= 9 { slot + 1 } else { slot }).unwrap_or(0));
        let track = index.saturating_add(1);
        let (written, placed) = lane_track(lane, performance, channel, &ticks, *options)?;
        smf.tracks.push(written);
        tracks.push(MidiTrack {
            index: track,
            part: Some(lane.name().to_owned()),
            channel: Some(channel.as_int()),
        });
        origins.extend(placed.into_iter().map(|(tick, key, event)| MidiOrigin {
            track,
            tick,
            key,
            event,
        }));
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

/// How finely a gradual tempo change is sampled into constant segments, per
/// whole note of its reach.
///
/// SMF has no continuous tempo — a *rit.* is a run of set-tempo events or it
/// is nothing — so a density has to be chosen, and per docs/rules/events/07 it is
/// chosen **here**, by the consumer, rather than by the map that holds the
/// normative shape. Thirty-two per whole note is a set-tempo every
/// thirty-second note: below the threshold at which a listener hears steps,
/// and small enough that an eight-bar riser costs a few hundred bytes.
const TEMPO_STEPS_PER_WHOLE: u32 = 32;

/// Exact physical seconds → MIDI ticks, one segment per tempo.
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
    seconds: Ratio<i64>,
    tick: u64,
    per_second: f64,
}

impl Ticks {
    fn new(performance: &GesturePlan, ticks_per_quarter: u16) -> Self {
        let mut segments: Vec<TickSegment> = Vec::new();
        for segment in performance.tempo().segments(TEMPO_STEPS_PER_WHOLE) {
            let seconds_per_quarter = ratio_to_f64(segment.seconds_per_quarter);
            let per_second = if seconds_per_quarter > 0.0 {
                f64::from(ticks_per_quarter) / seconds_per_quarter
            } else {
                0.0
            };
            let tick = segments.last().map_or(0, |previous: &TickSegment| {
                let seconds = ratio_to_f64(segment.physical.as_ratio() - previous.seconds);
                previous
                    .tick
                    .saturating_add((seconds * previous.per_second).round().max(0.0) as u64)
            });
            segments.push(TickSegment {
                seconds: segment.physical.as_ratio(),
                tick,
                per_second,
            });
        }
        Self { segments }
    }

    fn of(&self, seconds: Ratio<i64>) -> u64 {
        let Some(segment) = self
            .segments
            .iter()
            .rev()
            .find(|segment| segment.seconds <= seconds)
            .or_else(|| self.segments.first())
        else {
            return 0;
        };
        let elapsed = ratio_to_f64(seconds - segment.seconds).max(0.0);
        segment
            .tick
            .saturating_add((elapsed * segment.per_second).round().max(0.0) as u64)
    }
}

/// The conductor's track: every tempo and every meter, at the tick each takes
/// effect.
///
/// SMF has one of each for the whole file, which is what makes them a track
/// rather than a property of a part — and what will make polytempo lossy
/// here and nowhere else.
fn tempo_track(performance: &GesturePlan, ticks: &Ticks) -> Track<'static> {
    let mut absolute: Vec<(u64, u8, MetaMessage<'static>)> = Vec::new();
    for segment in performance.tempo().segments(TEMPO_STEPS_PER_WHOLE) {
        let micros = (ratio_to_f64(segment.seconds_per_quarter) * 1_000_000.0)
            .round()
            .max(1.0) as u32;
        absolute.push((
            ticks.of(segment.physical.as_ratio()),
            2,
            MetaMessage::Tempo(u24::new(micros.min(0x00FF_FFFF))),
        ));
    }
    for change in performance.meters() {
        let Some(message) = time_signature(change.meter) else {
            continue;
        };
        absolute.push((ticks.of(reference_seconds(performance, change.at)), 0, message));
    }
    for change in performance.keys() {
        absolute.push((
            ticks.of(reference_seconds(performance, change.at)),
            1,
            key_signature(change.key),
        ));
    }
    // A meter, then a key, then a tempo at the same tick — the order a
    // conductor reads them in and the order every other writer emits.
    absolute.sort_by_key(|(tick, rank, _)| (*tick, *rank));
    let mut events = Vec::new();
    let mut previous = 0u64;
    for (tick, _, message) in absolute {
        events.push(TrackEvent {
            delta: u28::new(u32::try_from(tick.saturating_sub(previous)).unwrap_or(u32::MAX)),
            kind: TrackEventKind::Meta(message),
        });
        previous = tick;
    }
    events.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    events
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

/// One track per part, named, with its notes on one channel.
fn lane_track<'a>(
    lane: &'a GestureLane,
    performance: &GesturePlan,
    channel: u4,
    ticks: &Ticks,
    options: MidiOptions,
) -> Result<(Track<'a>, Vec<(u32, u8, EventId)>), RenderError> {
    // Absolute-tick messages first; deltas are a rendering of them.
    let mut absolute: Vec<(u64, u8, MidiMessage)> = Vec::new();
    // Where each written note-on came from, so a consumer never has to guess
    // which gesture a key at a tick was.
    let mut placed: Vec<(u32, u8, EventId)> = Vec::new();
    for occurrence in lane.track().occurrences() {
        let note = occurrence.payload();
        let lineage = lane.lineage(note.instance()).ok_or_else(|| RenderError::Unsupported {
            event: EventId(0),
            what: format!("gesture {} has no written lineage", note.instance()),
        })?;
        let key = midi_key(note.pitch()).ok_or_else(|| RenderError::Unsupported {
            event: lineage.event(),
            what: format!("written pitch {} is outside MIDI's range", note.pitch()),
        })?;
        // Score mode reads the written positions, not the performed span.
        // The part's groove is in the scheduled frame, and a
        // notation program handed a swung onset would draw triplets — which
        // is an engraver printing an interpretation, the thing the groove
        // module exists not to do.
        let (on, off, velocity) = match options.mode {
            MidiMode::Score => (
                reference_seconds(performance, lineage.written_on()),
                reference_seconds(performance, lineage.written_off()),
                NEUTRAL_VELOCITY,
            ),
            MidiMode::Performance => (
                lane.physical(occurrence.span().start()).as_ratio(),
                lane.physical(occurrence.span().end()).as_ratio(),
                velocity_of(note.amplitude()),
            ),
        };
        let onset = ticks.of(on);
        placed.push((u32::try_from(onset).unwrap_or(u32::MAX), key, lineage.event()));
        absolute.push((
            onset,
            // Note-on sorts after note-off at the same tick, so a repeated
            // pitch retriggers instead of being cut by its predecessor.
            1,
            MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(velocity),
            },
        ));
        absolute.push((
            ticks.of(off.max(on)),
            0,
            MidiMessage::NoteOff {
                key: u7::new(key),
                vel: u7::new(0),
            },
        ));
    }
    absolute.sort_by_key(|(tick, order, message)| (*tick, *order, message_key(*message)));

    placed.sort_unstable_by_key(|(tick, key, event)| (*tick, *key, event.0));

    let mut track = Track::new();
    // The file says which part each track is. Logic and GarageBand show this
    // name in their track headers, and a bundle of tracks called `Track 2` is
    // a bundle nobody can mix.
    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::TrackName(lane.name().as_bytes())),
    });
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
    Ok((track, placed))
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
    let number = i64::from(pitch.octave)
        .checked_add(1)?
        .checked_mul(12)?
        .checked_add(pitch.semitone())?;
    u8::try_from(number).ok().filter(|key| *key <= 127)
}

/// Abstract amplitude → velocity. Linear and documented rather than clever:
/// a curve here would be a second interpretation layer on top of the profile
/// that already made the choice.
fn velocity_of(amplitude: Ratio<i64>) -> u8 {
    let scaled = (ratio_to_f64(amplitude).clamp(0.0, 1.0) * 127.0).round() as u8;
    scaled.max(1)
}

fn reference_seconds(performance: &GesturePlan, at: MusicalTime) -> Ratio<i64> {
    performance
        .tempo()
        .physical(Position::<PerformedTime>::new(at.as_ratio()))
        .as_ratio()
}

fn ratio_to_f64(value: Ratio<i64>) -> f64 {
    *value.numer() as f64 / *value.denom() as f64
}
