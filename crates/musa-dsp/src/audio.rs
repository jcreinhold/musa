//! Opaque preparation and one-frame execution for production audio.
//!
//! The public operation consumes exact performed gestures and an authored
//! studio value. It schedules first, prepares the closed native primitive
//! graph as a one-frame machine, and returns the one stateful object shared
//! by offline rendering and the live callback.
#![allow(clippy::arithmetic_side_effects)]

use musa_calculus::CheckedSource;
use musa_events::{Duration, PerformedTime, PhysicalTime, Position, empty};
use musa_score::{Gesture, GesturePlan, Tuning};

use crate::StudioSpec;
use crate::plan::{RenderPlan, prepare_plan};
use crate::primitive::{AudioLimits, resources};
use crate::schedule::{
    AudioFormat, ChannelLayout, Schedule, ScheduleError, SchedulePolicy, ScheduledSource, TimeDecision, TimeMap,
    merge_schedules, schedule,
};
use crate::spec::GraphOptions;
use crate::studio::lower_studio;

/// Every product-level choice that can change audio preparation or its finite
/// playback extent. There is deliberately no default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioOptions {
    /// Physical frame lattice.
    pub format: AudioFormat,
    /// Exact-to-frame and finite-resource policy.
    pub schedule: SchedulePolicy,
    /// Explicit pitch-to-frequency interpretation.
    pub tuning: Tuning,
    /// Explicit seed for every stochastic primitive in this render.
    pub render_seed: u64,
    /// Native primitive memory and per-frame work bounds.
    pub limits: AudioLimits,
    /// Frames retained after the last scheduled boundary for release tails.
    pub tail_frames: u64,
    /// Greatest accepted schedule plus tail.
    pub max_total_frames: u64,
}

/// A stable failure before any machine reaches a real-time thread.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum AudioPrepareError {
    /// Checked frame scheduling failed.
    #[error("gesture scheduling failed: {0}")]
    Schedule(#[from] ScheduleError),
    /// Native primitive graph validation or preparation failed.
    #[error("audio primitive preparation failed: {0}")]
    Primitive(String),
    /// Exact studio intent failed conversion or a private DSP range check.
    #[error("studio value preparation failed: {0}")]
    StudioValue(String),
    /// Checked source instrument declarations failed schema or contract checks.
    #[error("instrument contract preparation failed: {0}")]
    InstrumentContract(String),
    /// The requested tail makes the finite playback extent unacceptable.
    #[error("audio extent {actual} frames exceeds explicit limit {limit}")]
    FrameLimit { actual: u64, limit: u64 },
    /// The production machine has one stereo-frame reference meaning.
    #[error("unsupported audio layout {0:?}; production audio requires stereo")]
    UnsupportedLayout(ChannelLayout),
    /// Pitch interpretation must produce positive finite frequencies.
    #[error("concert A must be a positive finite frequency, got {0}")]
    InvalidTuning(f64),
    /// A checked native resource exceeds the product's explicit bound.
    #[error("audio {resource} requirement {actual} exceeds explicit limit {limit}")]
    ResourceLimit {
        /// Resource class with stable spelling.
        resource: &'static str,
        /// Conservatively priced requirement.
        actual: u64,
        /// Product-supplied bound.
        limit: u64,
    },
}

/// A fully scheduled, allocated audio machine. Its graph, state, buffers,
/// event cursor, and handle tables are intentionally inaccessible.
pub struct PreparedAudio {
    // Retain the complete exact checked argument. A digest or the queried
    // fields below cannot stand in for preparation identity (R1).
    _instrument_contracts: crate::InstrumentContracts,
    // The source instrument component is prepared through the governing
    // machine calculus even while the migration oracle supplies production
    // oscillator execution. Prompt 180a removes that oracle.
    _instrument_machine: crate::PreparedMachine,
    plan: RenderPlan,
    schedule: Schedule<PerformedTime, Gesture>,
    source: ScheduledSource<Gesture>,
    tuning: Tuning,
    sample_rate: u32,
    total_frames: u64,
    position: u64,
}

impl PreparedAudio {
    /// Complete auditable time decisions for all gesture lanes.
    pub fn decisions(&self) -> &[TimeDecision<PerformedTime>] {
        self.schedule.decisions()
    }

    /// Finite playback extent, including the explicit release tail.
    pub const fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Prepared physical sample rate.
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Frame that will be produced by the next call to [`Self::step`].
    pub const fn position(&self) -> u64 {
        self.position
    }

    /// Move the event source to an absolute frame. DSP state is retained,
    /// matching transport seek rather than reconstructing the instrument.
    pub fn seek(&mut self, frame: u64) {
        self.position = frame.min(self.total_frames);
        self.source.seek(self.position);
    }

    /// Execute exactly one reference frame, reading this frame's event batch
    /// before producing its stereo output. No allocation, lock, I/O, or log
    /// occurs on this path.
    pub fn step(&mut self) -> [f32; 2] {
        if self.position >= self.total_frames {
            return [0.0; 2];
        }
        let (_, messages) = self.source.step();
        let frame = self.plan.step(messages, self.tuning);
        self.position = self.position.saturating_add(1);
        frame
    }

    /// Fill an interleaved stereo host block by repeated reference steps.
    /// Host partitioning therefore cannot alter state or sound.
    pub fn render(&mut self, output: &mut [f32]) {
        let (frames, remainder) = output.as_chunks_mut::<2>();
        for frame in frames {
            let [left, right] = self.step();
            *frame = [left, right];
        }
        remainder.fill(0.0);
    }
}

/// Prepare exact gestures, checked scheduling, instruments, routing, effects,
/// and finite playback extent as one audio machine.
///
/// # Errors
/// [`AudioPrepareError`] names the scheduling, primitive, or extent check that
/// refused preparation.
pub fn prepare_execution(
    gestures: &GesturePlan,
    instruments: &CheckedSource,
    instrument_machine: &musa_score::MachineSpec,
    studio: &StudioSpec,
    options: AudioOptions,
) -> Result<PreparedAudio, AudioPrepareError> {
    let contracts = crate::decode_instrument_contracts(instruments)
        .map_err(|error| AudioPrepareError::InstrumentContract(error.to_string()))?;
    let basic = contracts.declaration("std.sound.basic_sine@1").ok_or_else(|| {
        AudioPrepareError::InstrumentContract("the edition-one basic instrument is absent".to_owned())
    })?;
    if basic.channels() != 2 {
        return Err(AudioPrepareError::InstrumentContract(
            "the edition-one basic instrument must promise stereo output".to_owned(),
        ));
    }
    if !basic
        .controls()
        .iter()
        .any(|control| control.namespace() == "std.performance" && control.name() == "expression")
    {
        return Err(AudioPrepareError::InstrumentContract(
            "the edition-one basic instrument does not accept standard expression".to_owned(),
        ));
    }
    let instrument_machine = crate::prepare_machine(instrument_machine)
        .map_err(|error| AudioPrepareError::InstrumentContract(error.to_string()))?;
    prepare_audio(gestures, contracts, instrument_machine, studio, options)
}

fn prepare_audio(
    gestures: &GesturePlan,
    instrument_contracts: crate::InstrumentContracts,
    instrument_machine: crate::PreparedMachine,
    studio: &StudioSpec,
    options: AudioOptions,
) -> Result<PreparedAudio, AudioPrepareError> {
    if options.format.layout() != ChannelLayout::Stereo {
        return Err(AudioPrepareError::UnsupportedLayout(options.format.layout()));
    }
    if !options.tuning.concert_a.is_finite() || options.tuning.concert_a <= 0.0 {
        return Err(AudioPrepareError::InvalidTuning(options.tuning.concert_a));
    }
    let sample_rate = options.format.sample_rate().get();
    let graph_options = GraphOptions {
        sample_rate,
        render_seed: options.render_seed,
    };
    let (graph, lowering) = lower_studio(studio, &graph_options);
    if !lowering.errors.is_empty() {
        return Err(AudioPrepareError::StudioValue(lowering.errors.join("; ")));
    }
    if graph.output_kind() != Some(crate::spec::PortKind::Audio { channels: 2 }) {
        return Err(AudioPrepareError::Primitive(
            "native graph output must be one stereo frame".to_owned(),
        ));
    }
    let required = resources(&graph, sample_rate, options.schedule.limits().max_messages).ok_or_else(|| {
        AudioPrepareError::Primitive("native primitive is absent from the closed registry".to_owned())
    })?;
    check_resources(required, options.limits)?;
    let plan = prepare_plan(&graph, &graph_options).map_err(|error| AudioPrepareError::Primitive(error.to_string()))?;
    let schedule = schedule_gestures(gestures, options)?;
    let studio_tail = (f64::from(lowering.release_tail) * f64::from(sample_rate)) as u64;
    let tail_frames = options.tail_frames.saturating_add(studio_tail);
    let total_frames = schedule
        .finish_frame()
        .checked_add(tail_frames)
        .filter(|frames| *frames <= options.max_total_frames)
        .ok_or_else(|| AudioPrepareError::FrameLimit {
            actual: schedule.finish_frame().saturating_add(tail_frames),
            limit: options.max_total_frames,
        })?;
    let source = schedule.source();
    Ok(PreparedAudio {
        _instrument_contracts: instrument_contracts,
        _instrument_machine: instrument_machine,
        plan,
        schedule,
        source,
        tuning: options.tuning,
        sample_rate,
        total_frames,
        position: 0,
    })
}

fn check_resources(actual: crate::primitive::AudioResources, limits: AudioLimits) -> Result<(), AudioPrepareError> {
    let checks = [
        (
            "primitive count",
            actual.primitives as u64,
            limits.max_primitives as u64,
        ),
        (
            "retained state bytes",
            actual.state_bytes as u64,
            limits.max_state_bytes as u64,
        ),
        ("one-frame work", actual.step_work, limits.max_step_work),
    ];
    for (resource, actual, limit) in checks {
        if actual > limit {
            return Err(AudioPrepareError::ResourceLimit {
                resource,
                actual,
                limit,
            });
        }
    }
    Ok(())
}

fn schedule_gestures(
    gestures: &GesturePlan,
    options: AudioOptions,
) -> Result<Schedule<PerformedTime, Gesture>, ScheduleError> {
    let mut lanes = gestures.lanes().iter();
    let mut combined = match lanes.next() {
        Some(lane) => schedule_lane(lane, options)?,
        None => {
            let track = empty(Duration::ZERO);
            schedule(
                options.format,
                options.schedule,
                &TimeMap::new(1, [(Position::ZERO, Position::<PhysicalTime>::ZERO)]),
                &track,
            )?
        }
    };
    for lane in lanes {
        let next = schedule_lane(lane, options)?;
        combined = merge_schedules(options.schedule, &combined, &next)?;
    }
    Ok(combined)
}

fn schedule_lane(
    lane: &musa_score::GestureLane,
    options: AudioOptions,
) -> Result<Schedule<PerformedTime, Gesture>, ScheduleError> {
    let track = lane.track();
    let assignments = track
        .occurrences()
        .iter()
        .flat_map(|occurrence| [occurrence.span().start(), occurrence.span().end()])
        .chain([track.duration().reach()])
        .map(|position| (position, lane.physical(position)));
    schedule(options.format, options.schedule, &TimeMap::new(1, assignments), track)
}
