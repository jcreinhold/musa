//! Platform and real-time integration.
//!
//! Owns: CPAL device negotiation and output stream lifecycle, live
//! scheduling, MIDI input (`midir`), transport state, the real-time command
//! boundary (`rtrb` queues in both directions, including retired-plan
//! return), render-plan installation, and clock synchronization — design
//! roadmap §15.6.
//!
//! Must never expose: a CPAL stream, stream config, or sample format to the
//! rest of the application; must never contain: composition, notation, or
//! project semantics.
//!
//! Intended facade (roadmap §15.6), to be implemented by prompts 13 and 23:
//!
//! ```text
//! pub struct AudioEngine { /* hidden */ }
//! impl AudioEngine {
//!     pub fn open(config: EngineConfig) -> Result<Self, EngineError>;
//!     pub fn install(&self, plan: PreparedPlaybackPlan) -> Result<(), EngineError>;
//!     pub fn command(&self, command: TransportCommand) -> Result<(), EngineError>;
//! }
//! ```
//!
//! Invariants (roadmap §13.2): the audio callback never allocates, acquires
//! locks, performs I/O, parses, compiles, logs, or destroys large objects;
//! plans are prepared and preallocated on the control side and cross the
//! boundary on `rtrb` queues; underruns produce silence, never panics.
