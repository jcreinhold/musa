//! Studio spec → graph spec (roadmap §6.5, §13.4).
//!
//! The compiler resolves a `studio` block into names, nodes, and connections;
//! this module turns that intent into the node graph the render plan is built
//! from. It is the one place where a written `-18 dB` becomes a linear gain
//! and a patch stage becomes a processor.
//!
//! **What a patch means here.** A patch describes one instrument's voice, and
//! the voice architecture it describes — oscillators feeding an envelope
//! feeding a filter — is prompts 30–31's work. Until then the note-producing
//! head of every patch is the existing polyphonic sine, and each stage the
//! patch writes downstream of it becomes a real processor if one exists and a
//! [`ProcessorSpec::Passthrough`] if it does not. The graph therefore has the
//! shape the patch describes from the first day, and filling it in later
//! changes processors, not topology.
//!
//! **One note stream.** Every patch reads the same scheduled events, because
//! that is what [`crate::RenderPlan`] delivers today. A studio that assigns
//! two parts to two different patches gets both patches sounding both parts;
//! [`lower_studio`] says so rather than pretending otherwise.

use musa_compiler::{Processor, StudioNode, StudioSpec};

use crate::spec::{GraphOptions, NodeId, ProcessorSpec, StudioGraphSpec};

/// What a lowering produced besides the graph.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StudioLowering {
    /// Things the studio asked for that the graph could not honour exactly.
    /// Not errors: the graph renders, and the caller decides whether to show
    /// them (the CLI prints them; the desktop app surfaces them in the Sound
    /// workspace).
    pub notes: Vec<String>,
}

/// The number of voices each patch's synthesizer gets.
const POLYPHONY: u8 = 16;

/// Mixer inputs, which bounds how many patches and buses can meet at master.
const MAX_MIX: usize = 8;

/// Lower a resolved studio into a graph.
///
/// Returns the graph and whatever the lowering had to say about it. An empty
/// studio produces the default instrument graph, which is what makes a piece
/// with no `studio` block render exactly as it did before studios existed
/// (§14.8).
pub fn lower_studio(studio: &StudioSpec, _options: &GraphOptions) -> (StudioGraphSpec, StudioLowering) {
    let mut lowering = StudioLowering::default();
    if studio.is_empty() {
        return (crate::poly_sine_spec(POLYPHONY), lowering);
    }

    let mut graph = StudioGraphSpec::new();
    // Every patch shares one note source until the engine can route events
    // per part. Said once, here, rather than left for a user to discover.
    let source = graph.add_node(ProcessorSpec::PolySine { voices: POLYPHONY });
    let assigned: Vec<&str> = distinct_patches(studio);
    if assigned.len() > 1 {
        lowering.notes.push(format!(
            "{} patches share one note stream: every assigned part sounds through all of them",
            assigned.len()
        ));
    }

    // Each assigned patch: the shared source through the patch's stages.
    let mut master_inputs: Vec<NodeId> = Vec::new();
    let mut patch_outputs: Vec<(&str, NodeId)> = Vec::new();
    for name in &assigned {
        let Some(patch) = studio.patch(name) else { continue };
        let tail = lower_stages(&mut graph, source, patch.nodes(), patch.output(), &mut lowering);
        patch_outputs.push((name, tail));
    }

    // Buses: their stages fed by the sends that name them.
    let mut bus_outputs: Vec<(&str, NodeId)> = Vec::new();
    for (name, bus) in studio.buses() {
        let mut feeds: Vec<NodeId> = Vec::new();
        for send in studio.sends().iter().filter(|send| send.bus == name) {
            let Some(from) = source_output(studio, &send.source, &patch_outputs) else {
                continue;
            };
            let attenuated = graph.add_node(ProcessorSpec::StereoGain);
            set_gain(&mut graph, attenuated, send.level.as_linear(), &mut lowering);
            graph.connect(from, 0, attenuated, 0);
            feeds.push(attenuated);
        }
        let Some(input) = merge(&mut graph, &feeds, &mut lowering) else {
            continue;
        };
        let tail = lower_stages(&mut graph, input, bus.nodes(), bus.output(), &mut lowering);
        bus_outputs.push((name, tail));
    }

    // Routes decide what reaches master. A studio that routes nothing is
    // silent, and that is the studio's statement, not a bug to paper over.
    for route in studio.routes() {
        if route.destination != "master" {
            continue;
        }
        let from = source_output(studio, &route.source, &patch_outputs).or_else(|| {
            bus_outputs
                .iter()
                .find(|(name, _)| *name == route.source)
                .map(|(_, id)| *id)
        });
        if let Some(from) = from {
            master_inputs.push(from);
        }
    }
    if master_inputs.is_empty() {
        lowering
            .notes
            .push("nothing is routed to `master`, so the studio renders silence".to_owned());
    }

    match merge(&mut graph, &master_inputs, &mut lowering) {
        Some(master) => graph.set_output(master),
        None => {
            // Keep the graph renderable: a silent master is still a master.
            let silent = graph.add_node(ProcessorSpec::Passthrough { channels: 2 });
            graph.set_output(silent);
        }
    }
    (graph, lowering)
}

/// Set a node's linear gain, recording rather than swallowing a refusal: a
/// rejected parameter means the graph is not what the studio asked for, and
/// silence about that is how a mix goes wrong invisibly.
fn set_gain(graph: &mut StudioGraphSpec, node: NodeId, linear: f64, lowering: &mut StudioLowering) {
    if graph.set_param(node, "gain", linear as f32).is_err() {
        lowering.notes.push(format!("a gain of {linear} could not be applied"));
    }
}

/// The patches parts are actually assigned to, in assignment order and
/// without repeats.
fn distinct_patches(studio: &StudioSpec) -> Vec<&str> {
    let mut seen: Vec<&str> = Vec::new();
    for (_, patch) in studio.assignments() {
        if !seen.contains(&patch) {
            seen.push(patch);
        }
    }
    seen
}

/// Where a `route`/`send` source's signal comes from: a bus by that name, or
/// the patch the named part is assigned to.
fn source_output(studio: &StudioSpec, name: &str, patches: &[(&str, NodeId)]) -> Option<NodeId> {
    let patch = studio.patch_for_part(name)?;
    patches
        .iter()
        .find(|(candidate, _)| *candidate == patch)
        .map(|(_, id)| *id)
}

/// Chain a patch's stages downstream of `input`, returning the last node.
///
/// Only the stages between the patch's note source and its output are
/// lowered: the source section (oscillators, their mix) is what the
/// polyphonic synth already is.
fn lower_stages(
    graph: &mut StudioGraphSpec,
    input: NodeId,
    nodes: &[StudioNode],
    output: Option<usize>,
    lowering: &mut StudioLowering,
) -> NodeId {
    let mut previous = input;
    let last = output.unwrap_or_else(|| nodes.len().saturating_sub(1));
    for (index, node) in nodes.iter().enumerate() {
        if index > last || is_source_stage(node) {
            continue;
        }
        let processor = match node.processor {
            Processor::Gain => ProcessorSpec::StereoGain,
            Processor::Envelope | Processor::Lowpass | Processor::Reverb | Processor::Scale | Processor::Bias => {
                lowering
                    .notes
                    .push(format!("`{}` renders as pass-through", node.processor.name()));
                ProcessorSpec::Passthrough { channels: 2 }
            }
            Processor::Oscillator | Processor::Mix => continue,
        };
        let id = graph.add_node(processor);
        if node.processor == Processor::Gain
            && let Some(level) = node.params.first()
        {
            set_gain(graph, id, level.as_linear(), lowering);
        }
        graph.connect(previous, 0, id, 0);
        previous = id;
    }
    previous
}

/// Whether a stage produces sound rather than shaping it. Those are the
/// stages the polyphonic synth stands in for.
fn is_source_stage(node: &StudioNode) -> bool {
    matches!(node.processor, Processor::Oscillator | Processor::Mix)
}

/// Sum several stereo signals, skipping the mixer when there is only one.
fn merge(graph: &mut StudioGraphSpec, inputs: &[NodeId], lowering: &mut StudioLowering) -> Option<NodeId> {
    match inputs {
        [] => None,
        [only] => Some(*only),
        many => {
            let used = many.len().min(MAX_MIX);
            if many.len() > MAX_MIX {
                lowering.notes.push(format!(
                    "a mixer takes {MAX_MIX} inputs; {} were dropped",
                    many.len().saturating_sub(MAX_MIX)
                ));
            }
            let mixer = graph.add_node(ProcessorSpec::Mixer {
                inputs: u8::try_from(used).unwrap_or(1),
            });
            for (port, input) in many.iter().take(used).enumerate() {
                graph.connect(*input, 0, mixer, port);
            }
            Some(mixer)
        }
    }
}
