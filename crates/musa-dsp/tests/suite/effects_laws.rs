//! Time effects, feedback, and the mix (§13.3, §13.6, §17.5).
//!
//! The processors' own arithmetic is tested beside them, in `effects.rs`,
//! where a delay line is a delay line. What only this level can state is what
//! happens when they are nodes in a graph: that a cycle is legal exactly when
//! it passes through a delay, that a send arrives at the level it was written
//! at, that master is bounded whatever the mix did, and that none of it stops
//! being a number.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// Sample arithmetic in tests is small and total.
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{CompileOptions, SourceDocument, StudioSpec, compile};
use musa_dsp::testing::{
    GraphError, GraphOptions, NodeId, ProcessorSpec, StudioGraphSpec, lower_studio, prepare_graph,
};

use super::support::render_source;

const RATE: u32 = 48_000;
const OPTIONS: GraphOptions = GraphOptions {
    sample_rate: RATE,
    render_seed: 0,
};

/// Render a graph with no events, as interleaved stereo.
fn render(spec: &StudioGraphSpec, frames: usize) -> Vec<f32> {
    let mut plan = prepare_graph(spec, RATE).expect("the graph is valid");
    let mut output = vec![0.0f32; frames * 2];
    plan.render(&[], &mut output);
    output
}

/// The left channel of an interleaved buffer.
fn left(output: &[f32]) -> Vec<f32> {
    output.iter().step_by(2).copied().collect()
}

/// The loudest sample anywhere in an interleaved buffer.
fn peak(output: &[f32]) -> f32 {
    output.iter().fold(0.0f32, |worst, sample| worst.max(sample.abs()))
}

fn studio_of(source: &str) -> StudioSpec {
    let compilation = compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default());
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    compilation.into_parts().1
}

fn piece(studio: &str) -> String {
    format!(
        "piece \"x\" {{ tempo 1/4 = 60; meter 4/4; score {{ part violin {{ voice v {{ c4/1 }} }} }} \
         studio {{ {studio} }} }}"
    )
}

/// A one-sample impulse into a stereo signal, as a graph a delay can be
/// hung off: a constant through a gain that opens for exactly one frame is
/// not expressible, so the impulse is written into the buffer by rendering a
/// noise-free source and reading the delay's own response instead.
fn impulse_graph(effect: ProcessorSpec) -> (StudioGraphSpec, NodeId) {
    let mut graph = StudioGraphSpec::new();
    let source = graph.add_node(ProcessorSpec::Sine);
    let stereo = graph.add_node(ProcessorSpec::MonoToStereo);
    let node = graph.add_node(effect);
    graph.connect(source, 0, stereo, 0);
    graph.connect(stereo, 0, node, 0);
    graph.set_output(node);
    (graph, node)
}

/// §13.3: a cycle that passes through a delay is legal, because the delay is
/// what makes it causal — the signal that comes back was held for an amount
/// the patch wrote.
#[test]
fn a_feedback_loop_through_a_delay_compiles() {
    let mut graph = StudioGraphSpec::new();
    let source = graph.add_node(ProcessorSpec::Sine);
    let stereo = graph.add_node(ProcessorSpec::MonoToStereo);
    let mixer = graph.add_node(ProcessorSpec::Mixer { inputs: 2 });
    let delay = graph.add_node(ProcessorSpec::Delay);
    graph.connect(source, 0, stereo, 0);
    graph.connect(stereo, 0, mixer, 0);
    graph.connect(mixer, 0, delay, 0);
    // The loop: the delay's output comes back into the mixer.
    graph.connect(delay, 0, mixer, 1);
    graph.set_output(mixer);

    let output = render(&graph, RATE as usize);
    assert!(output.iter().all(|sample| sample.is_finite()), "a loop that diverged");
    assert!(peak(&output) > 0.0, "a loop that is silent is not a loop");
}

/// §13.3: and any other cycle is rejected. There is no reading of it that is
/// not "the output before the output".
#[test]
fn a_feedback_loop_without_a_delay_is_rejected() {
    let mut graph = StudioGraphSpec::new();
    let source = graph.add_node(ProcessorSpec::Sine);
    let stereo = graph.add_node(ProcessorSpec::MonoToStereo);
    let mixer = graph.add_node(ProcessorSpec::Mixer { inputs: 2 });
    let gain = graph.add_node(ProcessorSpec::StereoGain);
    graph.connect(source, 0, stereo, 0);
    graph.connect(stereo, 0, mixer, 0);
    graph.connect(mixer, 0, gain, 0);
    graph.connect(gain, 0, mixer, 1);
    graph.set_output(mixer);

    assert!(matches!(prepare_graph(&graph, RATE), Err(GraphError::Cycle)));
}

/// A delay in a graph is the delay that was written: the effect arrives after
/// the time it names, not after whatever the block size happened to be.
#[test]
fn a_delay_in_a_graph_arrives_when_it_said_it_would() {
    let (mut graph, delay) = impulse_graph(ProcessorSpec::Delay);
    // 20 Hz through a 75 ms delay: one and a half periods, so the wet copy
    // comes back inverted. Half a period either way and it would not — which
    // is what makes this a test of the delay's length rather than of its
    // existence.
    graph.set_param(delay, "time", 0.075).expect("delay declares a time");
    graph
        .set_param(delay, "feedback", 0.0)
        .expect("delay declares feedback");
    graph.set_param(delay, "mix", 1.0).expect("delay declares a mix");
    let mut dry = StudioGraphSpec::new();
    let source = dry.add_node(ProcessorSpec::Sine);
    let stereo = dry.add_node(ProcessorSpec::MonoToStereo);
    dry.connect(source, 0, stereo, 0);
    dry.set_output(stereo);
    for graph in [&mut graph, &mut dry] {
        graph.set_param(NodeId(0), "frequency", 20.0).expect("a sine");
    }

    let frames = RATE as usize * 4;
    let (wet, plain) = (left(&render(&graph, frames)), left(&render(&dry, frames)));
    let tail = frames - RATE as usize;
    let error = wet
        .iter()
        .zip(&plain)
        .skip(tail)
        .fold(0.0f32, |worst, (a, b)| worst.max((a + b).abs()));
    assert!(
        error < 0.02,
        "a settled 75 ms delay of a 20 Hz sine is inverted: {error}"
    );
}

/// A reverb is what makes a sound outlast itself: two seconds after the note
/// stopped, the room is still audible where the dry patch is silent.
#[test]
fn a_reverb_keeps_sounding_after_the_note_stops() {
    let tail_energy = |studio: &str| {
        let source = format!(
            "piece \"x\" {{ tempo 1/4 = 240; meter 4/4; \
             score {{ part violin {{ voice v {{ c4/4 rest/2 }} }} }} studio {{ {studio} }} }}"
        );
        let samples = render_source(&source, RATE as usize * 4);
        // The note has been over for a second by here (interleaved stereo,
        // so twice the frame count).
        let from = RATE as usize * 2 * 2;
        peak(samples.get(from..).unwrap_or(&[]))
    };
    let dry = tail_energy("patch p { oscillator(sine) |> output; } assign violin -> p; route violin -> master;");
    let wet = tail_energy(
        "patch p { oscillator(sine) |> reverb(room: 0.95, damping: 0.1, mix: 1.0) |> output; } \
         assign violin -> p; route violin -> master;",
    );
    assert!(dry < 1e-4, "a plain voice is silent once released: {dry}");
    assert!(wet > dry * 100.0, "a room is not: {wet} against {dry}");
}

/// §13.6: master is bounded. Whatever the mix asked for, nothing leaves the
/// graph above 0 dBFS.
#[test]
fn nothing_leaves_master_above_full_scale() {
    let mut graph = StudioGraphSpec::new();
    let source = graph.add_node(ProcessorSpec::Sine);
    let stereo = graph.add_node(ProcessorSpec::MonoToStereo);
    let loud = graph.add_node(ProcessorSpec::StereoGain);
    let master = graph.add_node(ProcessorSpec::Limiter);
    graph.connect(source, 0, stereo, 0);
    graph.connect(stereo, 0, loud, 0);
    graph.connect(loud, 0, master, 0);
    graph.set_output(master);
    graph.set_param(loud, "gain", 16.0).expect("a gain declares a gain");

    let output = render(&graph, RATE as usize);
    assert!(peak(&output) <= 1.0, "peak {}", peak(&output));
    assert!(peak(&output) > 0.5, "a limiter is not a mute");
}

/// A send arrives at the level it was written at: `-18 dB` is a conversion
/// the lowering does once, not a number the graph reinterprets.
#[test]
fn a_send_arrives_at_the_level_it_was_written_at() {
    let quiet = studio_of(&piece(
        "patch p { oscillator(sine) |> output; } bus hall { reverb(room: 0.5, mix: 1.0); } \
         assign violin -> p; send violin -> hall at -40 dB; route hall -> master;",
    ));
    let loud = studio_of(&piece(
        "patch p { oscillator(sine) |> output; } bus hall { reverb(room: 0.5, mix: 1.0); } \
         assign violin -> p; send violin -> hall at 0 dB; route hall -> master;",
    ));
    let level = |studio: &StudioSpec| {
        let (graph, _) = lower_studio(studio, &OPTIONS);
        let rendered = format!("{graph:#?}");
        assert!(rendered.contains("Reverb"), "the bus is a reverb now: {rendered}");
        rendered
    };
    // -40 dB is a hundredth; the gain on the send says so in the graph.
    assert!(level(&quiet).contains("0.01"), "the send's level reached the graph");
    assert!(level(&loud).contains("1.0"));
}

/// §17.5: nothing in the effect chain produces a value that is not a number,
/// under the worst settings a patch can ask for.
#[test]
fn no_reachable_effect_setting_produces_a_non_finite_sample() {
    let studio = studio_of(&piece(
        "patch p { oscillator(sine) \
             |> delay(time: 0 s, feedback: 0.95, mix: 1.0) \
             |> chorus(rate: 20 Hz, depth: 10 ms, mix: 1.0) \
             |> reverb(room: 1.0, damping: 0.0, mix: 1.0) \
             |> output; } \
         assign violin -> p; route violin -> master;",
    ));
    let (graph, _) = lower_studio(&studio, &OPTIONS);
    let output = render(&graph, RATE as usize * 4);
    assert!(
        output.iter().all(|sample| sample.is_finite()),
        "an effect chain produced a non-number"
    );
    assert!(peak(&output) <= 1.0, "and master still holds: {}", peak(&output));
}

/// The whole thing is deterministic: two renders of one piece are the same
/// samples, effects and all (§13.8).
#[test]
fn an_effected_render_is_deterministic() {
    let source = piece(
        "patch p { oscillator(sine) |> delay(time: 120 ms, feedback: 0.5) |> output; } \
         bus hall { reverb(room: 0.7); } \
         assign violin -> p; send violin -> hall at -12 dB; route violin -> master; route hall -> master;",
    );
    let once = render_source(&source, RATE as usize * 2);
    let twice = render_source(&source, RATE as usize * 2);
    assert_eq!(once, twice);
}
