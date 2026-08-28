//! Checked scheduling and one-frame audio execution.
//!
//! [`prepare_execution`] consumes exact performed gestures, checked source
//! instruments, their machine projection, and the exact read-only projection
//! of a checked source studio, then returns opaque [`PreparedAudio`]. Preparation validates and
//! allocates the closed native primitive graph; both offline rendering and the
//! live callback then repeat the same event-before-output frame step.
//! CPAL and threads remain in `musa-playback`; the step path never allocates,
//! locks, logs, or performs I/O.

mod audio;
mod effects;
mod envelope;
mod error;
mod filter;
mod instrument;
mod instrument_source;
mod intent;
mod machine;
mod media;
mod offline;
mod plan;
mod primitive;
mod quantity;
mod sample_source;
mod sampler;
mod schedule;
mod source;
mod spec;
mod studio;
mod studio_source;
mod voice;

pub use crate::audio::{
    AudioOptions, AudioPrepareError, AudioRoute, AudioRouteKind, AudioTap, AudioTapRole, AuditionEvent,
    AuditionOutcome, PreparedAudio, PreparedAuditionTarget, prepare_execution, prepare_execution_with_media,
};
pub use crate::instrument_source::{
    InstrumentAuditionBinding, InstrumentContract, InstrumentContracts, InstrumentContractsError,
    InstrumentControlContract, InstrumentTechniqueContract, MidiAuditionInputKind, MidiAuditionScope,
    decode_instrument_contracts, instrument_contracts_schema,
};
pub use crate::machine::{MachineValue, PrepareError, PreparedMachine, StartedMachine, StepError, prepare_machine};
pub use crate::media::{MediaLimits, MediaPrepareError, PreparedMedia, prepare_media};
pub use crate::offline::{RenderedAudio, RenderedMultitrack, RenderedStem, render_offline, render_offline_multitrack};
pub use crate::primitive::{AudioLimits, VocabularyAgreementError, check_studio_vocabulary};
pub use crate::sample_source::{SampleMap, SampleMapError, decode_sample_map, sample_map_schema};
pub use crate::sampler::{
    PreparedSampleMap, SampleRuntime, SampleSelectionToken, SampleSelector, SamplerLimits, SamplerPrepareError,
    SamplerResources, prepare_sample_map,
};
pub use crate::schedule::{
    AudioFormat, BoundaryCollision, BoundaryKind, ChannelLayout, CollapsePolicy, EventBatch, EventHandle, EventMessage,
    FrameRounding, MessageKind, RoundingChoice, Schedule, ScheduleError, ScheduleLimits, SchedulePolicy,
    ScheduledSource, SourceState, TimeDecision, TimeMap, merge_schedules, schedule,
};
pub use crate::source::{
    CheckedExactQuantity, ExactQuantityError, ExactQuantityProjection, ParameterProjection, ParameterValueKind,
    PortContract, PortKindProjection, PortKindTag, PortPathProjection, PrimitiveRequirement, ProcessorContract,
    SignalRole, SoundDimension, SoundUnit, StudioDeclaration, StudioDeclarationKind, StudioDescription,
    StudioDescriptionError, StudioParameterContract, StudioTermContract, StudioVocabulary, StudioVocabularyError,
    SurfacePort, decode_exact_quantity, decode_studio_description, decode_studio_vocabulary, exact_quantity_schema,
    studio_description_schema, studio_vocabulary_schema,
};
pub use crate::studio_source::{
    StudioAssignmentProjection, StudioExecution, StudioExecutionError, StudioGraphProjection,
    StudioModulationProjection, StudioNodeProjection, StudioRouteProjection, StudioSendProjection,
    decode_studio_execution, studio_execution_schema,
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

    pub(crate) use crate::error::GraphError;
    pub(crate) use crate::instrument::poly_sine_spec;
    pub(crate) use crate::intent::Unit;
    pub(crate) use crate::spec::{
        Combination, FilterKind, GraphOptions, NodeId, ProcessorSpec, StudioGraphSpec, Waveform,
    };
    pub(crate) use crate::studio::lower_studio;

    /// One-frame harness for primitive and private-flattening laws.
    pub(crate) struct PreparedGraph(crate::plan::RenderPlan);

    /// Validate and allocate a private graph with one-frame execution.
    pub(crate) fn prepare_graph(spec: &StudioGraphSpec, sample_rate: u32) -> Result<PreparedGraph, GraphError> {
        prepare_graph_seeded(spec, sample_rate, 0)
    }

    /// Validate and allocate the test graph with an explicit stochastic seed.
    pub(crate) fn prepare_graph_seeded(
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
        pub(crate) fn render(&mut self, first: &[EventMessage<Gesture>], output: &mut [f32]) {
            for (index, frame) in output.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                let messages = if index == 0 { first } else { &[] };
                let [left, right] = self.0.step(messages, Tuning::default());
                *frame = [left, right];
            }
        }
    }

    /// Run a finite input history from the exact start state.
    pub(crate) fn run_machine_offline(
        machine: &PreparedMachine,
        inputs: impl IntoIterator<Item = MachineValue>,
    ) -> Result<Vec<MachineValue>, StepError> {
        let mut running = machine.start();
        inputs.into_iter().map(|input| running.step(input)).collect()
    }
}
