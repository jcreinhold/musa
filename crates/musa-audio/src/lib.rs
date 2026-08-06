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

mod error;
mod plan;
mod spec;

pub use crate::error::GraphError;
pub use crate::plan::{EventSlice, RenderPlan, compile_graph};
pub use crate::spec::{
    Combination, GraphOptions, NodeId, ParameterDescriptor, PortKind, ProcessorSpec, Smoothing, StudioGraphSpec, Unit,
};
