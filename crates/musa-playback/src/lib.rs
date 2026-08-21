//! The audio engine: CPAL output stream lifecycle, the real-time command
//! boundary, and transport.
//!
//! Owns: device negotiation, the output stream, `rtrb` command queues, the
//! audio callback, transport state, and live MIDI input. Must never contain: score or
//! language types in signatures (it executes prepared plans), GUI concerns,
//! or real-time violations — the callback never allocates, locks, does I/O,
//! logs, or destroys large objects.
//!
//! Facade: [`AudioEngine::open`], [`AudioEngine::install`],
//! [`AudioEngine::command`], and [`MidiInput`] for live keyboard input. The
//! rest of the application never sees a CPAL stream, stream config, sample
//! format, or midir connection.

mod core;
mod engine;
mod error;
mod midi;

pub use crate::core::{PreparedPlaybackPlan, TransportCommand};
pub use crate::engine::{AudioEngine, EngineConfig};
pub use crate::error::EngineError;
pub use crate::midi::{MidiInput, MidiInputEvent};

#[doc(hidden)]
pub mod testing {
    //! RT-contract test hooks: drive the callback core
    //! against a fake output without an audio device. Not part of the
    //! facade; hidden so no real caller depends on it.
    pub use crate::core::{CallbackCore, Message};
}
