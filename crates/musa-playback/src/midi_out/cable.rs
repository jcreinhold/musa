//! What live MIDI output needs from a platform, and a platform that is not one.
//!
//! Two traits, both tiny. [`Clock`] answers "what time is it" in microseconds
//! from an unspecified but monotonic epoch; [`Cable`] takes one bounded batch
//! of already-decided channel messages and puts them on a port. Everything
//! musical — routing, ordering, lateness, the panic sequence — is decided by
//! [`super::sender::Sender`] above these, so the same decisions run on macOS
//! and in a test with no device attached.

use std::time::Instant;

/// One monotonic microsecond clock.
pub(crate) trait Clock: Send {
    /// Microseconds since this clock's own epoch. Never decreases.
    fn now(&self) -> u64;
}

/// The process clock: `Instant`, which is monotonic on every platform Musa
/// builds for.
pub(crate) struct ProcessClock {
    opened: Instant,
}

impl ProcessClock {
    pub(crate) fn new() -> Self {
        Self { opened: Instant::now() }
    }
}

impl Clock for ProcessClock {
    fn now(&self) -> u64 {
        u64::try_from(self.opened.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

/// One message as the wire carries it.
///
/// The length matters and cannot be inferred from the status alone once the
/// stream carries transport bytes: a clock is one byte, a song position is
/// three, and a note is three. Sending a one-byte message padded to three
/// would put two stray data bytes on the cable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Wire {
    /// Status and up to two data bytes.
    pub(crate) bytes: [u8; 3],
    /// How many of them are on the wire: one, two, or three.
    pub(crate) len: u8,
}

impl Wire {
    /// The bytes actually sent.
    pub(crate) fn on_the_wire(&self) -> &[u8] {
        let len = (self.len as usize).min(3);
        self.bytes.get(..len).unwrap_or(&self.bytes)
    }
}

/// A batch of messages, each stamped in the clock's microseconds.
///
/// The stamp is what the port should deliver the message *at*. A backend that
/// can hand the host a future timestamp uses it; one that cannot delivers the
/// batch on arrival, which is why [`super::MidiOutputReport::window_micros`]
/// says how near its moment a message actually lands.
pub(crate) type Batch = [(u64, Wire)];

/// A port refused a batch: it vanished, or it was never opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Refused;

/// One platform's ports.
pub(crate) trait Cable: Send {
    /// Put one bounded batch on one port.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the platform refused the batch — a vanished
    /// endpoint, or a port index this cable never opened. The caller counts
    /// the refusal; it never retries inside the send loop.
    fn send(&mut self, port: usize, batch: &Batch) -> Result<(), Refused>;
}
