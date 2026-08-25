//! Studio spec → graph lowering.
//!
//! The contract is that the graph has the shape the patch describes. Not the
//! sound yet — most stages are pass-through — but the topology, because that
//! is what the real processors fill in rather than replace. The lowering must
//! also leave an unpatched piece exactly where it was: the default render is
//! golden, and a studio nobody wrote must not touch it.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, StudioSpec, compile};
use musa_dsp::testing::{GraphOptions, lower_studio, poly_sine_spec, prepare_graph};

const OPTIONS: GraphOptions = GraphOptions {
    sample_rate: 48_000,
    render_seed: 0,
};

const GLASS_MOUNTAIN: &str = include_str!("../../../../examples/glass-mountain.musa");

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

#[test]
fn an_empty_studio_is_the_default_instrument_graph() {
    // §14.8's zero-setup guarantee, stated as an identity rather than a
    // resemblance: this is what keeps the default render's audio golden.
    let (graph, lowering) = lower_studio(&StudioSpec::default(), &OPTIONS);
    assert_eq!(format!("{graph:#?}"), format!("{:#?}", poly_sine_spec(16)));
    assert!(lowering.notes.is_empty());
}

#[test]
fn a_patch_stage_becomes_a_node_between_the_synth_and_master() {
    let studio = studio_of(&piece(
        "patch p { oscillator(sine) |> gain(-6 dB) |> lowpass(cutoff: 800 Hz) |> output; } \
         assign violin -> p; route violin -> master;",
    ));
    let (graph, lowering) = lower_studio(&studio, &OPTIONS);
    // The synth stands in for the oscillator; the gain and the filter are
    // their own nodes, chained in the order the patch wrote them.
    let rendered = format!("{graph:#?}");
    assert!(rendered.contains("PolySine"), "{rendered}");
    assert!(rendered.contains("StereoGain"), "{rendered}");
    assert!(rendered.contains("Biquad"), "the filter is a filter now: {rendered}");
    assert!(rendered.contains("800.0"), "the written cutoff reaches it: {rendered}");
    assert!(lowering.notes.is_empty(), "{:?}", lowering.notes);
    prepare_graph(&graph, OPTIONS.sample_rate).expect("the lowered graph is valid");
}

#[test]
fn a_send_reaches_its_bus_and_the_bus_reaches_master() {
    let studio = studio_of(&piece(
        "patch p { oscillator(sine) |> output; } bus hall { reverb(room: 0.5); } \
         assign violin -> p; route violin -> master; send violin -> hall at -18 dB; route hall -> master;",
    ));
    let (graph, _) = lower_studio(&studio, &OPTIONS);
    prepare_graph(&graph, OPTIONS.sample_rate).expect("the send path is a valid graph");
    // Patch and bus both arrive at master, so they must meet in a mixer.
    assert!(format!("{graph:#?}").contains("Mixer"));
}

#[test]
fn a_studio_that_routes_nothing_says_so_and_still_renders() {
    let studio = studio_of(&piece("patch p { oscillator(sine) |> output; } assign violin -> p;"));
    let (graph, lowering) = lower_studio(&studio, &OPTIONS);
    assert!(
        lowering
            .notes
            .iter()
            .any(|note| note.contains("nothing is routed to `master`")),
        "{:?}",
        lowering.notes
    );
    prepare_graph(&graph, OPTIONS.sample_rate).expect("a silent graph is still a valid graph");
}

#[test]
fn the_roadmap_example_lowers_to_a_compilable_graph() {
    let (graph, lowering) = lower_studio(&studio_of(GLASS_MOUNTAIN), &OPTIONS);
    prepare_graph(&graph, OPTIONS.sample_rate).expect("glass-mountain's studio must render");
    // Both parts go to the one patch, so nothing is double-sounded and the
    // shared-note-stream note must not appear.
    assert!(
        !lowering.notes.iter().any(|note| note.contains("share one note stream")),
        "{:?}",
        lowering.notes
    );
    insta::assert_snapshot!("glass_mountain_graph", format!("{graph:#?}"));
}

#[test]
fn lowering_is_deterministic() {
    let studio = studio_of(GLASS_MOUNTAIN);
    let first = lower_studio(&studio, &OPTIONS);
    let second = lower_studio(&studio, &OPTIONS);
    assert_eq!(format!("{:#?}", first.0), format!("{:#?}", second.0));
    assert_eq!(first.1, second.1);
}
