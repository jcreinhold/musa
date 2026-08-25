//! Opaque preparation and one-frame execution for production audio.
//!
//! The public operation consumes exact performed gestures and an authored
//! studio value. It schedules first, prepares the closed native primitive
//! graph as a one-frame machine, and returns the one stateful object shared
//! by offline rendering and the live callback.
#![allow(clippy::arithmetic_side_effects)]

use musa_calculus::CheckedSource;
use musa_events::{Duration, PerformedTime, PhysicalTime, Position, empty};
use musa_score::{Gesture, GesturePlan, PartId, Tuning};
use num_rational::Ratio;

use crate::intent::StudioSpec;
use crate::plan::{PreparedParameterEvent, PreparedParameterId, RenderPlan, prepare_routed_plan};
use crate::primitive::{AudioLimits, resources};
use crate::schedule::{
    AudioFormat, ChannelLayout, Schedule, ScheduleError, SchedulePolicy, ScheduledSource, TimeDecision, TimeMap,
    merge_schedules, schedule,
};
use crate::spec::GraphOptions;
use crate::studio::{lower_studio, lower_studio_for_parts};

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
    // machine calculus beside the source-derived native graph projection.
    _instrument_machine: crate::PreparedMachine,
    // Exact source identities retained beside their compact prepared slots.
    // Equal declarations deliberately still have distinct instance ids.
    _instrument_instances: Vec<InstrumentInstanceBinding>,
    plan: RenderPlan,
    schedule: Schedule<PerformedTime, Gesture>,
    lanes: Vec<PreparedLane>,
    tuning: Tuning,
    sample_rate: u32,
    total_frames: u64,
    position: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PreparedInstrumentId(usize);

struct InstrumentInstanceBinding {
    _part: PartId,
    _declaration: String,
    _instance: PreparedInstrumentId,
}

struct PreparedLane {
    instance: PreparedInstrumentId,
    source: ScheduledSource<Gesture>,
    controls: Vec<PreparedControlBatch>,
    control_cursor: usize,
}

struct PreparedControlBatch {
    frame: u64,
    events: Box<[PreparedParameterEvent]>,
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
        for lane in &mut self.lanes {
            lane.source.seek(self.position);
            lane.control_cursor = lane.controls.partition_point(|batch| batch.frame < self.position);
        }
    }

    /// Execute exactly one reference frame, reading this frame's event batch
    /// before producing its stereo output. No allocation, lock, I/O, or log
    /// occurs on this path.
    pub fn step(&mut self) -> [f32; 2] {
        if self.position >= self.total_frames {
            return [0.0; 2];
        }
        for lane in &mut self.lanes {
            if let Some(batch) = lane.controls.get(lane.control_cursor)
                && batch.frame == self.position
            {
                self.plan.apply_parameter_events(&batch.events);
                lane.control_cursor = lane.control_cursor.saturating_add(1);
            }
            let (_, messages) = lane.source.step();
            self.plan.apply_events(lane.instance.0, messages, self.tuning);
        }
        let frame = self.plan.finish_step();
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
pub(crate) fn prepare_projected_execution(
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

/// Prepare audio from the one checked production studio value.
///
/// # Errors
/// Refuses malformed checked source before any primitive is allocated, then
/// reports the same scheduling, primitive, and extent errors as
/// [`AudioPrepareError`].
pub fn prepare_execution(
    gestures: &GesturePlan,
    instruments: &CheckedSource,
    instrument_machine: &musa_score::MachineSpec,
    studio: &crate::StudioExecution,
    options: AudioOptions,
) -> Result<PreparedAudio, AudioPrepareError> {
    let projection = studio.preparation_projection().ok_or_else(|| {
        AudioPrepareError::StudioValue("checked studio processor has no native preparation witness".to_owned())
    })?;
    prepare_projected_execution(gestures, instruments, instrument_machine, &projection, options)
}

fn prepare_audio(
    gestures: &GesturePlan,
    instrument_contracts: crate::InstrumentContracts,
    instrument_machine: crate::PreparedMachine,
    studio: &StudioSpec,
    options: AudioOptions,
) -> Result<PreparedAudio, AudioPrepareError> {
    let basic = instrument_contracts
        .declaration("std.sound.basic_sine@1")
        .ok_or_else(|| {
            AudioPrepareError::InstrumentContract("the edition-one basic instrument is absent".to_owned())
        })?;
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
    let part_names = gestures
        .lanes()
        .iter()
        .map(musa_score::GestureLane::name)
        .collect::<Vec<_>>();
    let (graph, lowering, part_inputs) = if part_names.is_empty() {
        let (graph, lowering) = lower_studio(studio, &graph_options);
        (graph, lowering, Vec::new())
    } else {
        lower_studio_for_parts(studio, &part_names, &graph_options)
    };
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
    let mut instrument_instances = Vec::with_capacity(gestures.lanes().len());
    let mut event_inputs = Vec::with_capacity(gestures.lanes().len());
    for (instance, lane) in gestures.lanes().iter().enumerate() {
        let input = part_inputs
            .iter()
            .find(|input| input.part == lane.name())
            .ok_or_else(|| {
                AudioPrepareError::StudioValue(format!("part `{}` has no prepared instrument instance", lane.name()))
            })?;
        event_inputs.push((input.node, instance));
        instrument_instances.push(InstrumentInstanceBinding {
            _part: lane.part(),
            _declaration: input.declaration.clone(),
            _instance: PreparedInstrumentId(instance),
        });
    }
    let mut plan = prepare_routed_plan(&graph, &graph_options, &event_inputs)
        .map_err(|error| AudioPrepareError::Primitive(error.to_string()))?;
    let (schedule, lane_schedules) = schedule_gestures(gestures, options)?;
    let mut lane_controls = Vec::with_capacity(lane_schedules.len());
    for (instance, lane_schedule) in lane_schedules.iter().enumerate() {
        let input = part_inputs.get(instance).ok_or_else(|| {
            AudioPrepareError::StudioValue("a scheduled lane has no prepared instrument input".to_owned())
        })?;
        let declaration = instrument_contracts.declaration(&input.declaration).unwrap_or(basic);
        lane_controls.push(prepare_control_batches(
            &mut plan,
            input.node,
            declaration,
            lane_schedule,
        )?);
    }
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
    let lanes = lane_schedules
        .iter()
        .zip(lane_controls)
        .enumerate()
        .map(|(instance, (schedule, controls))| PreparedLane {
            instance: PreparedInstrumentId(instance),
            source: schedule.source(),
            controls,
            control_cursor: 0,
        })
        .collect();
    Ok(PreparedAudio {
        _instrument_contracts: instrument_contracts,
        _instrument_machine: instrument_machine,
        _instrument_instances: instrument_instances,
        plan,
        schedule,
        lanes,
        tuning: options.tuning,
        sample_rate,
        total_frames,
        position: 0,
    })
}

struct ResolvedControlMapping {
    kind: String,
    namespace: String,
    name: String,
    continuous: bool,
    target: PreparedParameterId,
    transfer: Option<(Ratio<i64>, Ratio<i64>, bool)>,
    connection_values: Option<[Ratio<i64>; 3]>,
}

fn prepare_control_batches(
    plan: &mut RenderPlan,
    node: crate::spec::NodeId,
    declaration: &crate::InstrumentContract,
    schedule: &Schedule<PerformedTime, Gesture>,
) -> Result<Vec<PreparedControlBatch>, AudioPrepareError> {
    use std::collections::BTreeMap;

    let mut mappings = Vec::with_capacity(declaration.mappings().len());
    let mut defaults = Vec::new();
    for mapping in declaration.mappings() {
        if mapping.node() != "voice" {
            return Err(AudioPrepareError::InstrumentContract(format!(
                "private target `{}` is not supplied by the native instrument instance",
                mapping.node()
            )));
        }
        let Some(target) = plan
            .resolve_parameter(node, mapping.parameter())
            .map_err(|error| AudioPrepareError::InstrumentContract(error.to_string()))?
        else {
            continue;
        };
        let requirement = declaration
            .controls()
            .iter()
            .find(|control| {
                control.kind() == mapping.kind()
                    && control.namespace() == mapping.namespace()
                    && control.name() == mapping.name()
            })
            .ok_or_else(|| {
                AudioPrepareError::InstrumentContract(format!(
                    "mapping for `{}::{}` has no matching exposed control",
                    mapping.namespace(),
                    mapping.name()
                ))
            })?;
        let default = mapped_control_value(
            requirement.default_ratio(),
            requirement.default_symbol(),
            mapping.transfer(),
            mapping.connection_values(),
        )?;
        if !plan.parameter_is_written(target) {
            defaults.push(PreparedParameterEvent {
                target,
                value: default,
                ramp_frames: 0,
            });
        }
        mappings.push(ResolvedControlMapping {
            kind: mapping.kind().to_owned(),
            namespace: mapping.namespace().to_owned(),
            name: mapping.name().to_owned(),
            continuous: requirement.update_rate() == "Continuous",
            target,
            transfer: mapping.transfer(),
            connection_values: mapping.connection_values(),
        });
    }
    plan.apply_parameter_events(&defaults);

    let mut points: BTreeMap<PreparedParameterId, Vec<(u64, f32, bool)>> = BTreeMap::new();
    for (frame, batch) in schedule.batches() {
        for message in batch.messages() {
            let crate::schedule::EventMessage::Begin(_, gesture) = message else {
                continue;
            };
            for control in gesture.controls() {
                let accepted = declaration.controls().iter().any(|requirement| {
                    requirement.kind() == control.kind()
                        && requirement.namespace() == control.namespace()
                        && requirement.name() == control.name()
                });
                if !accepted {
                    return Err(AudioPrepareError::InstrumentContract(format!(
                        "instrument `{}` does not accept control `{}::{}` at source kind `{}`",
                        declaration.declaration_id(),
                        control.namespace(),
                        control.name(),
                        control.kind()
                    )));
                }
                for mapping in mappings.iter().filter(|mapping| {
                    mapping.kind == control.kind()
                        && mapping.namespace == control.namespace()
                        && mapping.name == control.name()
                }) {
                    points.entry(mapping.target).or_default().push((
                        frame,
                        mapped_control_value(
                            control.ratio(),
                            control.symbol(),
                            mapping.transfer,
                            mapping.connection_values,
                        )?,
                        mapping.continuous,
                    ));
                }
            }
        }
    }

    let mut batches: BTreeMap<u64, Vec<PreparedParameterEvent>> = BTreeMap::new();
    for (target, values) in points {
        for (index, &(frame, value, continuous)) in values.iter().enumerate() {
            batches.entry(frame).or_default().push(PreparedParameterEvent {
                target,
                value,
                ramp_frames: 0,
            });
            if continuous
                && let Some(&(next_frame, next_value, _)) = values.get(index.saturating_add(1))
                && next_frame > frame
            {
                batches.entry(frame).or_default().push(PreparedParameterEvent {
                    target,
                    value: next_value,
                    ramp_frames: next_frame.saturating_sub(frame),
                });
            }
        }
    }
    Ok(batches
        .into_iter()
        .map(|(frame, events)| PreparedControlBatch {
            frame,
            events: events.into_boxed_slice(),
        })
        .collect())
}

fn mapped_value(value: Ratio<i64>, transfer: Option<(Ratio<i64>, Ratio<i64>, bool)>) -> Result<f32, AudioPrepareError> {
    let exact = match transfer {
        Some((minimum, maximum, inverse)) => {
            let along = if inverse { Ratio::ONE - value } else { value };
            minimum + (maximum - minimum) * along
        }
        None => value,
    };
    let value = *exact.numer() as f64 / *exact.denom() as f64;
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return Err(AudioPrepareError::InstrumentContract(format!(
            "mapped exact value {}/{} is not a finite DSP parameter",
            exact.numer(),
            exact.denom()
        )));
    }
    Ok(value as f32)
}

fn mapped_control_value(
    ratio: Option<Ratio<i64>>,
    symbol: Option<&str>,
    transfer: Option<(Ratio<i64>, Ratio<i64>, bool)>,
    connection_values: Option<[Ratio<i64>; 3]>,
) -> Result<f32, AudioPrepareError> {
    if let Some(value) = ratio {
        return mapped_value(value, transfer);
    }
    let values = connection_values.ok_or_else(|| {
        AudioPrepareError::InstrumentContract("a symbolic control has no declared transfer".to_owned())
    })?;
    let exact = match symbol {
        Some("Detached") => values[0],
        Some("Ordinary") => values[1],
        Some("Legato") => values[2],
        _ => {
            return Err(AudioPrepareError::InstrumentContract(
                "a phrase-connection mapping received an unsupported source constructor".to_owned(),
            ));
        }
    };
    mapped_value(exact, None)
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
) -> Result<(Schedule<PerformedTime, Gesture>, Vec<Schedule<PerformedTime, Gesture>>), ScheduleError> {
    let lane_schedules = gestures
        .lanes()
        .iter()
        .map(|lane| schedule_lane(lane, options))
        .collect::<Result<Vec<_>, _>>()?;
    let mut schedules = lane_schedules.iter();
    let mut combined = match schedules.next() {
        Some(schedule) => schedule.clone(),
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
    for next in schedules {
        combined = merge_schedules(options.schedule, &combined, next)?;
    }
    Ok((combined, lane_schedules))
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
