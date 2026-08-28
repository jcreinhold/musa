//! The sync port: one chosen input, read for transport bytes and nothing else.
//!
//! This is a *second* connection, separate from [`crate::MidiInput`], and the
//! separation is deliberate twice over. A sync source is chosen separately
//! from a keyboard, and the keyboard connection deliberately asks its backend
//! to filter timing bytes out — which is exactly the traffic this one exists
//! to read. Sharing one connection would mean one of the two got the stream
//! it did not want.
//!
//! The callback does one thing: feed each byte to [`SyncParser`] and push what
//! comes back. No allocation, no lock, no arithmetic beyond a timestamp.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::MidiInputDevice;
use crate::sync::follower::SyncObservation;
use crate::sync::parser::SyncParser;

/// How many observations wait for the control side.
///
/// At 480 beats a minute MIDI clock is 192 pulses a second, so this is more
/// than twenty seconds of leader at a tempo no music uses. A control side
/// that falls this far behind has stopped ticking, and the counter says so.
const SYNC_CAPACITY: usize = 4_096;

const CLIENT: &str = "musa sync";

/// What the sync callback could not pass on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncInputLosses {
    /// Observations dropped because the control side was not draining.
    pub queue_overflow: u64,
}

/// A connection to one external clock source, or to none.
pub struct SyncInput {
    _connection: Option<midir::MidiInputConnection<()>>,
    observations: rtrb::Consumer<SyncObservation>,
    device: Option<MidiInputDevice>,
    losses: Arc<Losses>,
    opened: Instant,
}

#[derive(Default)]
struct Losses {
    queue_overflow: AtomicU64,
}

impl SyncInput {
    /// Connect to the source with this stable id, or to nothing when it is
    /// not present.
    ///
    /// A missing source is a normal empty connection rather than an error:
    /// the leader may simply not be running yet, and the follower's lock
    /// state already says it has heard nothing.
    #[must_use]
    pub fn open(id: &str) -> Self {
        let (mut producer, observations) = rtrb::RingBuffer::<SyncObservation>::new(SYNC_CAPACITY);
        let losses = Arc::new(Losses::default());
        let opened = Instant::now();
        let Some((mut input, port, device)) = choose(id) else {
            return Self {
                _connection: None,
                observations,
                device: None,
                losses,
                opened,
            };
        };
        // Timing bytes are the whole point of this connection, so `Ignore`
        // is asked for everything *except* them.
        input.ignore(midir::Ignore::Sysex | midir::Ignore::ActiveSense);
        let callback_losses = Arc::clone(&losses);
        let mut parser = SyncParser::new();
        let connection = input.connect(
            &port,
            CLIENT,
            move |device_micros, message, ()| {
                let micros = u64::try_from(opened.elapsed().as_micros()).unwrap_or(u64::MAX);
                for &byte in message {
                    let Some(message) = parser.feed(byte) else {
                        continue;
                    };
                    let observation = SyncObservation {
                        micros,
                        device_micros,
                        message,
                    };
                    if producer.push(observation).is_err() {
                        callback_losses.queue_overflow.fetch_add(1, Ordering::Relaxed);
                    }
                }
            },
            (),
        );
        match connection {
            Ok(connection) => Self {
                _connection: Some(connection),
                observations,
                device: Some(device),
                losses,
                opened,
            },
            Err(error) => {
                tracing::warn!(error = %error.kind(), "could not open the MIDI sync port");
                Self {
                    _connection: None,
                    observations,
                    device: None,
                    losses,
                    opened,
                }
            }
        }
    }

    /// The connected source, if its port opened.
    #[must_use]
    pub const fn device(&self) -> Option<&MidiInputDevice> {
        self.device.as_ref()
    }

    /// Microseconds since this port opened, on the same clock the callback
    /// stamps observations with. The control side needs it to ask the
    /// follower whether the leader has gone quiet.
    #[must_use]
    pub fn now(&self) -> u64 {
        u64::try_from(self.opened.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    /// Next observation, without blocking.
    pub fn poll(&mut self) -> Option<SyncObservation> {
        self.observations.pop().ok()
    }

    /// What the callback could not pass on.
    #[must_use]
    pub fn losses(&self) -> SyncInputLosses {
        SyncInputLosses {
            queue_overflow: self.losses.queue_overflow.load(Ordering::Relaxed),
        }
    }
}

fn choose(id: &str) -> Option<(midir::MidiInput, midir::MidiInputPort, MidiInputDevice)> {
    let input = midir::MidiInput::new(CLIENT).ok()?;
    let port = input.ports().into_iter().find(|port| port.id() == id)?;
    let device = MidiInputDevice {
        id: port.id(),
        name: input.port_name(&port).ok()?,
    };
    Some((input, port, device))
}

/// Device-free probe of the exact sync callback path.
///
/// The parser is the part under a real-time contract, so this drives it
/// through the same loop the callback runs and nothing else.
#[doc(hidden)]
pub struct SyncCallbackHarness {
    parser: SyncParser,
    producer: rtrb::Producer<SyncObservation>,
    consumer: rtrb::Consumer<SyncObservation>,
    losses: Losses,
}

impl SyncCallbackHarness {
    /// Allocate the ring before entering the measured path.
    #[must_use]
    pub fn new() -> Self {
        let (producer, consumer) = rtrb::RingBuffer::new(SYNC_CAPACITY);
        Self {
            parser: SyncParser::new(),
            producer,
            consumer,
            losses: Losses::default(),
        }
    }

    /// Read one backend callback message, exactly as the callback does.
    pub fn receive(&mut self, micros: u64, device_micros: u64, message: &[u8]) {
        for &byte in message {
            let Some(message) = self.parser.feed(byte) else {
                continue;
            };
            let observation = SyncObservation {
                micros,
                device_micros,
                message,
            };
            if self.producer.push(observation).is_err() {
                self.losses.queue_overflow.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Drain one observation on the simulated control side.
    pub fn poll(&mut self) -> Option<SyncObservation> {
        self.consumer.pop().ok()
    }

    /// What the callback could not pass on.
    #[must_use]
    pub fn losses(&self) -> SyncInputLosses {
        SyncInputLosses {
            queue_overflow: self.losses.queue_overflow.load(Ordering::Relaxed),
        }
    }
}

impl Default for SyncCallbackHarness {
    fn default() -> Self {
        Self::new()
    }
}
