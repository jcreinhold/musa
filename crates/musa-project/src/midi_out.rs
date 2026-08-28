//! Live MIDI output: the one schedule, played rather than written.
//!
//! `musa-notation` decides the MIDI — channels, velocities, ordering, the
//! moment of every message — and writes one reading of that decision as a
//! Standard MIDI File. This module takes the *same* schedule and hands it to
//! `musa-playback` as a run. Nothing between them reinterprets anything, so
//! what a workstation receives live and what it reads from `performance.mid`
//! are the same performance in two containers
//! (`docs/rules/across-stages/06-daw-boundary.md` §2).
//!
//! Exactly one side owns the clock (§4), and this is where that is settled
//! for a run: [`LiveMidiOptions::sync`] says which, and nothing opens until
//! [`musa_playback::SyncOptions::authority`] has agreed there is only one.
//! When Musa leads, the twenty-four-a-quarter pulses are read off the piece's
//! own exact tempo map here — the only place that map lives — and merged into
//! the same packet list as the notes, so one send loop walks one schedule.

// Microsecond arithmetic over 128-bit intermediates this module bounds
// itself; the one conversion that could leave that range is checked.
#![allow(clippy::arithmetic_side_effects)]

use musa_notation::{MidiMode, MidiOptions, MidiSchedule, midi_schedule};
use musa_playback::{
    ClockAuthority, LiveMidiPacket, LiveMidiPart, MidiOutput, MidiOutputConfig, MidiOutputMode, MidiOutputReport,
    MidiOutputTarget, MidiPortReport, PULSES_PER_QUARTER, SYNC_OPTIONS_VERSION, SyncOptions, SyncProtocol, SyncRefusal,
    transport_packets,
};
use musa_score::{IntegratedTempoMap, MusicalTime, Scope, ScoreSnapshot};
use num_rational::Ratio;

use crate::daw::DawLoss;
use crate::error::ProjectError;

/// What to send, and where.
#[derive(Clone, Debug)]
pub struct LiveMidiOptions {
    /// Which document to play: the neutral score reading, or the interpreted
    /// performance. They are different documents and Musa never conflates
    /// them (roadmap §12.5).
    pub mode: MidiMode,
    /// One published port a part, or one carrying every part.
    pub sources: MidiOutputMode,
    /// Virtual sources of Musa's own, or a destination the host offers.
    pub target: MidiOutputTarget,
    /// Which side owns the clock, and by which protocol.
    pub sync: SyncOptions,
}

impl Default for LiveMidiOptions {
    fn default() -> Self {
        Self {
            mode: MidiMode::Performance,
            sources: MidiOutputMode::default(),
            target: MidiOutputTarget::default(),
            sync: SyncOptions::default(),
        }
    }
}

/// One part of the run, and where its notes go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveMidiPartFacts {
    /// The part's name, as the score wrote it.
    pub name: String,
    /// The channel the schedule allocated it, zero-based.
    pub channel: u8,
    /// Which published port carries it.
    pub port: usize,
}

/// The run a schedule becomes, before any port exists.
///
/// This is what makes the differential law checkable without a device: it is
/// the whole live reading of a schedule, and every byte in it came from the
/// schedule the Standard MIDI writer reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveMidiProjection {
    parts: Vec<LiveMidiPart>,
    packets: Vec<LiveMidiPacket>,
    losses: Vec<DawLoss>,
    unsynchronized: Vec<String>,
}

impl LiveMidiProjection {
    /// The sounding parts, in schedule order.
    pub fn parts(&self) -> &[LiveMidiPart] {
        &self.parts
    }

    /// Every message, in the schedule's own order, placed in microseconds
    /// from the start of the run.
    pub fn packets(&self) -> &[LiveMidiPacket] {
        &self.packets
    }

    /// What the MIDI projection could not carry, in the words the schedule
    /// used. Live output loses exactly what the file loses; it does not lose
    /// less by being live.
    pub fn losses(&self) -> &[DawLoss] {
        &self.losses
    }
}

/// What starting a run published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveMidiReport {
    mode: MidiMode,
    output: MidiOutputReport,
    parts: Vec<LiveMidiPartFacts>,
    losses: Vec<DawLoss>,
    authority: Option<ClockAuthority>,
    protocol: Option<SyncProtocol>,
    reference: Option<String>,
    unsynchronized: Vec<String>,
}

impl LiveMidiReport {
    /// Which document is playing.
    pub const fn mode(&self) -> MidiMode {
        self.mode
    }

    /// Where the run sends, in words a person can read back.
    pub fn target(&self) -> &str {
        &self.output.target
    }

    /// Every published port, in order.
    pub fn ports(&self) -> &[MidiPortReport] {
        &self.output.ports
    }

    /// Each part, its channel, and the port it plays on.
    pub fn parts(&self) -> &[LiveMidiPartFacts] {
        &self.parts
    }

    /// What the projection could not carry.
    pub fn losses(&self) -> &[DawLoss] {
        &self.losses
    }

    /// The versioned shape of the synchronization options this run read.
    ///
    /// A consumer that stores a configuration needs to know which reading of
    /// it is in force, and gets told rather than inferring it from the fields
    /// that happen to be present.
    pub const fn sync_version(&self) -> u32 {
        SYNC_OPTIONS_VERSION
    }

    /// Which side owns the clock, or `None` when neither does and Musa
    /// simply plays.
    pub const fn authority(&self) -> Option<ClockAuthority> {
        self.authority
    }

    /// The synchronization protocol in force, if any.
    pub const fn protocol(&self) -> Option<SyncProtocol> {
        self.protocol
    }

    /// The named scope whose tempo the one clock states, for a polytempo
    /// piece.
    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }

    /// The scopes running at their own speeds that this clock does not
    /// state. Named rather than silently flattened: one MIDI clock lane
    /// cannot denote simultaneous independent tempos, and pretending
    /// otherwise is what §4 forbids.
    pub fn unsynchronized(&self) -> &[String] {
        &self.unsynchronized
    }

    /// What the protocol in force cannot represent, stated before the
    /// session starts rather than discovered by drift.
    pub fn limits(&self) -> Option<&'static str> {
        self.protocol.map(SyncProtocol::limits)
    }

    /// How near its moment a message lands, stated rather than assumed.
    pub fn timing(&self) -> String {
        format!(
            "each message is handed over as it comes due, within {} µs of its moment",
            self.output.window_micros
        )
    }
}

/// The most pulses one run will ever send.
///
/// Twenty-four a quarter is 172,800 an hour at a quarter of 120, so this is
/// hours of music and still a bound. A piece that reaches it is told, rather
/// than left to find out that the clock stopped.
const MAX_PULSES: i64 = 1_000_000;

/// Read one score as a live MIDI run.
///
/// When Musa leads, the transport stream is read off the piece's own tempo
/// map here and merged into the same packet list, so the notes and the clock
/// are one schedule and cannot disagree.
///
/// # Errors
/// [`ProjectError::Performance`] if the score's gestures do not lower,
/// [`ProjectError::Notation`] if the MIDI schedule cannot be decided, and
/// [`ProjectError::Sync`] if the clock authority is ambiguous or the piece is
/// polytempo with no reference scope named.
pub(crate) fn project(score: &ScoreSnapshot, options: &LiveMidiOptions) -> Result<LiveMidiProjection, ProjectError> {
    let authority = options
        .sync
        .authority()
        .map_err(|refusal| ProjectError::Sync(refusal.to_string()))?;
    let performance =
        musa_compiler::lower_gestures(score).map_err(|error| ProjectError::Performance(error.to_string()))?;
    let schedule = midi_schedule(
        &performance,
        &MidiOptions {
            mode: options.mode,
            ..MidiOptions::default()
        },
    )
    .map_err(|error| ProjectError::Notation(error.to_string()))?;
    let mut projection = read(&schedule);
    if authority != Some(ClockAuthority::Musa) {
        return Ok(projection);
    }
    let reference = reference_scope(score, &performance, options.sync.reference.as_deref())?;
    projection.unsynchronized = unsynchronized(score, &performance, options.sync.reference.as_deref());
    let extent = schedule.extent();
    let tempo = match reference {
        Some(scope) => IntegratedTempoMap::new(score, scope),
        None => performance.tempo().clone(),
    };
    let (pulses, capped) = pulse_micros(&tempo, extent);
    if capped {
        projection.losses.push(DawLoss::new(
            "clock-extent",
            "this run is longer than the transport stream Musa sends: the clock stops after \
             a million pulses and the music plays on",
        ));
    }
    let mut packets = transport_packets(MidiOutput::clock_part(&projection.parts), &pulses, micros(extent));
    packets.append(&mut projection.packets);
    // One list, in one order: the send loop reads it front to back.
    packets.sort_by_key(|packet| packet.micros);
    projection.packets = packets;
    Ok(projection)
}

/// The exact tempo map a follower reads a musical position through.
///
/// This is the whole conversion from MIDI clock to Musa's time: a pulse is
/// an exact number of quarters, and the piece's own map — the reference
/// scope's, where one is named — says what second that is. Nothing else in
/// the follower touches tempo.
///
/// # Errors
/// [`ProjectError::Performance`] if the score's gestures do not lower, and
/// [`ProjectError::Sync`] if the piece is polytempo with no reference named.
pub(crate) fn tempo_map(score: &ScoreSnapshot, reference: Option<&str>) -> Result<IntegratedTempoMap, ProjectError> {
    let performance =
        musa_compiler::lower_gestures(score).map_err(|error| ProjectError::Performance(error.to_string()))?;
    Ok(match reference_scope(score, &performance, reference)? {
        Some(scope) => IntegratedTempoMap::new(score, scope),
        None => performance.tempo().clone(),
    })
}

/// Exact quarters as whole microseconds through one tempo map.
pub(crate) fn quarters_micros(tempo: &IntegratedTempoMap, quarters: Ratio<i64>) -> u64 {
    micros(tempo.seconds_at(MusicalTime::new(quarters / 4)))
}

/// Exact seconds as whole microseconds, for a physical position a follower
/// read off MTC.
pub(crate) fn seconds_micros(seconds: Ratio<i64>) -> u64 {
    micros(seconds)
}

/// Move a run's packets to start at `from`, dropping what is already past.
///
/// This is what a seek or a loop boundary does to live output: the messages
/// before the boundary did not happen, and the ones after it are placed
/// against the new origin. The notes the old run left sounding are released
/// by the stop that precedes this, not by anything here.
pub(crate) fn from_micros(packets: &[LiveMidiPacket], from: u64) -> Vec<LiveMidiPacket> {
    packets
        .iter()
        .filter(|packet| packet.micros >= from)
        .map(|packet| LiveMidiPacket {
            micros: packet.micros.saturating_sub(from),
            ..*packet
        })
        .collect()
}

/// Which scope's tempo the one clock states, and the refusal when a
/// polytempo piece has not said.
///
/// A single clock lane denotes one tempo. A piece whose parts run at their
/// own speeds therefore has to name which one the workstation is following;
/// there is no honest default, so there is no default.
fn reference_scope(
    score: &ScoreSnapshot,
    performance: &musa_score::GesturePlan,
    reference: Option<&str>,
) -> Result<Option<Scope>, ProjectError> {
    let Some(name) = reference else {
        if performance.is_polytempo() {
            return Err(ProjectError::Sync(SyncRefusal::UnnamedReference.to_string()));
        }
        return Ok(None);
    };
    score
        .parts()
        .iter()
        .find(|(_, part)| part.name() == name)
        .map(|(id, _)| Some(Scope::Part { part: id.0 }))
        .ok_or_else(|| ProjectError::Sync(format!("this piece has no part named `{name}` to synchronize to")))
}

/// The scopes this clock does not state.
fn unsynchronized(
    score: &ScoreSnapshot,
    performance: &musa_score::GesturePlan,
    reference: Option<&str>,
) -> Vec<String> {
    if !performance.is_polytempo() {
        return Vec::new();
    }
    score
        .parts()
        .iter()
        .map(|(_, part)| part.name().to_owned())
        .filter(|name| Some(name.as_str()) != reference)
        .collect()
}

/// Where each of the twenty-four-a-quarter pulses falls, in microseconds.
///
/// Pulse `n` is at `n/96` of a whole note, and the tempo map — the one exact
/// thing in this conversion — says what second that is. The clock is
/// therefore a reading of the score's tempo and not a second opinion about it.
fn pulse_micros(tempo: &IntegratedTempoMap, extent: Ratio<i64>) -> (Vec<u64>, bool) {
    let per_whole = i64::try_from(PULSES_PER_QUARTER).unwrap_or(24) * 4;
    let mut pulses = Vec::new();
    for pulse in 0..=MAX_PULSES {
        let at = tempo.seconds_at(MusicalTime::new(Ratio::new(pulse, per_whole)));
        if at > extent {
            return (pulses, false);
        }
        pulses.push(micros(at));
    }
    (pulses, true)
}

/// The whole live reading of one schedule.
fn read(schedule: &MidiSchedule) -> LiveMidiProjection {
    LiveMidiProjection {
        parts: schedule
            .parts()
            .iter()
            .map(|part| LiveMidiPart {
                name: part.name.clone(),
                channel: part.channel,
            })
            .collect(),
        packets: schedule
            .messages()
            .iter()
            .map(|message| LiveMidiPacket {
                micros: micros(message.seconds),
                part: message.part,
                bytes: message.bytes(),
                len: 3,
            })
            .collect(),
        losses: schedule
            .losses()
            .iter()
            .map(|loss| DawLoss::new(loss.kind(), loss.message()))
            .collect(),
        unsynchronized: Vec::new(),
    }
}

/// Exact seconds as whole microseconds, rounded to the nearest.
///
/// This is the one place the run leaves exact time, and it leaves it at the
/// very edge — a port takes integers. Everything above it is still rational
/// (`constitution.md` §3).
fn micros(seconds: num_rational::Ratio<i64>) -> u64 {
    let numerator = i128::from(*seconds.numer()) * 1_000_000;
    let denominator = i128::from(*seconds.denom());
    if denominator == 0 {
        return 0;
    }
    let half = denominator / 2;
    let rounded = if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        (numerator - half) / denominator
    };
    u64::try_from(rounded).unwrap_or(0)
}

/// Turn one projection and one open output into the report a caller reads.
pub(crate) fn report(
    options: &LiveMidiOptions,
    projection: &LiveMidiProjection,
    output: &MidiOutputReport,
) -> LiveMidiReport {
    let parts = projection
        .parts
        .iter()
        .enumerate()
        .map(|(index, part)| LiveMidiPartFacts {
            name: part.name.clone(),
            channel: part.channel,
            port: output
                .ports
                .iter()
                .position(|port| port.parts.contains(&part.name))
                .unwrap_or_else(|| index.min(output.ports.len().saturating_sub(1))),
        })
        .collect();
    LiveMidiReport {
        mode: options.mode,
        output: output.clone(),
        parts,
        losses: projection.losses.clone(),
        authority: options.sync.authority().ok().flatten(),
        protocol: options.sync.protocol(),
        reference: options.sync.reference.clone(),
        unsynchronized: projection.unsynchronized.clone(),
    }
}

/// What one run needs before it opens.
pub(crate) fn config(name: &str, options: &LiveMidiOptions) -> MidiOutputConfig {
    MidiOutputConfig {
        mode: options.sources,
        target: options.target.clone(),
        client: format!("Musa — {}", piece(name)),
        clock: options.sync.authority().ok().flatten() == Some(ClockAuthority::Musa),
    }
}

/// The piece's name as a musician would say it.
///
/// A session opened from a path is named by that path, which is the right
/// answer for a diagnostic and the wrong one for a menu of MIDI inputs: what
/// belongs there is the piece, not where it happens to live.
fn piece(name: &str) -> &str {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    base.strip_suffix(".musa").unwrap_or(base)
}

#[cfg(test)]
mod boundary_laws {
    use super::{LiveMidiPacket, from_micros};

    fn packet(micros: u64, key: u8) -> LiveMidiPacket {
        LiveMidiPacket {
            micros,
            part: 0,
            bytes: [0x90, key, 80],
            len: 3,
        }
    }

    /// A seek or a loop boundary is a discontinuity: what is behind it did
    /// not happen, and what is ahead of it is placed against the new origin.
    /// The notes the old run left sounding are released by the stop that
    /// precedes this, which is why nothing here has to know about them.
    #[test]
    fn a_boundary_drops_what_is_past_and_moves_the_rest_to_the_front() {
        let run = [packet(0, 60), packet(500_000, 62), packet(750_000, 64)];
        let moved = from_micros(&run, 500_000);
        assert_eq!(
            moved.iter().map(|packet| packet.micros).collect::<Vec<_>>(),
            vec![0, 250_000]
        );
        assert_eq!(moved.first().map(|packet| packet.bytes[1]), Some(62));
        // A boundary past everything leaves nothing to restart.
        assert!(from_micros(&run, 1_000_000).is_empty());
        // A boundary at zero is the run itself.
        assert_eq!(from_micros(&run, 0), run);
    }
}
