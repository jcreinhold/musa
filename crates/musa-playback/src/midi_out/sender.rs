//! The live send loop, with no platform and no thread in it.
//!
//! A [`Sender`] holds one immutable schedule and walks it against a clock.
//! Everything it decides is decided here and nowhere else: which port and
//! channel a part sounds on, which messages are due inside the next window,
//! what happens to one that came due while we were asleep, and which notes a
//! panic has to release. A test drives [`Sender::pump`] with a fake clock and
//! a fake cable and sees exactly what macOS would see.
//!
//! Nothing in [`Sender::pump`] allocates. The staging buffers are sized at
//! construction and only ever cleared and refilled, which is what makes the
//! per-window refill bounded rather than merely usually small.

// Microsecond and index arithmetic over values this module bounds itself.
#![allow(clippy::arithmetic_side_effects)]

use crate::midi_out::cable::{Cable, Refused};
use crate::midi_out::{LiveMidiPacket, MidiOutputCounters};

/// How many messages one port accepts in one window.
///
/// A window that would exceed this leaves the remainder for the next one
/// rather than growing a buffer inside the send loop. At 256 messages every
/// two milliseconds this is far above any music and far below any allocation.
pub(crate) const BATCH: usize = 256;

/// Controller numbers the panic sequence uses.
const ALL_SOUND_OFF: u8 = 120;
const SUSTAIN: u8 = 64;
const ALL_NOTES_OFF: u8 = 123;

/// Where one part's messages go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Route {
    /// Which port of the cable.
    pub(crate) port: usize,
    /// Which channel on it.
    pub(crate) channel: u8,
}

/// Which notes are sounding, so a panic can release exactly those.
///
/// One `u128` a channel, one bit a key: sixteen channels of 128 keys is 256
/// bytes a port, allocated once and never grown.
struct Sounding {
    bits: Vec<[u128; CHANNELS]>,
}

/// MIDI 1.0 has sixteen channels, and this module never pretends otherwise.
const CHANNELS: usize = 16;

impl Sounding {
    fn new(ports: usize) -> Self {
        Self {
            bits: vec![[0u128; CHANNELS]; ports],
        }
    }

    fn set(&mut self, port: usize, channel: u8, key: u8, on: bool) {
        if let Some(word) = self
            .bits
            .get_mut(port)
            .and_then(|words| words.get_mut(usize::from(channel)))
        {
            if on {
                *word |= 1u128 << u32::from(key);
            } else {
                *word &= !(1u128 << u32::from(key));
            }
        }
    }

    fn is_silent(&self) -> bool {
        self.bits.iter().flatten().all(|word| *word == 0)
    }

    /// Take one sounding key off `channel` of `port`, or `None` when that
    /// channel is already quiet.
    fn take(&mut self, port: usize, channel: usize) -> Option<u8> {
        let word = self.bits.get_mut(port)?.get_mut(channel)?;
        if *word == 0 {
            return None;
        }
        let key = word.trailing_zeros();
        *word &= !(1u128 << key);
        Some(key as u8)
    }
}

/// Running totals, read from the control side while the loop runs.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Totals {
    pub(crate) sent: u64,
    pub(crate) late: u64,
    pub(crate) dropped: u64,
    pub(crate) refused: u64,
}

impl From<Totals> for MidiOutputCounters {
    fn from(totals: Totals) -> Self {
        Self {
            sent: totals.sent,
            late: totals.late,
            dropped: totals.dropped,
            refused: totals.refused,
        }
    }
}

/// One run of one schedule.
pub(crate) struct Sender {
    packets: Vec<LiveMidiPacket>,
    routes: Vec<Route>,
    next: usize,
    origin: Option<u64>,
    window: u64,
    tolerance: u64,
    staged: Vec<Vec<(u64, [u8; 3])>>,
    sounding: Sounding,
    totals: Totals,
}

impl Sender {
    /// Prepare a run. Every buffer this loop will ever need is sized here.
    pub(crate) fn new(
        packets: Vec<LiveMidiPacket>,
        routes: Vec<Route>,
        ports: usize,
        window: u64,
        tolerance: u64,
    ) -> Self {
        Self {
            packets,
            routes,
            next: 0,
            origin: None,
            window,
            tolerance,
            staged: (0..ports).map(|_| Vec::with_capacity(BATCH)).collect(),
            sounding: Sounding::new(ports),
            totals: Totals::default(),
        }
    }

    /// Cumulative facts about this run.
    pub(crate) const fn totals(&self) -> Totals {
        self.totals
    }

    /// Whether every message has been handed to the cable.
    pub(crate) fn is_drained(&self) -> bool {
        self.next >= self.packets.len()
    }

    /// Whether nothing is left to do: drained, and nothing left sounding.
    pub(crate) fn is_finished(&self) -> bool {
        self.is_drained() && self.sounding.is_silent()
    }

    /// Hand the cable everything due before `now + window`.
    ///
    /// The first pump fixes the run's origin, so `micros` in the schedule is
    /// read as "this long after the first pump" and the caller never has to
    /// pre-compute a start time it does not yet have.
    pub(crate) fn pump(&mut self, cable: &mut dyn Cable, now: u64) {
        let origin = *self.origin.get_or_insert(now);
        let horizon = now.saturating_add(self.window);
        for staged in &mut self.staged {
            staged.clear();
        }

        while let Some(packet) = self.packets.get(self.next) {
            let due = origin.saturating_add(packet.micros);
            if due > horizon {
                break;
            }
            let Some(&Route { port, channel }) = self.routes.get(packet.part) else {
                // A packet naming a part this run never routed is not music
                // we can place; it is counted, not guessed at.
                self.totals.dropped += 1;
                self.next += 1;
                continue;
            };
            let Some(staged) = self.staged.get_mut(port) else {
                self.totals.dropped += 1;
                self.next += 1;
                continue;
            };
            if staged.len() >= BATCH {
                // The window is full. The rest waits rather than growing a
                // buffer here: the loop is bounded, so the music arrives one
                // window later instead of one allocation later.
                break;
            }

            let behind = now.saturating_sub(due);
            let starts_a_note = packet.bytes[0] & 0xF0 == 0x90 && packet.bytes[2] > 0;
            if behind > self.tolerance && starts_a_note {
                // The one policy: a note whose moment is long past is not
                // played late, because a late attack is wrong music. A
                // release always goes out, because a hung note is worse than
                // a late one.
                self.totals.dropped += 1;
                self.next += 1;
                continue;
            }
            if behind > 0 {
                self.totals.late += 1;
            }
            let at = due.max(now);
            staged.push((at, packet.bytes));
            self.remember(port, channel, packet.bytes);
            self.next += 1;
        }

        self.flush(cable);
    }

    /// Release everything sounding, then quiet every channel in use.
    ///
    /// Bounded by construction: one note-off for each bit the run actually
    /// set — at most sixteen channels of 128 keys a port — and then three
    /// controllers for each channel a route names. The staging buffer is
    /// flushed whenever it fills, so a panic never grows one either.
    pub(crate) fn panic(&mut self, cable: &mut dyn Cable, now: u64) {
        for staged in &mut self.staged {
            staged.clear();
        }
        for port in 0..self.staged.len() {
            for channel in 0..CHANNELS {
                while let Some(key) = self.sounding.take(port, channel) {
                    self.stage(cable, port, now, [0x80 | channel as u8, key, 0]);
                }
            }
        }
        for index in 0..self.routes.len() {
            let Some(&Route { port, channel }) = self.routes.get(index) else {
                continue;
            };
            for controller in [ALL_SOUND_OFF, SUSTAIN, ALL_NOTES_OFF] {
                self.stage(cable, port, now, [0xB0 | channel, controller, 0]);
            }
        }
        self.flush(cable);
        self.next = self.packets.len();
    }

    /// Put one message on a port, sending the batch first if it is full.
    fn stage(&mut self, cable: &mut dyn Cable, port: usize, now: u64, bytes: [u8; 3]) {
        let full = match self.staged.get_mut(port) {
            Some(staged) => {
                staged.push((now, bytes));
                staged.len() >= BATCH
            }
            None => false,
        };
        if full {
            self.flush_port(cable, port);
        }
    }

    fn remember(&mut self, port: usize, channel: u8, bytes: [u8; 3]) {
        match bytes[0] & 0xF0 {
            0x90 if bytes[2] > 0 => self.sounding.set(port, channel, bytes[1], true),
            0x80 | 0x90 => self.sounding.set(port, channel, bytes[1], false),
            _ => {}
        }
    }

    fn flush(&mut self, cable: &mut dyn Cable) {
        for port in 0..self.staged.len() {
            self.flush_port(cable, port);
        }
    }

    fn flush_port(&mut self, cable: &mut dyn Cable, port: usize) {
        let Some(staged) = self.staged.get_mut(port) else {
            return;
        };
        if staged.is_empty() {
            return;
        }
        let count = staged.len() as u64;
        let outcome = cable.send(port, staged);
        staged.clear();
        match outcome {
            Ok(()) => self.totals.sent += count,
            Err(Refused) => self.totals.refused += count,
        }
    }
}
