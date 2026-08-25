//! Studio spec → graph spec (roadmap §6.5, §13.4).
//!
//! The compiler resolves a `studio` block into names, nodes, and connections;
//! this module turns that intent into the node graph the render plan is built
//! from. It is the one place where a written `-18 dB` becomes a linear gain
//! and a patch stage becomes a processor.
//!
//! **What a patch means here.** A patch describes one instrument's voice.
//! Its *source section* — the oscillators and the mix that combines them —
//! is what the polyphonic synth stands in for, so those stages become the
//! synth's settings rather than nodes of their own; everything downstream of
//! the source becomes a processor. An `envelope` stage is the same kind of
//! thing: an amplitude envelope is per-voice (§13.5), so it configures the
//! synth's ADSR instead of appearing as a node after it, which is the only
//! placement that makes two overlapping notes fade independently.
//!
//! **The spine.** Only the stages on the path from the source to `output`
//! become the patch's chain; the rest of the patch feeds that path from the
//! side and belongs to the source section. Walking backwards from `output`
//! is what keeps `shimmer = oscillator(...) |> gain(-15 dB)` a quieter
//! partial rather than a 15 dB cut on the whole instrument.
//!
//! **One note stream.** Every patch reads the same scheduled events, because
//! that is what the private one-frame plan delivers today. A studio that assigns
//! two parts to two different patches gets both patches sounding both parts;
//! [`lower_studio`] says so rather than pretending otherwise.

use crate::spec::{FilterKind, GraphOptions, NodeId, ProcessorSpec, StudioGraphSpec, Waveform};
use crate::{NodeIndex, Patch, Processor, StudioNode, StudioSpec};

/// What a lowering produced besides the graph.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StudioLowering {
    /// Things the studio asked for that the graph could not honour exactly.
    /// Not errors: the graph renders, and the caller decides whether to show
    /// them (the CLI prints them; the desktop app surfaces them in the Sound
    /// workspace).
    pub notes: Vec<String>,
    /// The longest release any patch asks for, in seconds.
    ///
    /// An offline render needs it to know when the piece is over. A fixed
    /// tail was enough while every note stopped when it ended; a patch with
    /// a 3.5 s release would be cut off mid-fade by one.
    pub release_tail: f32,
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
        return (crate::instrument::poly_sine_spec(POLYPHONY), lowering);
    }

    let mut graph = StudioGraphSpec::new();
    let assigned: Vec<&str> = distinct_patches(studio);
    if assigned.len() > 1 {
        // Every synth sees every note until the engine routes events per
        // part. Said once, here, rather than left for a user to discover.
        lowering.notes.push(format!(
            "{} patches share one note stream: every assigned part sounds through all of them",
            assigned.len()
        ));
    }

    // Each assigned patch: its own synth through its own stages.
    let mut master_inputs: Vec<NodeId> = Vec::new();
    let mut patch_outputs: Vec<(&str, NodeId)> = Vec::new();
    let mut addresses: Addresses<'_> = Vec::new();
    for name in &assigned {
        let Some(patch) = studio.patch(name) else { continue };
        let synth = graph.add_node(ProcessorSpec::PolySine { voices: POLYPHONY });
        lower_bank(&mut graph, synth, patch, &mut lowering);
        let tail = lower_container(
            &mut graph,
            synth,
            Some(synth),
            patch,
            name,
            &mut addresses,
            &mut lowering,
        );
        patch_outputs.push((name, tail));
    }

    // Buses: their stages fed by the sends that name them.
    let mut bus_outputs: Vec<(&str, NodeId)> = Vec::new();
    for (name, bus) in studio.buses() {
        // Two parts sharing a patch share its signal, so two sends of that
        // signal are one send: adding it twice would put the patch into the
        // bus at double strength for a reason nobody wrote. The loudest
        // level written wins, and the lowering says so.
        let mut levels: Vec<(NodeId, f64)> = Vec::new();
        for send in studio.sends().iter().filter(|send| send.bus == name) {
            let Some(from) = source_output(studio, &send.source, &patch_outputs) else {
                continue;
            };
            let level = send.level.as_linear();
            match levels.iter_mut().find(|(node, _)| *node == from) {
                Some(existing) => {
                    lowering.notes.push(format!(
                        "`{}` sends a signal already sent to `{name}`; the louder level applies",
                        send.source
                    ));
                    existing.1 = existing.1.max(level);
                }
                None => levels.push((from, level)),
            }
        }
        let mut feeds: Vec<NodeId> = Vec::new();
        for (from, level) in levels {
            let attenuated = graph.add_node(ProcessorSpec::StereoGain);
            set_param(&mut graph, attenuated, "gain", level, &mut lowering);
            graph.connect(from, 0, attenuated, 0);
            feeds.push(attenuated);
        }
        let Some(input) = merge(&mut graph, &feeds, &mut lowering) else {
            continue;
        };
        let tail = lower_container(&mut graph, input, None, bus, name, &mut addresses, &mut lowering);
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
        // Same reason as the sends: one signal reaching master twice is one
        // signal, not two.
        if let Some(from) = from
            && !master_inputs.contains(&from)
        {
            master_inputs.push(from);
        }
    }
    if master_inputs.is_empty() {
        lowering
            .notes
            .push("nothing is routed to `master`, so the studio renders silence".to_owned());
    }

    // Master always ends in the limiter (§13.6): a mix is a set of choices
    // about balance, and what leaves the graph should be bounded whatever
    // those choices were. Keep the graph renderable when nothing was routed —
    // a silent master is still a master.
    let mixed = merge(&mut graph, &master_inputs, &mut lowering)
        .unwrap_or_else(|| graph.add_node(ProcessorSpec::Passthrough { channels: 2 }));
    let master = graph.add_node(ProcessorSpec::Limiter);
    graph.connect(mixed, 0, master, 0);
    graph.set_output(master);

    lower_modulations(&mut graph, studio, &addresses, &mut lowering);
    (graph, lowering)
}

/// Where each addressable patch stage ended up: `(patch, stage, node)`.
///
/// A `modulate` path names a patch and a stage, and this is what turns that
/// pair into the node the control signal actually reaches.
type Addresses<'a> = Vec<(&'a str, NodeIndex, NodeId)>;

/// Set a node's parameter, recording rather than swallowing a refusal: a
/// rejected value means the graph is not what the studio asked for, and
/// silence about that is how a mix goes wrong invisibly.
fn set_param(graph: &mut StudioGraphSpec, node: NodeId, name: &'static str, value: f64, lowering: &mut StudioLowering) {
    if graph.set_param(node, name, value as f32).is_err() {
        lowering.notes.push(format!("`{name}` could not be set to {value}"));
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

/// The stages from a container's source to its `output`, in signal order.
///
/// Walking backwards along first inputs is what distinguishes the chain from
/// the things feeding it: `mix(carrier, shimmer)` has two inputs, and only
/// the path the chain was written along is the chain.
fn spine(patch: &Patch) -> Vec<NodeIndex> {
    let mut stages = Vec::new();
    let mut current = patch.output();
    while let Some(index) = current {
        if stages.contains(&index) {
            break;
        }
        stages.push(index);
        current = patch.nodes().get(index).and_then(|node| node.inputs.first()).copied();
    }
    stages.reverse();
    stages
}

/// Chain a container's stages downstream of `input`, returning the last node.
///
/// `synth` is the voice source an `envelope` stage configures, and is absent
/// for a bus: a bus has no voices to shape.
fn lower_container<'a>(
    graph: &mut StudioGraphSpec,
    input: NodeId,
    synth: Option<NodeId>,
    patch: &Patch,
    name: &'a str,
    addresses: &mut Addresses<'a>,
    lowering: &mut StudioLowering,
) -> NodeId {
    let mut previous = input;
    for index in spine(patch) {
        let Some(node) = patch.nodes().get(index) else { continue };
        let lowered = match node.processor {
            // The source section is what the polyphonic synth already is.
            Processor::Oscillator | Processor::Mix => continue,
            // An amplitude envelope is per-voice, so it is a setting of the
            // voice source rather than a stage after it.
            Processor::Envelope => {
                match synth {
                    Some(synth) => {
                        set_stage_params(graph, synth, node, lowering);
                        if let Some(release) = release_of(node) {
                            lowering.release_tail = lowering.release_tail.max(release);
                        }
                        addresses.push((name, index, synth));
                    }
                    None => lowering
                        .notes
                        .push(format!("`{name}` has no voices, so its `envelope` does nothing")),
                }
                continue;
            }
            Processor::Scale | Processor::Bias | Processor::Clamp | Processor::Smoothing => {
                lowering.notes.push(format!(
                    "`{}` shapes a control signal, not a sound; it does nothing in `{name}`",
                    node.processor.name()
                ));
                continue;
            }
            Processor::Gain => {
                let id = graph.add_node(ProcessorSpec::StereoGain);
                set_stage_params(graph, id, node, lowering);
                id
            }
            Processor::Lowpass | Processor::Highpass => {
                let kind = if node.processor == Processor::Lowpass {
                    FilterKind::LowPass
                } else {
                    FilterKind::HighPass
                };
                let id = graph.add_node(ProcessorSpec::Biquad { kind });
                set_stage_params(graph, id, node, lowering);
                id
            }
            Processor::Reverb => {
                let id = graph.add_node(ProcessorSpec::Reverb);
                set_stage_params(graph, id, node, lowering);
                id
            }
            Processor::Delay | Processor::Chorus => {
                let processor = if node.processor == Processor::Delay {
                    ProcessorSpec::Delay
                } else {
                    ProcessorSpec::Chorus
                };
                let id = graph.add_node(processor);
                set_stage_params(graph, id, node, lowering);
                id
            }
        };
        graph.connect(previous, 0, lowered, 0);
        addresses.push((name, index, lowered));
        previous = lowered;
    }
    previous
}

/// Configure the synth's oscillator bank from the patch's source section.
///
/// The bank is the note plus one partial at a ratio of it (§13.5). A patch
/// that mixes two oscillators is describing exactly that, so the second
/// oscillator's `ratio` and the gain in front of it become the bank's
/// settings; a patch that mixes more says so and gets the first extra
/// partial, because two is what a voice has.
fn lower_bank(graph: &mut StudioGraphSpec, synth: NodeId, patch: &Patch, lowering: &mut StudioLowering) {
    let Some(mix) = patch.nodes().iter().find(|node| node.processor == Processor::Mix) else {
        return;
    };
    let mut partials = mix.inputs.iter().skip(1).filter_map(|index| partial(patch, *index));
    let Some((ratio, level)) = partials.next() else { return };
    if partials.next().is_some() {
        lowering
            .notes
            .push("a voice has two partials; the rest of the mix is not sounded".to_owned());
    }
    set_param(graph, synth, "ratio", ratio, lowering);
    set_param(graph, synth, "blend", level, lowering);
}

/// One mixed partial as `(ratio, level)`: an oscillator, or an oscillator
/// with a gain in front of it.
fn partial(patch: &Patch, index: NodeIndex) -> Option<(f64, f64)> {
    let node = patch.nodes().get(index)?;
    match node.processor {
        Processor::Oscillator => Some((ratio_of(node), 1.0)),
        Processor::Gain => {
            let source = patch.nodes().get(*node.inputs.first()?)?;
            let level = node.params.first()?.as_linear();
            (source.processor == Processor::Oscillator).then(|| (ratio_of(source), level))
        }
        Processor::Mix
        | Processor::Envelope
        | Processor::Lowpass
        | Processor::Highpass
        | Processor::Reverb
        | Processor::Delay
        | Processor::Chorus
        | Processor::Scale
        | Processor::Bias
        | Processor::Clamp
        | Processor::Smoothing => None,
    }
}

/// An oscillator stage's frequency ratio to the note.
fn ratio_of(node: &StudioNode) -> f64 {
    node.processor
        .params()
        .iter()
        .position(|param| param.name == "ratio")
        .and_then(|index| node.params.get(index))
        .map_or(1.0, |value| value.as_linear())
}

/// The release time an `envelope` stage writes, in seconds.
fn release_of(node: &StudioNode) -> Option<f32> {
    let index = node
        .processor
        .params()
        .iter()
        .position(|param| param.name == "release")?;
    Some(node.params.get(index)?.as_linear() as f32)
}

/// Copy a stage's written values onto its node, by the names both sides
/// declare.
///
/// A written name the DSP does not declare is skipped rather than reported:
/// the language's `oscillator` carries a `ratio` that means nothing to an
/// LFO, and that is a difference between two vocabularies, not a mistake in
/// the patch. `gain` is written in dB and applied linearly; every other
/// parameter is the number that was written.
fn set_stage_params(graph: &mut StudioGraphSpec, id: NodeId, node: &StudioNode, lowering: &mut StudioLowering) {
    for (declared, value) in node.processor.params().iter().zip(&node.params) {
        let known = graph
            .processor_of(id)
            .is_some_and(|processor| processor.descriptor(declared.dsp_name).is_some());
        if known {
            set_param(graph, id, declared.dsp_name, value.as_linear(), lowering);
        }
    }
}

/// Wire every `modulate` into the parameter it names.
fn lower_modulations(
    graph: &mut StudioGraphSpec,
    studio: &StudioSpec,
    addresses: &Addresses<'_>,
    lowering: &mut StudioLowering,
) {
    let mut sources: Vec<(&str, NodeId)> = Vec::new();
    for modulation in studio.modulations() {
        let Some((_, _, target)) = addresses
            .iter()
            .find(|(patch, index, _)| *patch == modulation.patch && *index == modulation.node)
        else {
            lowering.notes.push(format!(
                "`{}.{}` is part of the voice source and cannot be modulated yet",
                modulation.patch, modulation.param
            ));
            continue;
        };
        let source = match sources.iter().find(|(name, _)| *name == modulation.source) {
            Some((_, id)) => Some(*id),
            None => {
                let built = studio
                    .signals()
                    .find(|(name, _)| *name == modulation.source)
                    .and_then(|(_, signal)| lower_signal(graph, signal, lowering));
                if let Some(id) = built {
                    sources.push((modulation.source.as_str(), id));
                }
                built
            }
        };
        if let Some(source) = source {
            let Some(dsp_name) = studio
                .patch(&modulation.patch)
                .and_then(|patch| patch.nodes().get(modulation.node))
                .and_then(|node| node.processor.param(modulation.param))
                .map(|param| param.dsp_name)
            else {
                lowering.notes.push(format!(
                    "`{}.{}` has no render parameter",
                    modulation.patch, modulation.param
                ));
                continue;
            };
            graph.modulate(source, 0, *target, dsp_name);
        }
    }
}

/// Build a control chain: an LFO and the stages that shape its output.
fn lower_signal(graph: &mut StudioGraphSpec, signal: &Patch, lowering: &mut StudioLowering) -> Option<NodeId> {
    let mut previous: Option<NodeId> = None;
    for index in spine(signal) {
        let Some(node) = signal.nodes().get(index) else {
            continue;
        };
        let processor = match node.processor {
            // A control-rate oscillator is what an oscillator means once it
            // is a modulation source rather than a voice.
            Processor::Oscillator => ProcessorSpec::Lfo {
                waveform: Waveform::Sine,
            },
            Processor::Scale => ProcessorSpec::Scale,
            Processor::Bias => ProcessorSpec::Bias,
            Processor::Clamp => ProcessorSpec::Clamp,
            Processor::Smoothing => ProcessorSpec::Smooth,
            Processor::Gain
            | Processor::Mix
            | Processor::Envelope
            | Processor::Lowpass
            | Processor::Highpass
            | Processor::Reverb
            | Processor::Delay
            | Processor::Chorus => {
                lowering.notes.push(format!(
                    "`{}` shapes a sound, not a control signal; it does nothing in a modulation source",
                    node.processor.name()
                ));
                continue;
            }
        };
        let id = graph.add_node(processor);
        set_stage_params(graph, id, node, lowering);
        if let Some(from) = previous {
            graph.connect(from, 0, id, 0);
        }
        previous = Some(id);
    }
    previous
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
