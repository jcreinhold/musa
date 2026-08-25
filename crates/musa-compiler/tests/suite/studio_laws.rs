//! The studio language.
//!
//! Three contracts, in the order they matter:
//!
//! - **the bridge stays narrow** — a `studio` block adds patches and
//!   bindings and changes not one note. The score compiled from a piece with
//!   a studio equals the score compiled from the same piece without one;
//! - **units are syntax** — §7.2 says a unit-bearing parameter written bare
//!   is a diagnostic, not a guess. A `cutoff` of `1400` is rejected;
//! - **names resolve or they are reported** — every part, patch, bus,
//!   signal, and parameter a studio names is checked, with a span.

// Test helpers use expect()/panic! on statically-valid inputs: a failure is a
// bug in the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::unwrap_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_dsp::{Processor, StudioSpec, Unit};

use musa_score::{Code, Severity};

const GLASS_MOUNTAIN: &str = include_str!("../../../../examples/glass-mountain.musa");

fn compile_text(text: &str) -> musa_compiler::Compilation {
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
}

fn studio_of(text: &str) -> StudioSpec {
    let compilation = compile_text(text);
    let errors: Vec<&str> = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();
    assert!(errors.is_empty(), "expected a clean compile; got {errors:?}");
    compilation.into_parts().1
}

fn errors_of(text: &str) -> Vec<String> {
    compile_text(text)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

/// A one-part piece whose studio block is supplied.
fn piece(studio: &str) -> String {
    format!(
        "piece \"x\" {{ tempo 1/4 = 60; meter 4/4; score {{ part violin {{ voice v {{ c4/1 }} }} }} \
         studio {{ {studio} }} }}"
    )
}

// --- The narrow bridge ------------------------------------------------------

#[test]
fn a_studio_block_changes_nothing_about_the_score() {
    // §6.5: the studio does not inspect notes, and adding one must not move
    // a single event. This is the property that lets sound design and
    // composition proceed independently.
    let with = compile_text(&piece(
        "patch p { oscillator(sine) |> output; } assign violin -> p; route violin -> master;",
    ));
    let without = compile_text("piece \"x\" { tempo 1/4 = 60; meter 4/4; score { part violin { voice v { c4/1 } } } }");
    let (with_score, studio) = with.into_parts();
    let (without_score, empty) = without.into_parts();
    assert_eq!(
        format!("{:#?}", with_score.expect("compiles")),
        format!("{:#?}", without_score.expect("compiles"))
    );
    assert!(!studio.is_empty());
    assert!(empty.is_empty(), "no `studio` block means no studio");
}

// --- Structure --------------------------------------------------------------

#[test]
fn a_chain_becomes_nodes_wired_head_to_tail() {
    let studio = studio_of(&piece(
        "patch p { oscillator(sine) |> gain(-6 dB) |> lowpass(cutoff: 800 Hz) |> output; } \
         assign violin -> p; route violin -> master;",
    ));
    let patch = studio.patch("p").expect("declared");
    let kinds: Vec<Processor> = patch.nodes().iter().map(|node| node.processor).collect();
    assert_eq!(kinds, vec![Processor::Oscillator, Processor::Gain, Processor::Lowpass]);
    // Each stage takes the previous one, and `output` marks the last rather
    // than adding a node of its own.
    assert_eq!(
        patch.nodes().get(1).map(|node| node.inputs.as_slice()),
        Some([0].as_slice())
    );
    assert_eq!(
        patch.nodes().get(2).map(|node| node.inputs.as_slice()),
        Some([1].as_slice())
    );
    assert_eq!(patch.output(), Some(2));
}

#[test]
fn a_named_signal_is_reachable_from_a_later_chain() {
    let studio = studio_of(&piece(
        "patch p { carrier = oscillator(sine); shimmer = oscillator(sine, ratio: 2); \
         mix(carrier, shimmer) |> output; } assign violin -> p; route violin -> master;",
    ));
    let patch = studio.patch("p").expect("declared");
    let mixer = patch.nodes().last().expect("the mix");
    assert_eq!(mixer.processor, Processor::Mix);
    assert_eq!(mixer.inputs, vec![0, 1], "both named signals feed the mix");
    assert_eq!(
        patch.nodes().first().and_then(|node| node.label.clone()),
        Some("carrier".to_owned())
    );
}

#[test]
fn written_units_are_kept_rather_than_converted() {
    // A `StudioSpec` is editable intent: a UI must be able to show `-15 dB`
    // because that is what was written. Linear gain is the graph's business.
    let studio = studio_of(&piece(
        "patch p { oscillator(sine) |> gain(-6 dB) |> output; } assign violin -> p; route violin -> master;",
    ));
    let gain = studio.patch("p").expect("declared").nodes().get(1).expect("the gain");
    let level = gain.params.first().copied().flatten().expect("written parameter");
    assert_eq!(level.unit, Unit::Decibels);
    assert_eq!(level.magnitude, num_rational::Ratio::from_integer(-6));
}

#[test]
fn milliseconds_are_seconds_written_smaller() {
    let studio = studio_of(&piece(
        "patch p { oscillator(sine) |> envelope(adsr(attack: 30 ms)) |> output; } \
         assign violin -> p; route violin -> master;",
    ));
    let envelope = studio
        .patch("p")
        .expect("declared")
        .nodes()
        .get(1)
        .expect("the envelope");
    let attack = envelope.params.first().copied().flatten().expect("attack");
    assert_eq!(attack.unit, Unit::Seconds);
    assert_eq!(attack.magnitude, num_rational::Ratio::new(3, 100));
}

#[test]
fn decimal_ratio_and_scaled_time_have_exact_meanings() {
    let value = |written: &str| {
        let studio = studio_of(&piece(&format!(
            "patch p {{ oscillator(sine) |> lowpass(resonance: {written}) |> output; }} \
             assign violin -> p; route violin -> master;"
        )));
        studio
            .patch("p")
            .expect("patch")
            .nodes()
            .get(1)
            .and_then(|node| node.params.get(1))
            .copied()
            .flatten()
            .expect("written resonance")
            .magnitude
    };
    assert_eq!(
        value("0.1"),
        value("1/10"),
        "decimal syntax denotes its exact decimal rational"
    );

    let studio = studio_of(&piece(
        "patch p { oscillator(sine) |> envelope(adsr(attack: 100 ms, decay: 0.1 s)) |> output; } \
         assign violin -> p; route violin -> master;",
    ));
    let envelope = studio.patch("p").expect("patch").nodes().get(1).expect("envelope");
    assert_eq!(
        envelope.params.first().copied().flatten().expect("attack").magnitude,
        envelope.params.get(1).copied().flatten().expect("decay").magnitude,
        "millisecond normalization is exact"
    );
}

#[test]
fn a_modulation_resolves_to_the_node_it_names() {
    let studio = studio_of(&piece(
        "patch p { oscillator(sine) |> lowpass(cutoff: 800 Hz) |> output; } \
         lfo = oscillator(sine, frequency: 1 Hz) |> scale(50 Hz); \
         modulate lfo -> p.lowpass.cutoff; assign violin -> p; route violin -> master;",
    ));
    let modulation = studio.modulations().first().expect("one connection");
    assert_eq!(modulation.source, "lfo");
    assert_eq!(modulation.patch, "p");
    assert_eq!(modulation.node, 1);
    assert_eq!(modulation.param, "cutoff");
}

#[test]
fn the_roadmap_example_compiles_and_resolves() {
    let studio = studio_of(GLASS_MOUNTAIN);
    assert_eq!(studio.patch_for_part("violin"), Some("glass_pad"));
    assert_eq!(studio.patch_for_part("strings"), Some("glass_pad"));
    assert_eq!(studio.sends().len(), 2);
    assert_eq!(studio.routes().len(), 3);
    assert_eq!(studio.modulations().len(), 1);
    insta::assert_snapshot!("glass_mountain_studio", format!("{studio:#?}"));
}

/// Every processor the language names has DSP behind it, so a patch that
/// uses one is not merely accepted — it is accepted without a word of
/// apology.
#[test]
fn a_written_effect_compiles_without_a_warning() {
    let compilation = compile_text(&piece(
        "patch p { oscillator(sine) |> reverb(room: 0.5) |> delay(time: 250 ms) |> chorus() |> output; } \
         assign violin -> p; route violin -> master;",
    ));
    assert!(!compilation.has_errors(), "the effects are part of the language");
    let warnings: Vec<&str> = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();
    assert_eq!(warnings, Vec::<&str>::new());
}

#[test]
fn assigning_one_part_twice_is_a_spanned_duplicate_binding() {
    let compilation = compile_text(&piece(
        "patch p { oscillator(sine) |> output; } patch q { oscillator(sine) |> output; } \
         assign violin -> p; assign violin -> q;",
    ));
    let diagnostic = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == Code::DuplicateName)
        .expect("duplicate assignment diagnostic");
    assert!(diagnostic.message.contains("already has an instrument"));
    assert!(!diagnostic.labels.is_empty(), "the second binding is pointed out");
}

#[test]
fn cyclic_bus_bindings_are_refused_at_their_statements() {
    let compilation = compile_text(&piece(
        "patch p { oscillator(sine) |> output; } bus one { reverb(); } bus two { reverb(); } \
         assign violin -> p; route violin -> master; \
         send one -> two at -6 dB; send two -> one at -6 dB;",
    ));
    let cycles = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::DependencyCycle)
        .collect::<Vec<_>>();
    assert_eq!(cycles.len(), 2, "each written cycle edge is identified");
    assert!(cycles.iter().all(|diagnostic| !diagnostic.labels.is_empty()));
}

// --- Diagnostics ------------------------------------------------------------

#[test]
fn a_unit_bearing_parameter_written_bare_is_rejected() {
    // §7.2: units are part of the syntax. `1400` is not a frequency.
    let errors = errors_of(&piece(
        "patch p { oscillator(sine) |> lowpass(cutoff: 1400) |> output; } assign violin -> p;",
    ));
    assert!(
        errors.contains(&"`cutoff` is written in `Hz`".to_owned()),
        "got {errors:?}"
    );
}

#[test]
fn a_ratio_written_with_a_unit_is_rejected_too() {
    let errors = errors_of(&piece(
        "patch p { oscillator(sine) |> lowpass(cutoff: 1400 Hz, resonance: 2 Hz) |> output; } assign violin -> p;",
    ));
    assert!(
        errors.contains(&"`resonance` takes no unit".to_owned()),
        "got {errors:?}"
    );
}

#[test]
fn a_parameter_outside_the_catalogues_written_range_is_rejected() {
    let compilation = compile_text(&piece(
        "patch p { oscillator(sine) |> lowpass(resonance: 25) |> output; } assign violin -> p;",
    ));
    let diagnostic = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message.contains("`resonance` must be between"))
        .expect("the public range is checked");
    assert_eq!(diagnostic.code, Code::OutOfRange);
}

#[test]
fn an_exact_ratio_just_beyond_a_range_is_rejected() {
    let compilation = compile_text(&piece(
        "patch p { oscillator(sine) |> lowpass(cutoff: 200001/10 Hz) |> output; } assign violin -> p;",
    ));
    assert!(
        compilation
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == Code::OutOfRange),
        "range checking must not round 20000.1 onto the boundary"
    );
}

#[test]
fn removed_q_is_a_hard_error_with_an_exact_fix() {
    let compilation = compile_text(&piece(
        "patch p { oscillator(sine) |> lowpass(cutoff: 1400 Hz, q: 0.7) |> output; } assign violin -> p;",
    ));
    let diagnostic = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message.contains("`q` is no longer"))
        .expect("the removed spelling is diagnosed");
    assert_eq!(diagnostic.code, Code::UnknownName);
    let fix = diagnostic.fixes.first().expect("one certain repair");
    assert_eq!(fix.edits.len(), 1);
    assert_eq!(fix.edits.first().expect("one edit").replacement, "resonance");
}

#[test]
fn every_unresolved_name_is_reported() {
    let cases = [
        (
            "patch p { oscillator(sine) |> output; } assign viola -> p;",
            "unknown part `viola`",
        ),
        (
            "patch p { oscillator(sine) |> output; } assign violin -> q;",
            "unknown patch `q`",
        ),
        (
            "patch p { oscillator(sine) |> output; } assign violin -> p; route violin -> hall;",
            "unknown destination `hall`",
        ),
        (
            "patch p { oscillator(sine) |> output; } assign violin -> p; send violin -> hall at -6 dB;",
            "unknown bus `hall`",
        ),
        (
            "patch p { oscillator(sine) |> output; } route violin -> master;",
            "`violin` is not an assigned part or a bus",
        ),
        (
            "patch p { oscillator(sine) |> output; } modulate lfo -> p.oscillator.frequency;",
            "unknown signal `lfo`",
        ),
        (
            "patch p { oscillator(sine) |> output; } lfo = oscillator(sine); modulate lfo -> p.lowpass.cutoff;",
            "`p` has no stage named `lowpass`",
        ),
        (
            "patch p { oscillator(sine) |> output; } lfo = oscillator(sine); modulate lfo -> p.oscillator.cutoff;",
            "`oscillator` has no parameter `cutoff`",
        ),
        (
            "patch p { oscillator(sine) |> flange() |> output; }",
            "unknown processor `flange`",
        ),
        ("patch p { oscillator(sine); }", "patch `p` never reaches `output`"),
        (
            "patch p { oscillator(sine) |> output; } patch p { oscillator(sine) |> output; }",
            "duplicate patch `p`",
        ),
    ];
    for (studio, expected) in cases {
        let errors = errors_of(&piece(studio));
        assert!(errors.contains(&expected.to_owned()), "for `{studio}`: got {errors:?}");
    }
}

#[test]
fn an_ambiguous_modulation_path_is_reported_rather_than_guessed() {
    // Two unnamed stages of the same processor: picking one silently would
    // move a parameter the user did not point at.
    let errors = errors_of(&piece(
        "patch p { oscillator(sine) |> lowpass(cutoff: 800 Hz) |> lowpass(cutoff: 900 Hz) |> output; } \
         lfo = oscillator(sine); modulate lfo -> p.lowpass.cutoff;",
    ));
    assert!(
        errors.contains(&"`p` has more than one `lowpass`".to_owned()),
        "got {errors:?}"
    );
}

#[test]
fn a_send_level_must_be_written_in_decibels() {
    let errors = errors_of(&piece(
        "patch p { oscillator(sine) |> output; } bus hall { reverb(room: 0.5); } \
         assign violin -> p; send violin -> hall at 0.5;",
    ));
    assert!(
        errors.contains(&"a send level is written in `dB`".to_owned()),
        "got {errors:?}"
    );
}
