//! The compiled render plan: validation, topological scheduling, buffer
//! arena, and the processor set (roadmap §13.3–§13.5, §13.8).
//!
//! Real-time contract (§13.2): everything is allocated in `compile_graph`;
//! `RenderPlan::render` never allocates, locks, or does I/O. Output buffers
//! are moved out of the arena, written, and moved back (`Box<[f32]>` moves
//! without touching the heap).
//!
//! FP arithmetic is inherent to DSP (§13); the workspace arithmetic lint is
//! allowed at module scope. Sample loops index by bound-checked cursor —
//! bounds are a compile-time invariant (port lists are fixed at compile).
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::PerformanceEvent;

use crate::error::GraphError;
use crate::spec::{Connection, GraphOptions, NodeId, PortKind, ProcessorSpec, StudioGraphSpec};
use crate::voice::VoiceAllocator;

/// A slice of scheduled performance events for one render call.
///
/// Nothing consumes events yet — note-event → oscillator routing arrives in
/// prompt 17; the type is part of the render signature now (§13.8: offline
/// and live rendering execute the same `render`) so prompt 17 adds routing
/// without breaking the facade.
#[derive(Clone, Copy, Debug, Default)]
pub struct EventSlice<'a> {
    events: &'a [PerformanceEvent],
}

impl<'a> EventSlice<'a> {
    /// Wrap scheduled events.
    pub fn new(events: &'a [PerformanceEvent]) -> Self {
        Self { events }
    }

    /// An empty slice.
    pub fn empty() -> EventSlice<'static> {
        EventSlice { events: &[] }
    }

    /// The events.
    pub fn events(&self) -> &[PerformanceEvent] {
        self.events
    }
}

/// A compiled, preallocated graph execution plan.
pub struct RenderPlan {
    options: GraphOptions,
    schedule: Vec<Step>,
    buffers: Vec<Box<[f32]>>,
    /// Buffer index of the master output (stereo planar or mono).
    master: usize,
    master_channels: usize,
    /// Absolute frames rendered so far (event dispatch window base).
    cursor: u64,
    sample_rate: u32,
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
}

enum ProcessorInstance {
    Sine { phase: f64, frequency: f32 },
    Noise { state: u32 },
    Constant { value: f32 },
    Gain { gain: f32 },
    Pan { pan: f32 },
    Mixer,
    Splitter,
    MonoToStereo,
    StereoToMono,
    PolySine { allocator: VoiceAllocator },
}

impl ProcessorInstance {
    /// Deliver a scheduled event. Only event-consuming processors act.
    fn apply_event(&mut self, event: &PerformanceEvent) {
        let Self::PolySine { allocator } = self else {
            return;
        };
        match event {
            PerformanceEvent::NoteOn { note, instance, .. } => {
                allocator.note_on(*instance, note.frequency as f32, note.amplitude);
            }
            PerformanceEvent::NoteOff { instance, .. } => allocator.note_off(*instance),
            PerformanceEvent::Parameter { .. } => {}
        }
    }

    fn instantiate(spec: &StudioGraphSpec, node: NodeId, processor: ProcessorSpec, sample_rate: u32) -> Self {
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
            ProcessorSpec::Noise => Self::Noise {
                state: 0x9E37_79B9u32.wrapping_add(node.0),
            },
            ProcessorSpec::Constant => Self::Constant { value: param("value") },
            ProcessorSpec::Gain => Self::Gain { gain: param("gain") },
            ProcessorSpec::Pan => Self::Pan { pan: param("pan") },
            ProcessorSpec::Mixer { .. } => Self::Mixer,
            ProcessorSpec::PolySine { voices } => Self::PolySine {
                allocator: VoiceAllocator::new(voices, sample_rate),
            },
            ProcessorSpec::Splitter => Self::Splitter,
            ProcessorSpec::MonoToStereo => Self::MonoToStereo,
            ProcessorSpec::StereoToMono => Self::StereoToMono,
        }
    }
}

/// Compile a spec into a preallocated plan (roadmap §13.3: validation,
/// topological sort, buffer allocation — all hidden behind this call).
///
/// # Errors
/// [`GraphError`] for unknown nodes/ports, kind or channel mismatches,
/// duplicate inputs, cycles, or a missing output designation.
pub fn compile_graph(spec: &StudioGraphSpec, options: &GraphOptions) -> Result<RenderPlan, GraphError> {
    validate(spec)?;

    // Topological order (Kahn). A cycle anywhere is rejected: cycles are
    // legal only through an explicit delay node (§5.6), which does not exist
    // yet — the rule is encoded, not just the current case.
    let mut indegree: std::collections::HashMap<NodeId, usize> = std::collections::HashMap::new();
    for node in spec.nodes() {
        indegree.insert(node.id(), 0);
    }
    for connection in spec.connections() {
        *indegree.entry(connection.to).or_insert(0) += 1;
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
        for connection in spec.connections().iter().filter(|c| c.from == id) {
            if let Some(degree) = indegree.get_mut(&connection.to) {
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    queue.push(connection.to);
                }
            }
        }
    }
    if order.len() != spec.nodes().len() {
        return Err(GraphError::Cycle);
    }

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
    }

    // Buffers: index 0 is the shared zero buffer (silence for unconnected
    // inputs); then one buffer per output port of reachable nodes.
    let block = options.block_size;
    let mut buffers: Vec<Box<[f32]>> = vec![vec![0.0; block].into_boxed_slice()];
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
            buffers.push(vec![0.0; block * channels].into_boxed_slice());
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
        schedule.push(Step {
            instance: ProcessorInstance::instantiate(spec, *id, processor, options.sample_rate),
            inputs,
            outputs,
            taken,
            takes_events,
        });
    }

    let master = port_buffers.get(&(output, 0)).copied().unwrap_or(0);
    let master_channels = spec
        .processor_of(output)
        .and_then(|p| p.output_ports().first().copied())
        .map_or(1, channels_of);
    Ok(RenderPlan {
        options: *options,
        schedule,
        buffers,
        master,
        master_channels,
        cursor: 0,
        sample_rate: options.sample_rate,
    })
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
        PortKind::Control | PortKind::Gate | PortKind::NoteEvents => 1,
    }
}

impl RenderPlan {
    /// Render `frames` samples with `events` scheduled, interleaved stereo
    /// into `output` (§13.8: the same function a live stream will call).
    /// Real-time-safe: no allocation, no locks (§13.2).
    pub fn render(&mut self, events: &EventSlice<'_>, output: &mut [f32], frames: usize) {
        let _ = events;
        let block = self.options.block_size;
        let sample_rate = f64::from(self.options.sample_rate);
        let mut written = 0;
        while written < frames {
            let count = (frames - written).min(block);
            let window_start = self.cursor;
            let window_end = window_start.saturating_add(count as u64);
            for step in &mut self.schedule {
                if step.takes_events {
                    for event in window(events.events(), window_start, window_end) {
                        step.instance.apply_event(event);
                    }
                }
                process_step(step, &mut self.buffers, count, sample_rate, block);
            }
            self.copy_master(output, written, count, block);
            written = written.saturating_add(count);
            self.cursor = window_end;
        }
    }

    /// The plan's sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The absolute frame the event-dispatch window is based on.
    pub fn cursor(&self) -> u64 {
        self.cursor
    }

    /// Move the event-dispatch cursor (transport seek/loop). Sounding
    /// voices are left alone — this moves the schedule, not the DSP state.
    pub fn seek(&mut self, frame: u64) {
        self.cursor = frame;
    }

    /// Interleave the master buffer into `output[2*written .. 2*(written+count)]`.
    fn copy_master(&self, output: &mut [f32], written: usize, count: usize, block: usize) {
        let Some(master) = self.buffers.get(self.master) else {
            return;
        };
        for i in 0..count {
            let left = master.get(i).copied().unwrap_or(0.0);
            let right = if self.master_channels > 1 {
                master.get(block.saturating_add(i)).copied().unwrap_or(0.0)
            } else {
                left
            };
            if let Some(slot) = output.get_mut(2 * (written.saturating_add(i))) {
                *slot = left;
            }
            if let Some(slot) = output.get_mut((2 * (written.saturating_add(i))).saturating_add(1)) {
                *slot = right;
            }
        }
    }
}

/// Events with frames in `[start, end)` (slice is sorted by frame).
fn window(events: &[PerformanceEvent], start: u64, end: u64) -> &[PerformanceEvent] {
    let from = events.partition_point(|event| event.frame() < start);
    let to = events.partition_point(|event| event.frame() < end);
    events.get(from..to).unwrap_or(&[])
}

/// Run one processor for `count` frames. Output buffers move out of the
/// arena, are written, and move back — no heap traffic.
fn process_step(step: &mut Step, buffers: &mut [Box<[f32]>], count: usize, sample_rate: f64, block: usize) {
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
    process(&mut step.instance, &inputs, &mut step.taken, count, sample_rate, block);
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
    block: usize,
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
        ProcessorInstance::Pan { pan } => {
            let angle = (f64::from(*pan) + 1.0) * std::f64::consts::FRAC_PI_4;
            let (left_gain, right_gain) = (angle.cos() as f32, angle.sin() as f32);
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let mono = input(0, i);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = mono * left_gain;
                    }
                    if let Some(slot) = out.get_mut(block.saturating_add(i)) {
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
                            right += buffer.get(block.saturating_add(i)).copied().unwrap_or(0.0);
                        }
                    }
                    if let Some(slot) = out.get_mut(i) {
                        *slot = left;
                    }
                    if let Some(slot) = out.get_mut(block.saturating_add(i)) {
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
                    if let Some(slot) = out.get_mut(block.saturating_add(i)) {
                        *slot = mono;
                    }
                }
            }
        }
        ProcessorInstance::PolySine { allocator } => {
            // Render the voice pool mono into the left half, then mirror.
            if let Some(out) = outputs.first_mut() {
                let (left, right) = out.split_at_mut(block);
                allocator.render(left, count, sample_rate);
                let (source, target) = (left.get(..count), right.get_mut(..count));
                if let (Some(source), Some(target)) = (source, target) {
                    target.copy_from_slice(source);
                }
            }
        }
        ProcessorInstance::StereoToMono => {
            if let Some(out) = outputs.first_mut() {
                for i in 0..count {
                    let left = input(0, i);
                    let right = channel(0)
                        .and_then(|b| b.get(block.saturating_add(i)).copied())
                        .unwrap_or(0.0);
                    if let Some(slot) = out.get_mut(i) {
                        *slot = left.midpoint(right);
                    }
                }
            }
        }
    }
}
