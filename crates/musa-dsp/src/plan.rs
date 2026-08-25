//! The compiled render plan: validation, topological scheduling, buffer
//! arena, and the processor set (roadmap §13.3–§13.5, §13.8).
//!
//! Real-time contract (§13.2): everything is allocated in `prepare_plan`;
//! one reference step never allocates, locks, or does I/O. Output buffers
//! are moved out of the arena, written, and moved back (`Box<[f32]>` moves
//! without touching the heap).
//!
//! FP arithmetic is inherent to DSP (§13); the workspace arithmetic lint is
//! allowed at module scope. Sample loops index by bound-checked cursor —
//! bounds are a compile-time invariant (port lists are fixed at compile).
#![allow(clippy::arithmetic_side_effects)]

use musa_score::{Gesture, Tuning};

use crate::effects::{Chorus, Delay, Limiter, Reverb};
use crate::error::GraphError;
use crate::filter::{Biquad, Coefficients, OnePole};
use crate::schedule::EventMessage;
use crate::spec::{
    Combination, Connection, FilterKind, GraphOptions, MAX_DELAY, NodeId, ParameterDescriptor, PortKind, ProcessorSpec,
    Smoothing, StudioGraphSpec, Waveform,
};
use crate::voice::VoiceAllocator;

/// A compiled, preallocated graph execution plan.
pub(crate) struct RenderPlan {
    sample_rate: u32,
    schedule: Vec<Step>,
    buffers: Vec<Box<[f32]>>,
    /// Buffer index of the master output (stereo planar or mono).
    master: usize,
    master_channels: usize,
}

struct Step {
    instance: ProcessorInstance,
    /// Buffer index per input port (silence/zero buffers when unconnected).
    inputs: Vec<usize>,
    /// Buffer index per output port.
    outputs: Vec<usize>,
    /// Preallocated scratch for taken output buffers (capacity = outputs).
    taken: Vec<Box<[f32]>>,
    /// Whether this processor consumes scheduled note events (§13.5).
    takes_events: bool,
    /// Control connections into this node's parameters (§13.7).
    modulations: Vec<ParamLink>,
}

/// One resolved modulation: where the control value comes from, what the
/// parameter it lands on says about it, and the state that says applies it
/// smoothly.
struct ParamLink {
    /// Buffer index of the modulating control signal.
    buffer: usize,
    descriptor: ParameterDescriptor,
    /// The value the spec wrote, which the modulation combines with.
    base: f32,
    /// The value last handed to the processor — the smoother's state.
    current: f32,
    /// One-pole coefficient applied per reference frame.
    coefficient: f32,
}

enum ProcessorInstance {
    Sine {
        phase: f64,
        frequency: f32,
    },
    Noise {
        state: u32,
    },
    Constant {
        value: f32,
    },
    Gain {
        gain: f32,
    },
    StereoGain {
        gain: f32,
    },
    Passthrough,
    Pan {
        pan: f32,
    },
    Mixer,
    Splitter,
    MonoToStereo,
    StereoToMono,
    PolySine {
        allocator: VoiceAllocator,
    },
    Lfo {
        waveform: Waveform,
        phase: f64,
        frequency: f32,
    },
    Scale {
        factor: f32,
    },
    Bias {
        offset: f32,
    },
    Clamp {
        min: f32,
        max: f32,
    },
    Smooth {
        time: f32,
        state: OnePole,
    },
    Filter {
        kind: FilterKind,
        cutoff: f32,
        q: f32,
        /// The coefficients in force, walked toward `target` each frame.
        coefficients: Coefficients,
        /// What the current parameters ask for.
        target: Coefficients,
        channels: [Biquad; 2],
    },
    OnePoleFilter {
        cutoff: f32,
        coefficient: f32,
        channels: [OnePole; 2],
    },
    Echo {
        time: f32,
        feedback: f32,
        mix: f32,
        channels: [Delay; 2],
    },
    Chorusing {
        rate: f32,
        depth: f32,
        mix: f32,
        channels: [Chorus; 2],
    },
    Room {
        room: f32,
        damping: f32,
        mix: f32,
        channels: [Reverb; 2],
    },
    Ceiling {
        ceiling: f32,
        limiter: Limiter,
    },
}

impl ProcessorInstance {
    /// Deliver a scheduled event. Only event-consuming processors act.
    fn apply_event(&mut self, event: &EventMessage<Gesture>, tuning: Tuning) {
        let Self::PolySine { allocator } = self else {
            return;
        };
        match event {
            EventMessage::Begin(handle, gesture) => {
                allocator.note_on(
                    handle,
                    tuning.frequency(&gesture.pitch) as f32,
                    ratio_to_f32(gesture.amplitude),
                    ratio_to_f32(gesture.attack_seconds),
                );
            }
            EventMessage::End(handle) => allocator.note_off(handle),
            EventMessage::Point(_, _) => {}
        }
    }

    fn instantiate(
        spec: &StudioGraphSpec,
        node: NodeId,
        processor: ProcessorSpec,
        sample_rate: u32,
        render_seed: u64,
    ) -> Self {
        let param = |name: &str| {
            processor
                .parameters()
                .iter()
                .find(|d| d.name == name)
                .map_or(0.0, |d| spec.param_value(node, d))
        };
        match processor {
            ProcessorSpec::Sine => Self::Sine {
                phase: 0.0,
                frequency: param("frequency"),
            },
            ProcessorSpec::Noise => {
                let mixed = render_seed ^ u64::from(node.0).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                let folded = (mixed ^ (mixed >> 32)) as u32;
                Self::Noise { state: folded.max(1) }
            }
            ProcessorSpec::Constant => Self::Constant { value: param("value") },
            ProcessorSpec::Gain => Self::Gain { gain: param("gain") },
            ProcessorSpec::StereoGain => Self::StereoGain { gain: param("gain") },
            ProcessorSpec::Passthrough { .. } => Self::Passthrough,
            ProcessorSpec::Pan => Self::Pan { pan: param("pan") },
            ProcessorSpec::Mixer { .. } => Self::Mixer,
            ProcessorSpec::PolySine { voices } => {
                // The synth's parameters are its envelope and its oscillator
                // bank, and unlike every other processor they live inside
                // the allocator rather than in the instance's own fields.
                let mut allocator = VoiceAllocator::new(voices, sample_rate);
                for descriptor in processor.parameters() {
                    allocator.set_voice(descriptor.name, spec.param_value(node, descriptor));
                }
                Self::PolySine { allocator }
            }
            ProcessorSpec::Splitter => Self::Splitter,
            ProcessorSpec::MonoToStereo => Self::MonoToStereo,
            ProcessorSpec::StereoToMono => Self::StereoToMono,
            ProcessorSpec::Lfo { waveform } => Self::Lfo {
                waveform,
                phase: 0.0,
                frequency: param("frequency"),
            },
            ProcessorSpec::Scale => Self::Scale {
                factor: param("factor"),
            },
            ProcessorSpec::Bias => Self::Bias {
                offset: param("offset"),
            },
            ProcessorSpec::Clamp => Self::Clamp {
                min: param("min"),
                max: param("max"),
            },
            ProcessorSpec::Smooth => Self::Smooth {
                time: param("time"),
                state: OnePole::default(),
            },
            ProcessorSpec::Biquad { kind } => {
                let (cutoff, q) = (param("cutoff"), param("q"));
                let coefficients = Coefficients::new(kind, cutoff, q, sample_rate as f32);
                Self::Filter {
                    kind,
                    cutoff,
                    q,
                    coefficients,
                    target: coefficients,
                    channels: [Biquad::default(); 2],
                }
            }
            ProcessorSpec::Delay => Self::Echo {
                time: param("time"),
                feedback: param("feedback"),
                mix: param("mix"),
                channels: [
                    Delay::new(MAX_DELAY, sample_rate as f32),
                    Delay::new(MAX_DELAY, sample_rate as f32),
                ],
            },
            ProcessorSpec::Chorus => Self::Chorusing {
                rate: param("rate"),
                depth: param("depth"),
                mix: param("mix"),
                // Half a cycle apart, so the two channels wobble against each
                // other rather than together: that difference is the width.
                channels: [
                    Chorus::new(0.0, sample_rate as f32),
                    Chorus::new(0.5, sample_rate as f32),
                ],
            },
            ProcessorSpec::Reverb => Self::Room {
                room: param("room"),
                damping: param("damping"),
                mix: param("mix"),
                channels: [
                    Reverb::new(0, sample_rate as f32),
                    Reverb::new(Reverb::spread(), sample_rate as f32),
                ],
            },
            ProcessorSpec::Limiter => Self::Ceiling {
                ceiling: param("ceiling"),
                limiter: Limiter::new(sample_rate as f32),
            },
            ProcessorSpec::OnePole => {
                let cutoff = param("cutoff");
                Self::OnePoleFilter {
                    cutoff,
                    coefficient: OnePole::coefficient(cutoff, sample_rate as f32),
                    channels: [OnePole::default(); 2],
                }
            }
        }
    }

    /// Apply a modulated parameter value.
    ///
    /// Called once per parameter per reference frame.
    fn set_param(&mut self, name: &str, value: f32, sample_rate: f32) {
        match self {
            Self::Sine { frequency, .. } | Self::Lfo { frequency, .. } => set(frequency, name, "frequency", value),
            Self::Constant { value: held } => set(held, name, "value", value),
            Self::Gain { gain } | Self::StereoGain { gain } => set(gain, name, "gain", value),
            Self::Pan { pan } => set(pan, name, "pan", value),
            Self::Scale { factor } => set(factor, name, "factor", value),
            Self::Bias { offset } => set(offset, name, "offset", value),
            Self::Clamp { min, max } => {
                set(min, name, "min", value);
                set(max, name, "max", value);
            }
            Self::Smooth { time, .. } => set(time, name, "time", value),
            Self::PolySine { allocator } => allocator.set_voice(name, value),
            Self::Filter {
                kind,
                cutoff,
                q,
                target,
                ..
            } => {
                set(cutoff, name, "cutoff", value);
                set(q, name, "q", value);
                *target = Coefficients::new(*kind, *cutoff, *q, sample_rate);
            }
            Self::OnePoleFilter {
                cutoff, coefficient, ..
            } => {
                set(cutoff, name, "cutoff", value);
                *coefficient = OnePole::coefficient(*cutoff, sample_rate);
            }
            Self::Echo {
                time, feedback, mix, ..
            } => {
                set(time, name, "time", value);
                set(feedback, name, "feedback", value);
                set(mix, name, "mix", value);
            }
            Self::Chorusing { rate, depth, mix, .. } => {
                set(rate, name, "rate", value);
                set(depth, name, "depth", value);
                set(mix, name, "mix", value);
            }
            Self::Room { room, damping, mix, .. } => {
                set(room, name, "room", value);
                set(damping, name, "damping", value);
                set(mix, name, "mix", value);
            }
            Self::Ceiling { ceiling, .. } => set(ceiling, name, "ceiling", value),
            Self::Noise { .. }
            | Self::Passthrough
            | Self::Mixer
            | Self::Splitter
            | Self::MonoToStereo
            | Self::StereoToMono => {}
        }
    }
}

/// Assign `value` to `slot` when `name` is the parameter `slot` holds.
fn set(slot: &mut f32, name: &str, expected: &str, value: f32) {
    if name == expected {
        *slot = value;
    }
}

/// Compile a spec into a preallocated plan (roadmap §13.3: validation,
/// topological sort, buffer allocation — all hidden behind this call).
///
/// # Errors
/// [`GraphError`] for unknown nodes/ports, kind or channel mismatches,
/// duplicate inputs, cycles, or a missing output designation.
pub(crate) fn prepare_plan(spec: &StudioGraphSpec, options: &GraphOptions) -> Result<RenderPlan, GraphError> {
    let span = tracing::info_span!(
        "prepare_audio_primitives",
        nodes = spec.nodes().len(),
        connections = spec.connections().len(),
        sample_rate = options.sample_rate
    );
    let _entered = span.enter();
    validate(spec)?;

    let order = schedule_order(spec)?;

    // Only ancestors of the output render; the rest is dead weight (§13.3).
    let output = spec.output().ok_or(GraphError::OutputMissing)?;
    let mut reachable = std::collections::HashSet::from([output]);
    let mut grew = true;
    while grew {
        grew = false;
        for connection in spec.connections() {
            if reachable.contains(&connection.to) && reachable.insert(connection.from) {
                grew = true;
            }
        }
        // A modulator of a reachable node is itself reachable: it is not
        // heard, but it is the reason what is heard moves.
        for edge in spec.modulations() {
            if reachable.contains(&edge.to) && reachable.insert(edge.from) {
                grew = true;
            }
        }
    }

    // Buffers: index 0 is the shared zero buffer (silence for unconnected
    // inputs); then one buffer per output port of reachable nodes.
    const FRAME_WIDTH: usize = 1;
    let stride = FRAME_WIDTH;
    let mut buffers: Vec<Box<[f32]>> = vec![vec![0.0; stride].into_boxed_slice()];
    let mut port_buffers: std::collections::HashMap<(NodeId, usize), usize> = std::collections::HashMap::new();
    for node in spec.nodes() {
        if !reachable.contains(&node.id()) {
            continue;
        }
        for (port, kind) in spec
            .processor_of(node.id())
            .map_or(Vec::new(), |p| p.output_ports())
            .iter()
            .enumerate()
        {
            let channels = channels_of(*kind);
            buffers.push(vec![0.0; stride * channels].into_boxed_slice());
            port_buffers.insert((node.id(), port), buffers.len().saturating_sub(1));
        }
    }

    let mut schedule = Vec::new();
    for id in &order {
        if !reachable.contains(id) {
            continue;
        }
        let Some(processor) = spec.processor_of(*id) else {
            continue;
        };
        let inputs = processor
            .input_ports()
            .iter()
            .enumerate()
            .map(|(port, _)| {
                spec.connections()
                    .iter()
                    .find(|c| c.to == *id && c.to_port == port)
                    .and_then(|c| port_buffers.get(&(c.from, c.from_port)).copied())
                    .unwrap_or(0)
            })
            .collect();
        let outputs = processor
            .output_ports()
            .iter()
            .enumerate()
            .map(|(port, _)| port_buffers.get(&(*id, port)).copied().unwrap_or(0))
            .collect::<Vec<usize>>();
        let taken = Vec::with_capacity(outputs.len());
        let takes_events = processor.input_ports().contains(&PortKind::NoteEvents);
        let modulations = spec
            .modulations()
            .iter()
            .filter(|edge| edge.to == *id)
            .filter_map(|edge| {
                let descriptor = processor.descriptor(edge.param)?;
                let base = spec.param_value(*id, &descriptor);
                Some(ParamLink {
                    buffer: port_buffers.get(&(edge.from, edge.from_port)).copied().unwrap_or(0),
                    descriptor,
                    base,
                    current: base,
                    coefficient: frame_coefficient(descriptor.smoothing, options.sample_rate),
                })
            })
            .collect();
        schedule.push(Step {
            instance: ProcessorInstance::instantiate(spec, *id, processor, options.sample_rate, options.render_seed),
            inputs,
            outputs,
            taken,
            takes_events,
            modulations,
        });
    }

    let master = port_buffers.get(&(output, 0)).copied().unwrap_or(0);
    let master_channels = spec
        .processor_of(output)
        .and_then(|p| p.output_ports().first().copied())
        .map_or(1, channels_of);
    // Scheduled nodes are the reachable ones, so a schedule shorter than the
    // graph is dead weight the compiler dropped — which is correct, and which
    // is also what a patch that makes no sound looks like from here.
    tracing::debug!(
        scheduled = schedule.len(),
        buffers = buffers.len(),
        master_channels,
        "compiled a render plan"
    );
    Ok(RenderPlan {
        sample_rate: options.sample_rate,
        schedule,
        buffers,
        master,
        master_channels,
    })
}

/// The order the nodes run in: a topological sort (Kahn) of every edge except
/// the ones a legal feedback loop is allowed to defer.
///
/// **Feedback (§13.3, §5.6).** A cycle is legal exactly when it passes through
/// a [`ProcessorSpec::Delay`], and it is legal because of what a delay does: a
/// signal that comes back has been held for an amount the patch wrote, so the
/// loop is causal and its period is something a reader can see. A delay on a
/// cycle reads its input from the preceding reference frame. Any other cycle
/// is rejected, because there is no interpretation of it that is not "the
/// output before the output".
fn schedule_order(spec: &StudioGraphSpec) -> Result<Vec<NodeId>, GraphError> {
    // Edges into these nodes do not constrain the order: they are the ones
    // closing a loop, and the delay is where the loop is allowed to close.
    let deferred: std::collections::HashSet<NodeId> = spec
        .nodes()
        .iter()
        .filter(|node| node.processor() == ProcessorSpec::Delay && on_a_cycle(spec, node.id()))
        .map(|node| node.id())
        .collect();

    let edges = |from: NodeId| {
        spec.connections()
            .iter()
            .filter(move |c| c.from == from)
            .map(|c| c.to)
            // A modulation is an edge for scheduling purposes too: a control
            // signal must be computed before the node it steers reads it, or
            // the parameter lags the sound by a frame.
            .chain(spec.modulations().iter().filter(move |e| e.from == from).map(|e| e.to))
            .filter(|to| !deferred.contains(to))
    };

    let mut indegree: std::collections::HashMap<NodeId, usize> = std::collections::HashMap::new();
    for node in spec.nodes() {
        indegree.insert(node.id(), 0);
    }
    for node in spec.nodes() {
        for to in edges(node.id()) {
            *indegree.entry(to).or_insert(0) += 1;
        }
    }
    let mut queue: Vec<NodeId> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| *id)
        .collect();
    queue.sort_unstable();
    let mut order = Vec::new();
    while let Some(id) = queue.pop() {
        order.push(id);
        for to in edges(id) {
            if let Some(degree) = indegree.get_mut(&to) {
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    queue.push(to);
                }
            }
        }
    }
    if order.len() != spec.nodes().len() {
        return Err(GraphError::Cycle);
    }
    Ok(order)
}

/// Whether `start` can reach itself: the definition of being on a cycle.
fn on_a_cycle(spec: &StudioGraphSpec, start: NodeId) -> bool {
    let mut seen: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
    let mut frontier = vec![start];
    while let Some(id) = frontier.pop() {
        let downstream = spec
            .connections()
            .iter()
            .filter(|c| c.from == id)
            .map(|c| c.to)
            .chain(spec.modulations().iter().filter(|e| e.from == id).map(|e| e.to));
        for to in downstream {
            if to == start {
                return true;
            }
            if seen.insert(to) {
                frontier.push(to);
            }
        }
    }
    false
}

/// How far a smoothed parameter closes on its target in one frame.
fn frame_coefficient(smoothing: Smoothing, sample_rate: u32) -> f32 {
    const TIME_CONSTANT: f32 = 0.005;
    match smoothing {
        Smoothing::None => 1.0,
        Smoothing::FrameSlew => OnePole::time_coefficient(TIME_CONSTANT, sample_rate.max(1) as f32),
    }
}

/// Structural validation (§13.3): nodes, ports, kinds, duplicate inputs,
/// processor-local limits.
fn validate(spec: &StudioGraphSpec) -> Result<(), GraphError> {
    for node in spec.nodes() {
        if let ProcessorSpec::Mixer { inputs } = node.processor()
            && (inputs == 0 || inputs > 8)
        {
            return Err(GraphError::PortOutOfRange {
                node: node.id(),
                port: usize::from(inputs),
            });
        }
    }
    for connection in spec.connections() {
        check_connection(spec, connection)?;
    }
    for edge in spec.modulations() {
        let from = spec.processor_of(edge.from).ok_or(GraphError::UnknownNode(edge.from))?;
        let to = spec.processor_of(edge.to).ok_or(GraphError::UnknownNode(edge.to))?;
        let kind = from.output_ports().get(edge.from_port).copied();
        if kind != Some(PortKind::Control) {
            return Err(GraphError::PortMismatch {
                from: kind.map_or_else(|| "none".to_owned(), |kind| kind.to_string()),
                to: PortKind::Control.to_string(),
            });
        }
        if to.descriptor(edge.param).is_none() {
            return Err(GraphError::InvalidParameter {
                node: edge.to,
                name: edge.param.to_owned(),
            });
        }
    }
    let mut fed: std::collections::HashSet<(NodeId, usize)> = std::collections::HashSet::new();
    for connection in spec.connections() {
        if !fed.insert((connection.to, connection.to_port)) {
            return Err(GraphError::DuplicateInput {
                node: connection.to,
                port: connection.to_port,
            });
        }
    }
    Ok(())
}

fn check_connection(spec: &StudioGraphSpec, connection: &Connection) -> Result<(), GraphError> {
    let from_processor = spec
        .processor_of(connection.from)
        .ok_or(GraphError::UnknownNode(connection.from))?;
    let to_processor = spec
        .processor_of(connection.to)
        .ok_or(GraphError::UnknownNode(connection.to))?;
    let out_ports = from_processor.output_ports();
    let in_ports = to_processor.input_ports();
    let Some(from_kind) = out_ports.get(connection.from_port) else {
        return Err(GraphError::PortOutOfRange {
            node: connection.from,
            port: connection.from_port,
        });
    };
    let Some(to_kind) = in_ports.get(connection.to_port) else {
        return Err(GraphError::PortOutOfRange {
            node: connection.to,
            port: connection.to_port,
        });
    };
    if from_kind != to_kind {
        return Err(GraphError::PortMismatch {
            from: from_kind.to_string(),
            to: to_kind.to_string(),
        });
    }
    Ok(())
}

fn channels_of(kind: PortKind) -> usize {
    match kind {
        PortKind::Audio { channels } => usize::from(channels),
        PortKind::Control | PortKind::NoteEvents => 1,
    }
}

impl RenderPlan {
    /// Execute exactly one reference audio-frame step.
    ///
    /// Event input is applied before the processors produce this frame. Every
    /// processor, feedback edge, modulation source, and smoother therefore
    /// advances once regardless of the host callback partition.
    pub(crate) fn step(&mut self, events: &[EventMessage<Gesture>], tuning: Tuning) -> [f32; 2] {
        const FRAME_WIDTH: usize = 1;
        let sample_rate = f64::from(self.sample_rate);
        for step in &mut self.schedule {
            if step.takes_events {
                for event in events {
                    step.instance.apply_event(event, tuning);
                }
            }
            process_step(step, &mut self.buffers, 1, sample_rate, FRAME_WIDTH);
        }
        self.master_frame(FRAME_WIDTH)
    }

    fn master_frame(&self, channel_stride: usize) -> [f32; 2] {
        let Some(master) = self.buffers.get(self.master) else {
            return [0.0; 2];
        };
        let left = master.first().copied().unwrap_or(0.0);
        let right = if self.master_channels > 1 {
            master.get(channel_stride).copied().unwrap_or(0.0)
        } else {
            left
        };
        [finite(left), finite(right)]
    }
}

fn ratio_to_f32(value: num_rational::Ratio<i64>) -> f32 {
    *value.numer() as f32 / *value.denom() as f32
}

fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

/// Apply this frame's modulations to a node's parameters (§13.7).
///
/// Everything
/// the parameter says about the value happens here and in this order —
/// combine, clamp, smooth — so a modulated parameter can never leave its
/// declared range and never arrives as a step.
fn apply_modulations(step: &mut Step, buffers: &[Box<[f32]>], sample_rate: f32) {
    for link in &mut step.modulations {
        let signal = buffers.get(link.buffer).and_then(|buffer| buffer.first()).copied();
        // A control value that is not a number is not an instruction: the
        // parameter keeps what the patch wrote rather than propagating a NaN
        // into a filter coefficient (§17.5's NaN-freedom).
        let signal = signal.filter(|value| value.is_finite()).unwrap_or(link.base);
        let combined = match link.descriptor.combination {
            Combination::Replace => signal,
            Combination::Multiply => link.base * signal,
        };
        let (low, high) = link.descriptor.range;
        let target = if combined.is_finite() {
            combined.clamp(low, high)
        } else {
            link.base
        };
        link.current = link.coefficient.mul_add(target - link.current, link.current);
        step.instance.set_param(link.descriptor.name, link.current, sample_rate);
    }
}

/// Dry and wet in the balance a `mix` parameter asks for: 0 is the input
/// untouched, 1 is the effect alone.
fn blend(dry: f32, wet: f32, mix: f32) -> f32 {
    let mix = mix.clamp(0.0, 1.0);
    mix.mul_add(wet - dry, dry)
}

/// One period of an LFO shape at `phase` in `[0, 1)`.
fn waveform_value(waveform: Waveform, phase: f64) -> f32 {
    match waveform {
        Waveform::Sine => (std::f64::consts::TAU * phase).sin() as f32,
        Waveform::Triangle => 4.0f64.mul_add(-(phase - 0.5).abs(), 1.0) as f32,
        Waveform::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
    }
}

/// Run one processor for `count` frames. Output buffers move out of the
/// arena, are written, and move back — no heap traffic.
fn process_step(step: &mut Step, buffers: &mut [Box<[f32]>], count: usize, sample_rate: f64, channel_stride: usize) {
    apply_modulations(step, buffers, sample_rate as f32);
    step.taken.clear();
    for &index in &step.outputs {
        if let Some(buffer) = buffers.get_mut(index) {
            step.taken.push(std::mem::take(buffer));
        }
    }
    let mut inputs: [Option<&[f32]>; 8] = [None; 8];
    for (port, &index) in step.inputs.iter().enumerate() {
        if let Some(slot) = inputs.get_mut(port) {
            *slot = buffers.get(index).map(|b| b.as_ref());
        }
    }
    process(
        &mut step.instance,
        &inputs,
        &mut step.taken,
        count,
        sample_rate,
        channel_stride,
    );
    for (&index, buffer) in step.outputs.iter().zip(step.taken.drain(..)) {
        if let Some(slot) = buffers.get_mut(index) {
            *slot = buffer;
        }
    }
}

fn process(
    instance: &mut ProcessorInstance,
    inputs: &[Option<&[f32]>],
    outputs: &mut [Box<[f32]>],
    count: usize,
    sample_rate: f64,
    channel_stride: usize,
) {
    let input = |port: usize, i: usize| {
        inputs
            .get(port)
            .copied()
            .flatten()
            .and_then(|b: &[f32]| b.get(i))
            .copied()
            .unwrap_or(0.0)
    };
    let channel = |port: usize| inputs.get(port).copied().flatten();
    match instance {
        ProcessorInstance::Sine { phase, frequency } => {
            let increment = f64::from(*frequency) / sample_rate;
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let sample = (2.0 * std::f64::consts::PI * *phase).sin() as f32;
                    *phase += increment;
                    if *phase >= 1.0 {
                        *phase -= 1.0;
                    }
                    if let Some(slot) = out.get_mut(i) {
                        *slot = sample;
                    }
                }
            }
        }
        ProcessorInstance::Noise { state } => {
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    *state ^= *state << 13;
                    *state ^= *state >> 17;
                    *state ^= *state << 5;
                    let sample = (f64::from(*state) / f64::from(u32::MAX)).mul_add(2.0, -1.0) as f32;
                    if let Some(slot) = out.get_mut(i) {
                        *slot = sample;
                    }
                }
            }
        }
        ProcessorInstance::Constant { value } => {
            if let Some(out) = outputs.first_mut() {
                for slot in out.iter_mut().take(count) {
                    *slot = *value;
                }
            }
        }
        ProcessorInstance::Gain { gain } => {
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    if let Some(slot) = out.get_mut(i) {
                        *slot = input(0, i) * *gain;
                    }
                }
            }
        }
        ProcessorInstance::StereoGain { gain } => {
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let right = channel_stride.saturating_add(i);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = input(0, i) * *gain;
                    }
                    if let Some(slot) = out.get_mut(right) {
                        *slot = channel(0).and_then(|buffer| buffer.get(right)).copied().unwrap_or(0.0) * *gain;
                    }
                }
            }
        }
        ProcessorInstance::Passthrough => {
            // Every channel copied, so a stage whose DSP does not exist yet
            // is silent about itself rather than silent full stop.
            if let Some(out) = outputs.first_mut()
                && let Some(buffer) = channel(0)
            {
                let width = out.len().min(buffer.len());
                if let (Some(target), Some(source)) = (out.get_mut(..width), buffer.get(..width)) {
                    target.copy_from_slice(source);
                }
            }
        }
        ProcessorInstance::Pan { pan } => {
            let angle = (f64::from(*pan) + 1.0) * std::f64::consts::FRAC_PI_4;
            let (left_gain, right_gain) = (angle.cos() as f32, angle.sin() as f32);
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let mono = input(0, i);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = mono * left_gain;
                    }
                    if let Some(slot) = out.get_mut(channel_stride.saturating_add(i)) {
                        *slot = mono * right_gain;
                    }
                }
            }
        }
        ProcessorInstance::Mixer => {
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let mut left = 0.0f32;
                    let mut right = 0.0f32;
                    for port in 0..inputs.len() {
                        if let Some(buffer) = channel(port) {
                            left += buffer.get(i).copied().unwrap_or(0.0);
                            right += buffer.get(channel_stride.saturating_add(i)).copied().unwrap_or(0.0);
                        }
                    }
                    if let Some(slot) = out.get_mut(i) {
                        *slot = left;
                    }
                    if let Some(slot) = out.get_mut(channel_stride.saturating_add(i)) {
                        *slot = right;
                    }
                }
            }
        }
        ProcessorInstance::Splitter => {
            for out in outputs.iter_mut().take(2) {
                for i in 0..count {
                    if let Some(slot) = out.get_mut(i) {
                        *slot = input(0, i);
                    }
                }
            }
        }
        ProcessorInstance::MonoToStereo => {
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let mono = input(0, i);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = mono;
                    }
                    if let Some(slot) = out.get_mut(channel_stride.saturating_add(i)) {
                        *slot = mono;
                    }
                }
            }
        }
        ProcessorInstance::PolySine { allocator } => {
            // Render the voice pool mono into the left half, then mirror.
            if let Some(out) = outputs.first_mut() {
                let (left, right) = out.split_at_mut(channel_stride);
                allocator.render(left, count, sample_rate);
                let (source, target) = (left.get(..count), right.get_mut(..count));
                if let (Some(source), Some(target)) = (source, target) {
                    target.copy_from_slice(source);
                }
            }
        }
        ProcessorInstance::Lfo {
            waveform,
            phase,
            frequency,
        } => {
            // One value for this reference frame.
            let value = waveform_value(*waveform, *phase);
            *phase += f64::from(*frequency) * count as f64 / sample_rate;
            *phase -= phase.floor();
            if let Some(out) = outputs.first_mut() {
                for slot in out.iter_mut().take(count) {
                    *slot = value;
                }
            }
        }
        ProcessorInstance::Scale { factor } => {
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    if let Some(slot) = out.get_mut(i) {
                        *slot = input(0, i) * *factor;
                    }
                }
            }
        }
        ProcessorInstance::Bias { offset } => {
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    if let Some(slot) = out.get_mut(i) {
                        *slot = input(0, i) + *offset;
                    }
                }
            }
        }
        ProcessorInstance::Clamp { min, max } => {
            let (low, high) = (min.min(*max), max.max(*min));
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    if let Some(slot) = out.get_mut(i) {
                        *slot = input(0, i).clamp(low, high);
                    }
                }
            }
        }
        ProcessorInstance::Smooth { time, state } => {
            let coefficient = OnePole::time_coefficient(*time, sample_rate as f32);
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let smoothed = state.process(input(0, i), coefficient);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = smoothed;
                    }
                }
            }
        }
        ProcessorInstance::Filter {
            coefficients,
            target,
            channels,
            ..
        } => {
            // The response walks to what this frame's parameters ask for;
            // see `Coefficients`.
            let step = coefficients.step_to(target, count);
            let [left_state, right_state] = channels;
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let right_index = channel_stride.saturating_add(i);
                    coefficients.advance(&step);
                    let left = left_state.process(input(0, i), coefficients);
                    let right = right_state.process(
                        channel(0).and_then(|b| b.get(right_index)).copied().unwrap_or(0.0),
                        coefficients,
                    );
                    if let Some(slot) = out.get_mut(i) {
                        *slot = left;
                    }
                    if let Some(slot) = out.get_mut(right_index) {
                        *slot = right;
                    }
                }
            }
            // End the frame exactly on target, so a long sweep cannot drift.
            *coefficients = *target;
        }
        ProcessorInstance::OnePoleFilter {
            coefficient, channels, ..
        } => {
            let [left_state, right_state] = channels;
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let right_index = channel_stride.saturating_add(i);
                    let left = left_state.process(input(0, i), *coefficient);
                    let right = right_state.process(
                        channel(0).and_then(|b| b.get(right_index)).copied().unwrap_or(0.0),
                        *coefficient,
                    );
                    if let Some(slot) = out.get_mut(i) {
                        *slot = left;
                    }
                    if let Some(slot) = out.get_mut(right_index) {
                        *slot = right;
                    }
                }
            }
        }
        ProcessorInstance::Echo {
            time,
            feedback,
            mix,
            channels,
        } => {
            let [left_state, right_state] = channels;
            let rate = sample_rate as f32;
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let right_index = channel_stride.saturating_add(i);
                    let dry_left = input(0, i);
                    let dry_right = channel(0).and_then(|b| b.get(right_index)).copied().unwrap_or(0.0);
                    let wet_left = left_state.process(dry_left, *time, *feedback, rate);
                    let wet_right = right_state.process(dry_right, *time, *feedback, rate);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = blend(dry_left, wet_left, *mix);
                    }
                    if let Some(slot) = out.get_mut(right_index) {
                        *slot = blend(dry_right, wet_right, *mix);
                    }
                }
            }
        }
        ProcessorInstance::Chorusing {
            rate,
            depth,
            mix,
            channels,
        } => {
            let [left_state, right_state] = channels;
            let sample_rate = sample_rate as f32;
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let right_index = channel_stride.saturating_add(i);
                    let dry_left = input(0, i);
                    let dry_right = channel(0).and_then(|b| b.get(right_index)).copied().unwrap_or(0.0);
                    let wet_left = left_state.process(dry_left, *rate, *depth, sample_rate);
                    let wet_right = right_state.process(dry_right, *rate, *depth, sample_rate);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = blend(dry_left, wet_left, *mix);
                    }
                    if let Some(slot) = out.get_mut(right_index) {
                        *slot = blend(dry_right, wet_right, *mix);
                    }
                }
            }
        }
        ProcessorInstance::Room {
            room,
            damping,
            mix,
            channels,
        } => {
            let [left_state, right_state] = channels;
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let right_index = channel_stride.saturating_add(i);
                    let dry_left = input(0, i);
                    let dry_right = channel(0).and_then(|b| b.get(right_index)).copied().unwrap_or(0.0);
                    // Both networks are fed the sum: a reverb is a room, and
                    // a room does not have a left half and a right half.
                    let source = dry_left + dry_right;
                    let wet_left = left_state.process(source, *room, *damping);
                    let wet_right = right_state.process(source, *room, *damping);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = blend(dry_left, wet_left, *mix);
                    }
                    if let Some(slot) = out.get_mut(right_index) {
                        *slot = blend(dry_right, wet_right, *mix);
                    }
                }
            }
        }
        ProcessorInstance::Ceiling { ceiling, limiter } => {
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let right_index = channel_stride.saturating_add(i);
                    let left = input(0, i);
                    let right = channel(0).and_then(|b| b.get(right_index)).copied().unwrap_or(0.0);
                    let (limited_left, limited_right) = limiter.process(left, right, *ceiling);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = limited_left;
                    }
                    if let Some(slot) = out.get_mut(right_index) {
                        *slot = limited_right;
                    }
                }
            }
        }
        ProcessorInstance::StereoToMono => {
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let left = input(0, i);
                    let right = channel(0)
                        .and_then(|b| b.get(channel_stride.saturating_add(i)).copied())
                        .unwrap_or(0.0);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = left.midpoint(right);
                    }
                }
            }
        }
    }
}
