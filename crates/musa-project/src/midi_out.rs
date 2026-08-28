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
//! Musa's transport is the clock authority for that run (§4). Following an
//! external clock is prompt 214's subject, not this one's.

// Microsecond arithmetic over 128-bit intermediates this module bounds
// itself; the one conversion that could leave that range is checked.
#![allow(clippy::arithmetic_side_effects)]

use musa_notation::{MidiMode, MidiOptions, MidiSchedule, midi_schedule};
use musa_playback::{
    LiveMidiPacket, LiveMidiPart, MidiOutputConfig, MidiOutputMode, MidiOutputReport, MidiOutputTarget, MidiPortReport,
};
use musa_score::ScoreSnapshot;

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
}

impl Default for LiveMidiOptions {
    fn default() -> Self {
        Self {
            mode: MidiMode::Performance,
            sources: MidiOutputMode::default(),
            target: MidiOutputTarget::default(),
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

    /// How near its moment a message lands, stated rather than assumed.
    pub fn timing(&self) -> String {
        format!(
            "each message is handed over as it comes due, within {} µs of its moment",
            self.output.window_micros
        )
    }
}

/// Read one score as a live MIDI run.
///
/// # Errors
/// [`ProjectError::Performance`] if the score's gestures do not lower, and
/// [`ProjectError::Notation`] if the MIDI schedule cannot be decided.
pub(crate) fn project(score: &ScoreSnapshot, mode: MidiMode) -> Result<LiveMidiProjection, ProjectError> {
    let performance =
        musa_compiler::lower_gestures(score).map_err(|error| ProjectError::Performance(error.to_string()))?;
    let schedule = midi_schedule(
        &performance,
        &MidiOptions {
            mode,
            ..MidiOptions::default()
        },
    )
    .map_err(|error| ProjectError::Notation(error.to_string()))?;
    Ok(read(&schedule))
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
            })
            .collect(),
        losses: schedule
            .losses()
            .iter()
            .map(|loss| DawLoss::new(loss.kind(), loss.message()))
            .collect(),
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
pub(crate) fn report(mode: MidiMode, projection: &LiveMidiProjection, output: &MidiOutputReport) -> LiveMidiReport {
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
        mode,
        output: output.clone(),
        parts,
        losses: projection.losses.clone(),
    }
}

/// What one run needs before it opens.
pub(crate) fn config(name: &str, options: &LiveMidiOptions) -> MidiOutputConfig {
    MidiOutputConfig {
        mode: options.sources,
        target: options.target.clone(),
        client: format!("Musa — {}", piece(name)),
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
