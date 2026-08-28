//! The control side of following: smoothing, tempo, drift, and lock.
//!
//! Nothing here runs in a callback. The parser hands up raw observations,
//! each carrying the microsecond it arrived at, and this is where they become
//! a tempo estimate, a jitter figure, a lock state, and — sparingly — an
//! instruction to Musa's own transport. That split is the prompt's rule and
//! §4's: the callback reads bytes, the control side decides.
//!
//! Two protocols arrive here and they are *not* the same kind of fact:
//!
//! - **MIDI clock** counts twenty-four pulses to a quarter note. It is
//!   musical, so its position is exact quarters and Musa reads it through
//!   the piece's own tempo map.
//! - **MTC** counts frames of wall-clock time. It is physical, so its
//!   position is exact seconds and it never becomes a musical value
//!   (`docs/rules/across-stages/06-daw-boundary.md` §4).
//!
//! [`SyncPosition`] keeps those two apart all the way to the caller, which is
//! the whole conversion policy: neither protocol is converted into the other,
//! and only the score's tempo map turns quarters into time.

// Microsecond and pulse arithmetic over values this module bounds itself.
#![allow(clippy::arithmetic_side_effects)]

use num_rational::Ratio;

use crate::sync::SyncProtocol;
use crate::sync::parser::SyncMessage;

/// Twenty-four pulses to the quarter, as MIDI has counted since 1983.
pub const PULSES_PER_QUARTER: u64 = 24;

/// How many pulses of steady clock a lock is worth claiming after.
const LOCK_PULSES: u64 = PULSES_PER_QUARTER;

/// How long a running follower waits for the next pulse before calling it a
/// dropout. Half a second is four beats at 480 and one at 120: long enough
/// that a stall is real, short enough that a stopped leader is noticed.
const DROPOUT_MICROS: u64 = 500_000;

/// How many pulse intervals the tempo estimate and the jitter figure read.
const WINDOW: usize = PULSES_PER_QUARTER as usize;

/// How far Musa may sit from the leader before it re-seeks rather than
/// letting the difference stand.
///
/// A resynchronization is audible — it flushes and restarts — so it is worth
/// doing only when the alternative is worse. Thirty milliseconds is roughly
/// where a listener starts to hear two attacks as separate.
pub const RESYNC_MICROS: u64 = 30_000;

/// Two frames, which is how far behind the wire a completed MTC reading is.
const MTC_LAG_FRAMES: i64 = 2;

/// How close the follower is to the leader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SyncLock {
    /// Nothing has been heard yet.
    #[default]
    Idle,
    /// Something is arriving, but not enough of it to trust.
    Acquiring,
    /// The leader is steady and Musa is following it.
    Locked,
    /// The leader stopped arriving. Musa stopped rather than free-running.
    Lost,
}

/// Where the leader says it is — in the units its protocol actually has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncPosition {
    /// Exact quarter notes, from MIDI clock or a song-position pointer.
    /// Musical: the piece's tempo map says what second this is.
    Quarters(Ratio<i64>),
    /// Exact seconds, from MTC. Physical: it is a fact about the leader's
    /// wall clock and never enters a score value.
    Seconds(Ratio<i64>),
}

/// What following the leader asks Musa's transport to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncIntent {
    /// Go to this position and play, flushing whatever was sounding.
    Start(SyncPosition),
    /// Play on from where the follower already is.
    Continue,
    /// Stop, and release what the run left sounding.
    Stop,
    /// Move without changing whether the transport is rolling.
    Seek(SyncPosition),
}

/// What the follower has measured.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyncStatus {
    /// The versioned shape of this reading.
    pub version: u32,
    /// Which protocol is being followed.
    pub protocol: SyncProtocol,
    /// How close the follower is to the leader.
    pub lock: SyncLock,
    /// Whether the leader says it is rolling.
    pub running: bool,
    /// Estimated leader tempo in beats per minute, once enough pulses have
    /// been seen. `None` under MTC, which carries no tempo at all.
    pub tempo_bpm: Option<f64>,
    /// Where the leader last said it was.
    pub position: Option<SyncPosition>,
    /// Measured departure from the rate the lock was taken at, in
    /// microseconds. Signed: positive means the leader is running late.
    pub drift_micros: i64,
    /// Greatest departure of one pulse interval from the window's mean.
    pub jitter_micros: u64,
    /// Observations admitted since the follower opened.
    pub observations: u64,
    /// Times the leader stopped arriving while it was rolling.
    pub dropouts: u64,
}

/// One raw observation, exactly as the callback saw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncObservation {
    /// Microseconds since the sync port opened, stamped in the callback.
    pub micros: u64,
    /// The backend's own timestamp, kept unrounded and uninterpreted.
    pub device_micros: u64,
    /// What the parser read.
    pub message: SyncMessage,
}

/// The version of [`SyncStatus`] this build reports.
pub const SYNC_STATUS_VERSION: u32 = 1;

/// Turns one external leader's stream into Musa transport intents.
#[derive(Clone, Debug)]
pub struct TransportFollower {
    protocol: SyncProtocol,
    lock: SyncLock,
    running: bool,
    /// Pulses since the position origin, under MIDI clock.
    pulses: u64,
    /// Where the position origin is, in pulses, as a song-position set it.
    origin_pulses: u64,
    intervals: [u64; WINDOW],
    filled: usize,
    cursor: usize,
    /// Whether a previous pulse's moment is known, so the gap to this one is
    /// an interval and not the distance from a transport message.
    primed: bool,
    last_micros: u64,
    /// The rate and the moment the current lock was taken at.
    locked: Option<(u64, u64, u64)>,
    drift_micros: i64,
    jitter_micros: u64,
    observations: u64,
    dropouts: u64,
    /// MTC nibbles, and which of them have arrived since the last complete
    /// reading.
    nibbles: [u8; 8],
    seen: u8,
    seconds: Option<Ratio<i64>>,
}

impl TransportFollower {
    /// A follower of one protocol, before anything has arrived.
    #[must_use]
    pub const fn new(protocol: SyncProtocol) -> Self {
        Self {
            protocol,
            lock: SyncLock::Idle,
            running: false,
            pulses: 0,
            origin_pulses: 0,
            intervals: [0; WINDOW],
            filled: 0,
            cursor: 0,
            primed: false,
            last_micros: 0,
            locked: None,
            drift_micros: 0,
            jitter_micros: 0,
            observations: 0,
            dropouts: 0,
            nibbles: [0; 8],
            seen: 0,
            seconds: None,
        }
    }

    /// What the follower has measured so far.
    #[must_use]
    pub fn status(&self) -> SyncStatus {
        SyncStatus {
            version: SYNC_STATUS_VERSION,
            protocol: self.protocol,
            lock: self.lock,
            running: self.running,
            tempo_bpm: self.tempo_bpm(),
            position: self.position(),
            drift_micros: self.drift_micros,
            jitter_micros: self.jitter_micros,
            observations: self.observations,
            dropouts: self.dropouts,
        }
    }

    /// Where the leader last said it was, in its own units.
    #[must_use]
    pub fn position(&self) -> Option<SyncPosition> {
        match self.protocol {
            SyncProtocol::MidiClock => Some(SyncPosition::Quarters(Ratio::new(
                i64::try_from(self.origin_pulses + self.pulses).unwrap_or(i64::MAX),
                i64::try_from(PULSES_PER_QUARTER).unwrap_or(24),
            ))),
            SyncProtocol::MidiTimecode => self.seconds.map(SyncPosition::Seconds),
        }
    }

    /// Admit one observation, and answer with what it asks the transport to
    /// do — which for the overwhelming majority of them is nothing.
    pub fn observe(&mut self, observation: SyncObservation) -> Option<SyncIntent> {
        self.observations += 1;
        let micros = observation.micros;
        match observation.message {
            SyncMessage::Clock => {
                self.pulse(micros);
                None
            }
            SyncMessage::Start => {
                self.last_micros = micros;
                self.restart();
                self.running = true;
                self.lock = SyncLock::Acquiring;
                Some(SyncIntent::Start(SyncPosition::Quarters(Ratio::ZERO)))
            }
            SyncMessage::Continue => {
                self.last_micros = micros;
                self.running = true;
                self.lock = SyncLock::Acquiring;
                Some(SyncIntent::Continue)
            }
            SyncMessage::Stop => {
                self.running = false;
                self.lock = SyncLock::Idle;
                self.locked = None;
                Some(SyncIntent::Stop)
            }
            SyncMessage::SongPosition(beats) => {
                // A MIDI beat is a sixteenth, so six pulses.
                self.origin_pulses = u64::from(beats) * (PULSES_PER_QUARTER / 4);
                self.pulses = 0;
                self.locked = None;
                self.position().map(SyncIntent::Seek)
            }
            SyncMessage::QuarterFrame { piece, value } => self.quarter_frame(micros, piece, value),
        }
    }

    /// Ask whether the leader has gone quiet. Called on the control side's
    /// own tick, so a leader that simply stops sending is noticed rather
    /// than waited on forever.
    pub fn idle_at(&mut self, micros: u64) -> Option<SyncIntent> {
        if !self.running || micros.saturating_sub(self.last_micros) <= DROPOUT_MICROS {
            return None;
        }
        self.running = false;
        self.lock = SyncLock::Lost;
        self.locked = None;
        self.dropouts += 1;
        Some(SyncIntent::Stop)
    }

    /// Whether this follower has heard a steady leader.
    #[must_use]
    pub const fn is_locked(&self) -> bool {
        matches!(self.lock, SyncLock::Locked)
    }

    fn restart(&mut self) {
        self.pulses = 0;
        self.origin_pulses = 0;
        self.filled = 0;
        self.cursor = 0;
        self.primed = false;
        self.locked = None;
        self.drift_micros = 0;
        self.jitter_micros = 0;
    }

    fn pulse(&mut self, micros: u64) {
        let interval = micros.saturating_sub(self.last_micros);
        let primed = self.primed;
        self.primed = true;
        self.last_micros = micros;
        if !self.running {
            // Some leaders send clock while stopped. It still tells us the
            // tempo; it does not move the position.
            if primed {
                self.admit(interval);
            }
            return;
        }
        self.pulses += 1;
        if primed {
            self.admit(interval);
        }
        if self.filled >= LOCK_PULSES as usize {
            if self.locked.is_none() {
                self.locked = Some((self.mean(), micros, self.pulses));
                self.lock = SyncLock::Locked;
            }
            if let Some((rate, at, pulse)) = self.locked {
                let expected = at + rate * self.pulses.saturating_sub(pulse);
                self.drift_micros =
                    i64::try_from(micros).unwrap_or(i64::MAX) - i64::try_from(expected).unwrap_or(i64::MAX);
            }
        } else if matches!(self.lock, SyncLock::Idle | SyncLock::Lost) {
            self.lock = SyncLock::Acquiring;
        }
    }

    fn admit(&mut self, interval: u64) {
        if let Some(slot) = self.intervals.get_mut(self.cursor) {
            *slot = interval;
        }
        self.cursor = (self.cursor + 1) % WINDOW;
        self.filled = (self.filled + 1).min(WINDOW);
        let mean = self.mean();
        self.jitter_micros = self
            .intervals
            .iter()
            .take(self.filled)
            .map(|value| value.abs_diff(mean))
            .max()
            .unwrap_or(0);
    }

    fn mean(&self) -> u64 {
        if self.filled == 0 {
            return 0;
        }
        let total: u64 = self.intervals.iter().take(self.filled).sum();
        total / self.filled as u64
    }

    fn tempo_bpm(&self) -> Option<f64> {
        if matches!(self.protocol, SyncProtocol::MidiTimecode) || self.filled < 2 {
            return None;
        }
        let mean = self.mean();
        if mean == 0 {
            return None;
        }
        Some(60_000_000.0 / (mean as f64 * PULSES_PER_QUARTER as f64))
    }

    /// Assemble one nibble of a timecode reading.
    ///
    /// Eight of them make `hh:mm:ss:ff` and a frame rate, spread over two
    /// frames of the leader's own time. A reading is only used when all
    /// eight arrived in order, which is what makes a stream joined halfway
    /// through wait for the next whole one instead of seeking to a value
    /// half of which is stale.
    fn quarter_frame(&mut self, micros: u64, piece: u8, value: u8) -> Option<SyncIntent> {
        self.last_micros = micros;
        if piece == 0 {
            self.seen = 0;
        } else if self.seen != piece {
            // Out of order, or joined mid-reading: wait for the next piece 0.
            self.seen = u8::MAX;
            return None;
        }
        if let Some(slot) = self.nibbles.get_mut(usize::from(piece)) {
            *slot = value & 0x0F;
        }
        self.seen = piece.saturating_add(1);
        if self.seen < 8 {
            return None;
        }
        self.seen = 0;
        let seconds = self.timecode()?;
        let was = self.seconds.replace(seconds);
        self.running = true;
        if matches!(self.lock, SyncLock::Idle | SyncLock::Lost) {
            self.lock = SyncLock::Acquiring;
            return Some(SyncIntent::Start(SyncPosition::Seconds(seconds)));
        }
        self.lock = SyncLock::Locked;
        // MTC arrives four times a second. Re-seeking on every reading would
        // be a stutter, so a reading that lands where the last one predicted
        // is a confirmation and not an instruction.
        // Eight quarter frames span two frames of the leader's own time,
        // which is where the next whole reading is expected to land.
        let expected = was? + self.frame_seconds()? * Ratio::from_integer(2);
        let apart = (seconds - expected) * Ratio::from_integer(1_000_000);
        if apart.to_integer().unsigned_abs() > RESYNC_MICROS {
            return Some(SyncIntent::Seek(SyncPosition::Seconds(seconds)));
        }
        None
    }

    /// Exact seconds per frame at the rate the reading declares.
    fn frame_seconds(&self) -> Option<Ratio<i64>> {
        Some(match (self.nibbles[7] >> 1) & 0x03 {
            0 => Ratio::new(1, 24),
            1 => Ratio::new(1, 25),
            // 29.97 drop-frame: 30000/1001 frames a second, exactly.
            2 => Ratio::new(1_001, 30_000),
            _ => Ratio::new(1, 30),
        })
    }

    /// The assembled reading, in exact seconds, advanced by the two frames
    /// the wire is behind by the time the eighth nibble lands.
    fn timecode(&self) -> Option<Ratio<i64>> {
        let frame = i64::from(self.nibbles[0] | (self.nibbles[1] & 0x03) << 4);
        let second = i64::from(self.nibbles[2] | (self.nibbles[3] & 0x07) << 4);
        let minute = i64::from(self.nibbles[4] | (self.nibbles[5] & 0x07) << 4);
        let hour = i64::from(self.nibbles[6] | (self.nibbles[7] & 0x01) << 4);
        let per_frame = self.frame_seconds()?;
        let whole = hour * 3_600 + minute * 60 + second;
        Some(Ratio::from_integer(whole) + per_frame * Ratio::from_integer(frame + MTC_LAG_FRAMES))
    }
}
