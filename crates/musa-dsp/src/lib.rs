//! The declarative studio graph: `StudioGraphSpec` (editable, serializable
//! intent) → compiled `RenderPlan` (preallocated execution) (roadmap §13.3,
//! §13.8).
//!
//! Owns: graph validation (port-kind/channel compatibility, cycle rejection),
//! topological scheduling, buffer allocation, the processor set, and
//! deterministic offline rendering. Must never contain: musical semantics
//! (it consumes scheduled `PerformanceEvent`s, never scores), CPAL or
//! threads (those belong to `musa-playback`), or real-time violations — `RenderPlan::render`
//! allocates nothing and takes no locks (§13.2).
//!
//! Facade (roadmap §15.5): [`compile_graph`], [`RenderPlan::render`]. The
//! graph compiler hides validation, topological sort, and buffer allocation;
//! callers see `build` and `render` (§3).

mod effects;
mod envelope;
mod error;
mod filter;
mod instrument;
mod machine;
mod offline;
mod plan;
mod schedule;
mod spec;
mod studio;
mod voice;

pub use crate::error::GraphError;
pub use crate::instrument::poly_sine_spec;
pub use crate::machine::{MachineValue, PrepareError, PreparedMachine, StartedMachine, StepError, prepare_machine};
pub use crate::offline::{RenderedAudio, render_offline};
pub use crate::plan::{EventSlice, RenderPlan, compile_graph};
pub use crate::schedule::{
    AudioFormat, BoundaryCollision, BoundaryKind, CollapsePolicy, EventBatch, EventHandle, EventMessage, FrameRounding,
    MessageKind, RoundingChoice, Schedule, ScheduleError, ScheduleLimits, SchedulePolicy, ScheduledSource, SourceState,
    TimeDecision, TimeMap, merge_schedules, schedule,
};
pub use crate::spec::{
    Combination, FilterKind, GraphOptions, MAX_DELAY, NodeId, ParameterDescriptor, PortKind, ProcessorSpec, Smoothing,
    StudioGraphSpec, Unit, Waveform,
};
pub use crate::studio::{StudioLowering, lower_studio};
pub use crate::voice::VoiceAllocator;

#[doc(hidden)]
pub mod testing {
    //! Cross-crate semantic harnesses. Production callers prepare and start a
    //! machine directly; tests use this to prove offline iteration calls that
    //! same one-step operation rather than a block-specific interpreter.

    use crate::{MachineValue, PreparedMachine, StepError};

    /// Run a finite input history from the exact start state.
    pub fn run_machine_offline(
        machine: &PreparedMachine,
        inputs: impl IntoIterator<Item = MachineValue>,
    ) -> Result<Vec<MachineValue>, StepError> {
        let mut running = machine.start();
        inputs.into_iter().map(|input| running.step(input)).collect()
    }
}
