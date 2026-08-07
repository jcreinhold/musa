//! The declarative studio graph: `StudioGraphSpec` (editable, serializable
//! intent) → compiled `RenderPlan` (preallocated execution) (roadmap §13.3,
//! §13.8).
//!
//! Owns: graph validation (port-kind/channel compatibility, cycle rejection),
//! topological scheduling, buffer allocation, the processor set, and
//! deterministic offline rendering. Must never contain: musical semantics
//! (it consumes scheduled `PerformanceEvent`s, never scores), CPAL or
//! threads (prompt 18), or real-time violations — `RenderPlan::render`
//! allocates nothing and takes no locks (§13.2).
//!
//! Facade (roadmap §15.5): [`compile_graph`], [`RenderPlan::render`]. The
//! graph compiler hides validation, topological sort, and buffer allocation;
//! callers see `build` and `render` (§3).

mod envelope;
mod error;
mod filter;
mod instrument;
mod offline;
mod plan;
mod spec;
mod studio;
mod voice;

pub use crate::error::GraphError;
pub use crate::instrument::poly_sine_spec;
pub use crate::offline::{RenderedAudio, render_offline};
pub use crate::plan::{EventSlice, RenderPlan, compile_graph};
pub use crate::spec::{
    Combination, FilterKind, GraphOptions, NodeId, ParameterDescriptor, PortKind, ProcessorSpec, Smoothing,
    StudioGraphSpec, Unit, Waveform,
};
pub use crate::studio::{StudioLowering, lower_studio};
pub use crate::voice::VoiceAllocator;
