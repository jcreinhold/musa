//! The audio engine: CPAL output stream lifecycle, the real-time command
//! boundary, and transport (roadmap §13.1–§13.2, §15.6).
//!
//! Owns: device negotiation, the output stream, `rtrb` command queues, the
//! audio callback, and transport state. Must never contain: score or
//! language types in signatures (it executes prepared plans), GUI concerns,
//! or real-time violations — the callback never allocates, locks, does I/O,
//! logs, or destroys large objects (§13.2).
//!
//! Facade (§15.6): [`AudioEngine::open`], [`AudioEngine::install`],
//! [`AudioEngine::command`]. The rest of the application never sees a CPAL
//! stream, stream config, or sample format.

mod core;
mod engine;
mod error;

pub use crate::core::{PreparedPlaybackPlan, TransportCommand};
pub use crate::engine::{AudioEngine, EngineConfig};
pub use crate::error::EngineError;

#[doc(hidden)]
pub mod testing {
    //! RT-contract test hooks (roadmap §17.5): drive the callback core
    //! against a fake output without an audio device. Not part of the
    //! facade; hidden so no real caller depends on it.
    pub use crate::core::{CallbackCore, Message};
}
