//! The one MIDI decision path, before any container or any host.
//!
//! Everything MIDI is *decided* here: which key a written pitch becomes,
//! which velocity a dynamic becomes, when a note starts and stops in exact
//! physical seconds, which channel a part is allocated, what order
//! simultaneous messages go out in, and what the format could not carry.
//! What is left over — writing a Standard MIDI File, or handing packets to
//! `CoreMIDI` — is *transport*, and a transport reinterprets nothing.
//!
//! That split is the point. A live projection and a written file that each
//! made their own decisions would be two performances of one piece with one
//! name, and the difference would surface as a composer's bug report rather
//! than as a test. Here there is one schedule and two readings of it, and a
//! differential law can hold them to each other
//! (`docs/rules/across-stages/06-daw-boundary.md` §2).
//!
//! The schedule is host-neutral: exact rational seconds from the start of
//! the performance, never an operating-system timestamp. Turning seconds
//! into a host clock is the live adapter's job and happens at the edge
//! (§4: physical time is a fact of the performance edge, not a score value).

use musa_events::{PerformedTime, Position};
use musa_score::{EventId, GestureLane, GesturePlan, MusicalTime, WrittenPitch};
use num_rational::Ratio;

use crate::error::RenderError;

use super::{MidiMode, MidiOptions};

/// The velocity every note gets in [`MidiMode::Score`]: mezzo-forte, the
/// conventional neutral. Score MIDI states no opinion about loudness, and a
/// file full of velocity 127 is an opinion.
pub(super) const NEUTRAL_VELOCITY: u8 = 80;

/// How finely a gradual tempo change is sampled into constant segments, per
/// whole note of its reach.
///
/// SMF has no continuous tempo — a *rit.* is a run of set-tempo events or it
/// is nothing — so a density has to be chosen, and per `docs/rules/events/07`
/// it is chosen **here**, by the consumer, rather than by the map that holds
/// the normative shape. Thirty-two per whole note is a set-tempo every
/// thirty-second note: below the threshold at which a listener hears steps,
/// and small enough that an eight-bar riser costs a few hundred bytes.
pub(super) const TEMPO_STEPS_PER_WHOLE: u32 = 32;

/// How many channels a single MIDI cable has.
const CHANNELS: usize = 16;

/// Channel 10 (index 9) is percussion by convention, so parts skip it: a
/// tenth part silently rewritten to a drum kit is the kind of loss this
/// boundary exists to refuse.
const PERCUSSION: usize = 9;

/// The channels a melodic part may be given, in allocation order.
const MELODIC: usize = CHANNELS - 1;

/// One part of the schedule and the channel it was allocated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiPart {
    /// Which track of a Standard MIDI File this part is, counting the tempo
    /// track as zero. A live projection uses the same index to name its
    /// source, so the two agree about which part is which.
    pub track: usize,
    /// The part's name, as the score wrote it.
    pub name: String,
    /// The channel its messages carry, 0–15.
    pub channel: u8,
}

/// What the schedule states at one instant about the piece as a whole.
///
/// Tempo, meter, and key belong to the conductor rather than to a part.
/// A file writes them as one track; a live projection has nowhere to put
/// them and says so as a loss rather than inventing a place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MidiMeta {
    /// Microseconds per quarter note.
    Tempo(u32),
    /// A meter, if MIDI can spell it.
    Meter(musa_score::Meter),
    /// A key signature.
    Key(musa_score::Key),
}

/// One conductor's statement, at the moment it takes effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduledMeta {
    /// Exact seconds from the start of the performance.
    pub seconds: Ratio<i64>,
    /// The same instant in ticks, for a metrical file.
    pub tick: u64,
    /// A meter, then a key, then a tempo at one instant — the order a
    /// conductor reads them in, and the order every writer emits.
    pub rank: u8,
    pub what: MidiMeta,
}

/// One channel-voice message, at the moment it is played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduledMessage {
    /// Exact seconds from the start of the performance.
    pub seconds: Ratio<i64>,
    /// The same instant in ticks, for a metrical file.
    pub tick: u64,
    /// Which part, indexed into [`MidiSchedule::parts`].
    pub part: usize,
    /// The status byte with its channel already in the low nibble.
    pub status: u8,
    /// The one or two data bytes the status takes.
    pub data: [u8; 2],
    /// The source event this message came from, for a note-on.
    ///
    /// Only note-ons carry one: a note-off is the end of the same event, and
    /// naming it twice would make one note look like two origins.
    pub origin: Option<EventId>,
}

impl ScheduledMessage {
    /// Whether this message starts a note.
    #[must_use]
    pub const fn is_note_on(&self) -> bool {
        self.status & 0xF0 == 0x90
    }

    /// The channel it carries, 0–15.
    #[must_use]
    pub const fn channel(&self) -> u8 {
        self.status & 0x0F
    }

    /// The message as the three bytes a cable carries.
    #[must_use]
    pub const fn bytes(&self) -> [u8; 3] {
        [self.status, self.data[0], self.data[1]]
    }
}

/// One thing MIDI could not carry, named rather than discovered.
///
/// `06-daw-boundary.md` §6: a presentation may lose information; it may not
/// lose it silently. These are the losses the *schedule* knows about — the
/// ones that follow from allocating sixteen channels and one tempo track.
/// A boundary adds its own on top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiLoss {
    kind: String,
    message: String,
}

impl MidiLoss {
    /// A stable word for the class: `channel`, `polytempo`, `polymeter`,
    /// `tuning`, `controller`, or `notation`.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// What was lost, in this path's own words.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    pub(super) fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.to_owned(),
            message: message.into(),
        }
    }
}

/// Every MIDI decision this performance produces, in one immutable value.
///
/// Messages are sorted by instant, then by part, then by a total order among
/// simultaneous messages, so two readings of one schedule cannot disagree
/// about what happens first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiSchedule {
    mode: MidiMode,
    ticks_per_quarter: u16,
    parts: Vec<MidiPart>,
    meta: Vec<ScheduledMeta>,
    messages: Vec<ScheduledMessage>,
    losses: Vec<MidiLoss>,
}

impl MidiSchedule {
    /// Which document this is: the written reading or the played one.
    #[must_use]
    pub const fn mode(&self) -> MidiMode {
        self.mode
    }

    /// The tick grid the [`Self::messages`] ticks are counted on.
    #[must_use]
    pub const fn ticks_per_quarter(&self) -> u16 {
        self.ticks_per_quarter
    }

    /// Every sounding part, in track order, with the channel it was given.
    #[must_use]
    pub fn parts(&self) -> &[MidiPart] {
        &self.parts
    }

    /// Tempo, meter, and key, in the order a conductor reads them.
    #[must_use]
    pub fn meta(&self) -> &[ScheduledMeta] {
        &self.meta
    }

    /// Every channel-voice message, in the order it is played.
    #[must_use]
    pub fn messages(&self) -> &[ScheduledMessage] {
        &self.messages
    }

    /// What this projection could not carry.
    #[must_use]
    pub fn losses(&self) -> &[MidiLoss] {
        &self.losses
    }

    /// When the last message is played, in exact seconds.
    #[must_use]
    pub fn extent(&self) -> Ratio<i64> {
        self.messages
            .last()
            .map_or_else(|| Ratio::new(0, 1), |message| message.seconds)
    }
}

/// Decide every MIDI fact about one performance.
///
/// # Errors
/// [`RenderError::Unsupported`] when a written pitch falls outside MIDI's
/// 0–127 range, or when a gesture has no written lineage to place.
pub fn midi_schedule(performance: &GesturePlan, options: &MidiOptions) -> Result<MidiSchedule, RenderError> {
    let span = tracing::info_span!("midi_schedule", lanes = performance.lanes().len());
    let _entered = span.enter();
    let ticks = Ticks::new(performance, options.ticks_per_quarter);

    let mut parts = Vec::with_capacity(performance.lanes().len());
    let mut messages = Vec::new();
    let mut shared: Option<String> = None;
    for (index, lane) in performance.lanes().iter().enumerate() {
        let channel = channel_for(index);
        if index >= MELODIC && shared.is_none() {
            shared = Some(lane.name().to_owned());
        }
        parts.push(MidiPart {
            track: index.saturating_add(1),
            name: lane.name().to_owned(),
            channel,
        });
        lane_messages(lane, performance, index, channel, &ticks, *options, &mut messages)?;
    }
    // One total order, decided once. Both readings inherit it, which is what
    // makes "the same schedule" a checkable claim rather than a hope.
    messages.sort_by_key(|message| {
        (
            message.seconds,
            message.part,
            simultaneity(message.status),
            message.data[0],
            message.data[1],
        )
    });

    let mut losses = Vec::new();
    if let Some(shared) = shared {
        losses.push(MidiLoss::new(
            "channel",
            format!(
                "this piece has more parts than MIDI has melodic channels, so `{shared}` shares a \
                 channel with another part; a consumer reading channels rather than tracks or \
                 sources will merge them"
            ),
        ));
    }
    if performance.is_polytempo() {
        losses.push(MidiLoss::new(
            "polytempo",
            "MIDI states one tempo: the parts play at their own speeds and every message is placed \
             at the moment it sounds, but the tempo stated is the piece's and not theirs",
        ));
    }
    losses.push(MidiLoss::new(
        "notation",
        "MIDI carries note numbers: written spelling, ties, voices, and beams are not in this \
         projection at all",
    ));
    losses.push(MidiLoss::new(
        "tuning",
        "MIDI states no tuning, so a consumer plays these key numbers at whatever its own \
         instruments are tuned to",
    ));

    Ok(MidiSchedule {
        mode: options.mode,
        ticks_per_quarter: options.ticks_per_quarter,
        parts,
        meta: conductor(performance, &ticks),
        messages,
        losses,
    })
}

/// Which channel the nth part is given.
const fn channel_for(index: usize) -> u8 {
    let slot = index % MELODIC;
    let channel = if slot >= PERCUSSION { slot + 1 } else { slot };
    channel as u8
}

/// A total order among messages at one instant on one part.
///
/// Note-off before note-on, so a repeated pitch retriggers instead of being
/// cut by its own predecessor.
const fn simultaneity(status: u8) -> u8 {
    match status & 0xF0 {
        0x80 => 0,
        0x90 => 1,
        _ => 2,
    }
}

/// Every note of one part, as messages.
fn lane_messages(
    lane: &GestureLane,
    performance: &GesturePlan,
    part: usize,
    channel: u8,
    ticks: &Ticks,
    options: MidiOptions,
    into: &mut Vec<ScheduledMessage>,
) -> Result<(), RenderError> {
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
        // The part's groove is in the scheduled frame, and a notation program
        // handed a swung onset would draw triplets — which is an engraver
        // printing an interpretation, the thing the groove module exists not
        // to do.
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
        let end = off.max(on);
        into.push(ScheduledMessage {
            seconds: on,
            tick: ticks.of(on),
            part,
            status: 0x90 | (channel & 0x0F),
            data: [key, velocity],
            origin: Some(lineage.event()),
        });
        into.push(ScheduledMessage {
            seconds: end,
            tick: ticks.of(end),
            part,
            status: 0x80 | (channel & 0x0F),
            data: [key, 0],
            origin: None,
        });
    }
    Ok(())
}

/// The conductor's statements: every tempo, meter, and key at its instant.
fn conductor(performance: &GesturePlan, ticks: &Ticks) -> Vec<ScheduledMeta> {
    let mut meta: Vec<ScheduledMeta> = Vec::new();
    for segment in performance.tempo().segments(TEMPO_STEPS_PER_WHOLE) {
        let micros = (ratio_to_f64(segment.seconds_per_quarter) * 1_000_000.0)
            .round()
            .max(1.0) as u32;
        let seconds = segment.physical.as_ratio();
        meta.push(ScheduledMeta {
            seconds,
            tick: ticks.of(seconds),
            rank: 2,
            what: MidiMeta::Tempo(micros.min(0x00FF_FFFF)),
        });
    }
    for change in performance.meters() {
        let seconds = reference_seconds(performance, change.at);
        meta.push(ScheduledMeta {
            seconds,
            tick: ticks.of(seconds),
            rank: 0,
            what: MidiMeta::Meter(change.meter),
        });
    }
    for change in performance.keys() {
        let seconds = reference_seconds(performance, change.at);
        meta.push(ScheduledMeta {
            seconds,
            tick: ticks.of(seconds),
            rank: 1,
            what: MidiMeta::Key(change.key),
        });
    }
    meta.sort_by_key(|entry| (entry.tick, entry.rank));
    meta
}

/// Exact physical seconds → MIDI ticks, one segment per tempo.
///
/// The plan is scheduled in frames; a file is metrical, so ticks are beats
/// and the conversion factor changes wherever the tempo does. Each segment
/// carries the tick its first frame lands on, so a note after a tempo change
/// is placed against the tempo actually in force there rather than against
/// the tempo the piece started in.
pub(super) struct Ticks {
    segments: Vec<TickSegment>,
}

#[derive(Clone, Copy)]
struct TickSegment {
    seconds: Ratio<i64>,
    tick: u64,
    per_second: f64,
}

impl Ticks {
    pub(super) fn new(performance: &GesturePlan, ticks_per_quarter: u16) -> Self {
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

    pub(super) fn of(&self, seconds: Ratio<i64>) -> u64 {
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

pub(super) fn reference_seconds(performance: &GesturePlan, at: MusicalTime) -> Ratio<i64> {
    performance
        .tempo()
        .physical(Position::<PerformedTime>::new(at.as_ratio()))
        .as_ratio()
}

pub(super) fn ratio_to_f64(value: Ratio<i64>) -> f64 {
    *value.numer() as f64 / *value.denom() as f64
}
