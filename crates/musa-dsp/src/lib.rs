//! Checked scheduling and one-frame audio execution.
//!
//! [`prepare_audio`] consumes exact performed gestures plus authored studio
//! intent and returns opaque [`PreparedAudio`]. Preparation validates and
//! allocates the closed native primitive graph; both offline rendering and
//! the live callback then repeat the same event-before-output frame step.
//! CPAL and threads remain in `musa-playback`; the step path never allocates,
//! locks, logs, or performs I/O.

mod audio;
mod catalogue;
mod effects;
mod envelope;
mod error;
mod filter;
mod instrument;
mod intent;
mod machine;
mod offline;
mod plan;
mod primitive;
mod quantity;
mod schedule;
mod source;
mod spec;
mod studio;
mod voice;

pub use crate::audio::{AudioOptions, AudioPrepareError, PreparedAudio, prepare_audio};
pub use crate::catalogue::{
    BuiltinKey, PROCESSORS, PortSchema, ProcessorDoc, SignalRole, StudioTermDoc, SurfacePort, SurfaceSchema, TERMS,
    processor as processor_doc, reference_markdown as studio_reference, term as studio_term,
};
pub use crate::intent::{
    Assignment, Modulation, NodeIndex, ParamSpec, Patch, Processor, Route, Send, StudioNode, StudioSpec, Unit,
    WrittenQuantity, written_ratio,
};
pub use crate::machine::{MachineValue, PrepareError, PreparedMachine, StartedMachine, StepError, prepare_machine};
pub use crate::offline::{RenderedAudio, render_offline};
pub use crate::primitive::AudioLimits;
pub use crate::schedule::{
    AudioFormat, BoundaryCollision, BoundaryKind, ChannelLayout, CollapsePolicy, EventBatch, EventHandle, EventMessage,
    FrameRounding, MessageKind, RoundingChoice, Schedule, ScheduleError, ScheduleLimits, SchedulePolicy,
    ScheduledSource, SourceState, TimeDecision, TimeMap, merge_schedules, schedule,
};
pub use crate::source::{
    ParameterProjection, ParameterValueKind, PortKindProjection, PortKindTag, PortPathProjection, StudioDeclaration,
    StudioDeclarationKind, StudioDescription, StudioDescriptionError, decode_studio_description,
    studio_description_schema,
};

#[cfg(test)]
extern crate self as musa_dsp;

#[cfg(test)]
#[path = "../tests/suite/main.rs"]
mod suite;

#[cfg(any(test, feature = "testing"))]
#[doc(hidden)]
pub mod testing {
    //! Cross-crate semantic harnesses. Production callers prepare and start a
    //! machine directly; tests use this to prove offline iteration calls that
    //! same one-step operation rather than a block-specific interpreter.

    use musa_score::{Gesture, Tuning};

    use crate::{EventMessage, MachineValue, PreparedMachine, StepError};

    pub use crate::Unit;
    pub use crate::error::GraphError;
    pub use crate::instrument::poly_sine_spec;
    pub use crate::spec::{Combination, FilterKind, GraphOptions, NodeId, ProcessorSpec, StudioGraphSpec, Waveform};
    pub use crate::studio::lower_studio;

    /// One-frame harness for primitive and private-flattening laws.
    pub struct PreparedGraph(crate::plan::RenderPlan);

    /// Validate and allocate a private graph with one-frame execution.
    pub fn prepare_graph(spec: &StudioGraphSpec, sample_rate: u32) -> Result<PreparedGraph, GraphError> {
        prepare_graph_seeded(spec, sample_rate, 0)
    }

    /// Validate and allocate the test graph with an explicit stochastic seed.
    pub fn prepare_graph_seeded(
        spec: &StudioGraphSpec,
        sample_rate: u32,
        render_seed: u64,
    ) -> Result<PreparedGraph, GraphError> {
        crate::plan::prepare_plan(
            spec,
            &GraphOptions {
                sample_rate,
                render_seed,
            },
        )
        .map(PreparedGraph)
    }

    impl PreparedGraph {
        /// Run repeated reference steps, delivering `first` before frame zero.
        pub fn render(&mut self, first: &[EventMessage<Gesture>], output: &mut [f32]) {
            for (index, frame) in output.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                let messages = if index == 0 { first } else { &[] };
                let [left, right] = self.0.step(messages, Tuning::default());
                *frame = [left, right];
            }
        }
    }

    /// Run a finite input history from the exact start state.
    pub fn run_machine_offline(
        machine: &PreparedMachine,
        inputs: impl IntoIterator<Item = MachineValue>,
    ) -> Result<Vec<MachineValue>, StepError> {
        let mut running = machine.start();
        inputs.into_iter().map(|input| running.step(input)).collect()
    }
}
