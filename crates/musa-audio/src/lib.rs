//! Studio graph compilation and pure DSP.
//!
//! Owns: declarative studio graph types (`StudioGraphSpec`), graph validation
//! (port compatibility, channel counts, cycles legal only through explicit
//! delay), graph compilation into preallocated `RenderPlan`s, the processor
//! interface, oscillators, envelopes, filters, effects, voice allocation,
//! block processing, deterministic offline rendering, and the built-in patch
//! library — design roadmap §15.5.
//!
//! Must never expose: `FunDSP` types (if adopted as an implementation
//! backend, §13.6) or DSP internals across the crate boundary; must never contain:
//! CPAL or any platform audio code (that is `musa-engine`), GUI concepts, or
//! score semantics (the studio receives performance events, never notes —
//! §6.5).
//!
//! Intended facade (roadmap §13.3/§13.8), to be implemented by prompts
//! 11–12 and 19–21:
//!
//! ```text
//! pub fn compile_graph(spec: &StudioGraphSpec, options: &GraphOptions)
//!     -> Result<RenderPlan, GraphError>;
//! impl RenderPlan {
//!     pub fn render(&mut self, events: &EventSlice, output: &mut [f32], frames: usize);
//! }
//! pub fn render_offline(plan: &mut RenderPlan, events: &PerformanceEvents,
//!     frames: u64) -> RenderedAudio;
//! ```
//!
//! Invariants: `RenderPlan::render` never allocates, locks, or destroys large
//! objects; offline and live rendering execute the same processors on the
//! same scheduled events; rendering is deterministic and NaN/infinity-free.
