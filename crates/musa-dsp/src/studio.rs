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
//! A patch declaration is a recipe, not a stateful instrument. Lowering makes
//! one synth and one downstream chain for every part that selects the recipe;
//! the preparation boundary then binds that synth to exactly that part's
//! scheduled event lane.

use crate::intent::{
    NodeIndex, ParamSpec, Patch, Processor, StudioNode, StudioSpec, Unit, WrittenQuantity, written_ratio,
};
use crate::spec::{FilterKind, GraphOptions, NodeId, ProcessorSpec, StudioGraphSpec, Waveform};

/// What a lowering produced besides the graph.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StudioLowering {
    /// Things the studio asked for that the graph could not honour exactly.
    /// Not errors: the graph renders, and the caller decides whether to show
    /// them (the CLI prints them; the desktop app surfaces them in the Sound
    /// workspace).
    pub(crate) notes: Vec<String>,
    /// Invalid exact intent refused at the DSP boundary. Compiler-produced
    /// specs have already passed the same public ranges; this catches
    /// programmatic construction and any future producer drift.
    pub(crate) errors: Vec<String>,
    /// The longest release any patch asks for, in seconds.
    ///
    /// An offline render needs it to know when the piece is over. A fixed
    /// tail was enough while every note stopped when it ended; a patch with
    /// a 3.5 s release would be cut off mid-fade by one.
    pub(crate) release_tail: f32,
    /// Where a caller may read the signal this studio already routes: one
    /// entry per part output and one per named bus, in that order.
    ///
    /// These are not a second mixer. Each names a node the graph writes
    /// anyway, so reading one cannot change what the master hears.
    pub(crate) taps: Vec<TapPoint>,
}

/// Which kind of declared route a tap reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TapRole {
    /// The output of one part's own instrument chain, before any bus.
    Part,
    /// The output of one named bus, after its own chain.
    Bus,
}

/// One readable point on the routing the source declared.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TapPoint {
    /// Whether this is a part output or a bus output.
    pub(crate) role: TapRole,
    /// The name the source wrote, kept for the caller to name the tap by.
    pub(crate) name: String,
    /// The node whose output buffer carries it.
    pub(crate) node: NodeId,
}

/// One part-local event input created while lowering the compatibility studio.
///
/// This is a prepared binding, not source routing vocabulary: the source part
/// and declaration names are retained only long enough for audio preparation
/// to resolve them to private compact indices.
pub(crate) struct PartInput {
    pub(crate) part: String,
    pub(crate) declaration: String,
    pub(crate) node: NodeId,
}

/// One named recorded-media input bound to a private graph source slot.
pub(crate) struct MediaInput {
    pub(crate) name: String,
    pub(crate) node: NodeId,
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
pub(crate) fn lower_studio(studio: &StudioSpec, options: &GraphOptions) -> (StudioGraphSpec, StudioLowering) {
    let parts = studio.assignments().map(|(part, _)| part).collect::<Vec<_>>();
    let lowered = lower_studio_for_parts(studio, &parts, options);
    (lowered.0, lowered.1)
}

/// Lower one independently stateful instrument for every prepared part.
pub(crate) fn lower_studio_for_parts(
    studio: &StudioSpec,
    parts: &[&str],
    options: &GraphOptions,
) -> (StudioGraphSpec, StudioLowering, Vec<PartInput>) {
    let (graph, lowering, parts, _) = lower_studio_for_sources(studio, parts, &[], options);
    (graph, lowering, parts)
}

/// Lower part-local instruments and named recorded-media machines through the
/// same source-authored bus/send/master graph.
pub(crate) fn lower_studio_for_sources(
    studio: &StudioSpec,
    parts: &[&str],
    media: &[&str],
    _options: &GraphOptions,
) -> (StudioGraphSpec, StudioLowering, Vec<PartInput>, Vec<MediaInput>) {
    let mut lowering = StudioLowering::default();
    validate_exact_intent(studio, &mut lowering);
    if !lowering.errors.is_empty() {
        return (
            crate::instrument::poly_sine_spec(POLYPHONY),
            lowering,
            Vec::new(),
            Vec::new(),
        );
    }
    if studio.is_empty() && parts.len() <= 1 && media.is_empty() {
        let graph = crate::instrument::poly_sine_spec(POLYPHONY);
        let inputs = parts
            .first()
            .map(|part| PartInput {
                part: (*part).to_owned(),
                declaration: "std.sound.basic_sine@1".to_owned(),
                node: NodeId(0),
            })
            .into_iter()
            .collect();
        // The default instrument graph *is* the part's output, so the one
        // part a piece without a studio can have taps the graph output.
        lowering.taps = parts
            .first()
            .and_then(|part| {
                graph.output().map(|node| TapPoint {
                    role: TapRole::Part,
                    name: (*part).to_owned(),
                    node,
                })
            })
            .into_iter()
            .collect();
        return (graph, lowering, inputs, Vec::new());
    }

    let mut graph = StudioGraphSpec::new();

    // Every part gets its own synth and chain. Reusing a declaration reuses
    // only its immutable recipe, never voices, envelopes, or effect state.
    let mut master_inputs: Vec<NodeId> = Vec::new();
    let mut part_outputs: Vec<(&str, NodeId)> = Vec::new();
    let mut inputs = Vec::with_capacity(parts.len());
    let mut addresses: Addresses<'_> = Vec::new();
    for part in parts {
        let declaration = studio.patch_for_part(part);
        let synth = graph.add_node(ProcessorSpec::PolySine { voices: POLYPHONY });
        let tail = match declaration.and_then(|name| studio.patch(name).map(|patch| (name, patch))) {
            Some((name, patch)) => {
                lower_bank(&mut graph, synth, patch, &mut lowering);
                lower_container(
                    &mut graph,
                    synth,
                    Some(synth),
                    patch,
                    name,
                    &mut addresses,
                    &mut lowering,
                )
            }
            None => synth,
        };
        part_outputs.push((part, tail));
        inputs.push(PartInput {
            part: (*part).to_owned(),
            declaration: declaration.unwrap_or("std.sound.basic_sine@1").to_owned(),
            node: synth,
        });
    }
    let mut media_inputs = Vec::with_capacity(media.len());
    for name in media {
        let node = graph.add_node(ProcessorSpec::Passthrough { channels: 2 });
        part_outputs.push((name, node));
        media_inputs.push(MediaInput {
            name: (*name).to_owned(),
            node,
        });
    }

    // Buses are shared mix machines. A source may be a part instance or an
    // already-prepared bus; compiler cycle checking makes this finite order
    // exist before DSP preparation begins.
    let mut bus_outputs: Vec<(&str, NodeId)> = Vec::new();
    let mut pending = studio.buses().collect::<Vec<_>>();
    while !pending.is_empty() {
        let Some(index) = pending.iter().position(|(name, _)| {
            studio
                .sends()
                .iter()
                .filter(|send| send.bus == *name && studio.has_bus(&send.source))
                .all(|send| bus_outputs.iter().any(|(bus, _)| *bus == send.source))
                && studio
                    .routes()
                    .iter()
                    .filter(|route| route.destination == *name && studio.has_bus(&route.source))
                    .all(|route| bus_outputs.iter().any(|(bus, _)| *bus == route.source))
        }) else {
            lowering
                .errors
                .push("cyclic bus bindings reached DSP preparation".to_owned());
            break;
        };
        let (name, bus) = pending.remove(index);
        let mut feeds: Vec<NodeId> = Vec::new();
        for send in studio.sends().iter().filter(|send| send.bus == name) {
            let from = source_output(&send.source, &part_outputs).or_else(|| {
                bus_outputs
                    .iter()
                    .find(|(bus, _)| *bus == send.source)
                    .map(|(_, node)| *node)
            });
            let Some(from) = from else { continue };
            let attenuated = graph.add_node(ProcessorSpec::StereoGain);
            set_param(
                &mut graph,
                attenuated,
                "gain",
                crate::quantity::linear(send.level),
                &mut lowering,
            );
            graph.connect(from, 0, attenuated, 0);
            feeds.push(attenuated);
        }
        for route in studio.routes().iter().filter(|route| route.destination == name) {
            let from = source_output(&route.source, &part_outputs).or_else(|| {
                bus_outputs
                    .iter()
                    .find(|(bus, _)| *bus == route.source)
                    .map(|(_, node)| *node)
            });
            if let Some(from) = from {
                feeds.push(from);
            }
        }
        let input = merge(&mut graph, &feeds, &mut lowering)
            .unwrap_or_else(|| graph.add_node(ProcessorSpec::Passthrough { channels: 2 }));
        let tail = lower_container(&mut graph, input, None, bus, name, &mut addresses, &mut lowering);
        bus_outputs.push((name, tail));
    }

    // Routes decide what reaches master. A studio that routes nothing is
    // silent, and that is the studio's statement, not a bug to paper over.
    for route in studio.routes() {
        if route.destination != "master" {
            continue;
        }
        let from = source_output(&route.source, &part_outputs).or_else(|| {
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
    // A part absent from the compatibility studio has the edition default and
    // its implicit route to master. This is independent of whether another
    // part made its own studio choices explicit.
    for (part, output) in &part_outputs {
        let is_media = media.iter().any(|name| name == part);
        if ((is_media && studio.is_empty()) || (!is_media && studio.patch_for_part(part).is_none()))
            && !master_inputs.contains(output)
        {
            master_inputs.push(*output);
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
    // Parts first, then buses, each in the order lowering met them. Recorded
    // media is deliberately absent: it is an input the studio routes, not a
    // part the score has, and the master already carries it.
    lowering.taps = part_outputs
        .iter()
        .take(parts.len())
        .map(|(name, node)| TapPoint {
            role: TapRole::Part,
            name: (*name).to_owned(),
            node: *node,
        })
        .chain(bus_outputs.iter().map(|(name, node)| TapPoint {
            role: TapRole::Bus,
            name: (*name).to_owned(),
            node: *node,
        }))
        .collect();
    (graph, lowering, inputs, media_inputs)
}

/// Defensively recheck the source-derived exact contract at the lossy DSP edge.
/// The preparation projection is private, but checking here keeps malformed
/// internal data from reaching float conversion or primitive allocation.
fn validate_exact_intent(studio: &StudioSpec, lowering: &mut StudioLowering) {
    for (container, patch) in studio.patches().chain(studio.buses()).chain(studio.signals()) {
        for node in patch.nodes() {
            for (index, value) in node.params.iter().enumerate() {
                let Some(value) = value else { continue };
                let Some(declared) = node.processor.params().get(index) else {
                    lowering.errors.push(format!(
                        "`{container}` has an undeclared parameter on `{}`",
                        node.processor.name()
                    ));
                    continue;
                };
                if value.unit != declared.unit {
                    lowering.errors.push(format!(
                        "`{container}.{}.{}` has unit {:?}, expected {:?}",
                        node.processor.name(),
                        declared.name,
                        value.unit,
                        declared.unit
                    ));
                } else if value.magnitude < declared.range.0 || value.magnitude > declared.range.1 {
                    lowering.errors.push(format!(
                        "`{container}.{}.{}` exact value {} is outside written range {}–{}",
                        node.processor.name(),
                        declared.name,
                        format_quantity(*value),
                        written_ratio(declared.range.0),
                        written_ratio(declared.range.1)
                    ));
                }
            }
        }
    }

    for send in studio.sends() {
        if send.level.unit != Unit::Decibels {
            lowering.errors.push(format!(
                "send `{} -> {}` has unit {:?}, expected {:?}",
                send.source,
                send.bus,
                send.level.unit,
                Unit::Decibels
            ));
        } else if !crate::quantity::linear(send.level).is_finite() {
            lowering.errors.push(format!(
                "send `{} -> {}` cannot be represented finitely at the DSP boundary",
                send.source, send.bus
            ));
        }
    }
}

fn format_quantity(value: WrittenQuantity) -> String {
    match value.unit.spelling() {
        Some(unit) => format!("{} {unit}", written_ratio(value.magnitude)),
        None => written_ratio(value.magnitude),
    }
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
    if graph.set_param(node, name, crate::quantity::f64_to_f32(value)).is_err() {
        lowering.errors.push(format!("`{name}` could not be set to {value}"));
    }
}

/// Where a part-valued `route`/`send` source's signal comes from.
fn source_output(name: &str, parts: &[(&str, NodeId)]) -> Option<NodeId> {
    parts
        .iter()
        .find(|(candidate, _)| *candidate == name)
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
            let declared = node.processor.params().first()?;
            let level = crate::quantity::linear(value_or_default(node, 0, declared));
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
        .and_then(|index| node.processor.params().get(index).map(|declared| (index, declared)))
        .map_or(1.0, |(index, declared)| {
            crate::quantity::linear(value_or_default(node, index, declared))
        })
}

/// The release time an `envelope` stage writes, in seconds.
fn release_of(node: &StudioNode) -> Option<f32> {
    let index = node
        .processor
        .params()
        .iter()
        .position(|param| param.name == "release")?;
    let declared = node.processor.params().get(index)?;
    Some(crate::quantity::linear_f32(value_or_default(node, index, declared)))
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
    for (index, declared) in node.processor.params().iter().enumerate() {
        let known = graph
            .processor_of(id)
            .is_some_and(|processor| processor.descriptor(declared.dsp_name).is_some());
        if known {
            let value = value_or_default(node, index, declared);
            set_param(graph, id, declared.dsp_name, crate::quantity::linear(value), lowering);
        }
    }
}

fn value_or_default(node: &StudioNode, index: usize, declared: &ParamSpec) -> WrittenQuantity {
    node.params
        .get(index)
        .copied()
        .flatten()
        .unwrap_or_else(|| WrittenQuantity::new(declared.default, declared.unit))
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
        let targets = addresses
            .iter()
            .filter(|(patch, index, _)| *patch == modulation.patch && *index == modulation.node)
            .map(|(_, _, target)| *target)
            .collect::<Vec<_>>();
        if targets.is_empty() {
            lowering.notes.push(format!(
                "`{}.{}` is part of the voice source and cannot be modulated yet",
                modulation.patch, modulation.param
            ));
            continue;
        }
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
            for target in targets {
                graph.modulate(source, 0, target, dsp_name);
            }
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
