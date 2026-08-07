//! Differential validation (course correction §30 Step 5): the kernel
//! elaboration path must reproduce the direct lowerer's snapshots exactly —
//! positions, durations, spelling, identity, multiplicity, ordering, and
//! provenance — on fixtures and a generated corpus. Also the first
//! falsification fixtures (§33): twinkle, canon, counterpoint normal forms.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_compiler::{Compilation, CompileOptions, Elaboration, Severity, SourceDocument, compile, kernel_normal_form};
use proptest::prelude::*;

const GLASS_MOUNTAIN: &str = include_str!("../../../examples/glass-mountain.musa");
const INVENTION: &str = include_str!("../../../examples/invention.musa");
const COUNTERPOINT: &str = include_str!("../../../examples/counterpoint.musa");
const TWINKLE: &str = include_str!("../../../examples/twinkle.musa");
const CANON: &str = include_str!("../../../examples/canon.musa");

fn compile_with(text: &str, elaboration: Elaboration) -> Compilation {
    compile(
        &SourceDocument::new(text, "diff.musa"),
        &CompileOptions {
            elaboration,
            ..CompileOptions::default()
        },
    )
}

/// Full parity: snapshot equality (positions, durations, spelling, identity,
/// multiplicity, ordering, provenance) plus identical diagnostics.
fn assert_parity(text: &str) {
    let direct = compile_with(text, Elaboration::Direct);
    let kernel = compile_with(text, Elaboration::Kernel);
    let direct_diags: Vec<String> = direct
        .diagnostics()
        .iter()
        .map(|d| format!("{:?}:{}:{:?}", d.severity, d.message, d.span))
        .collect();
    let kernel_diags: Vec<String> = kernel
        .diagnostics()
        .iter()
        .map(|d| format!("{:?}:{}:{:?}", d.severity, d.message, d.span))
        .collect();
    assert_eq!(direct_diags, kernel_diags, "diagnostics diverged");
    let direct_snap = direct.into_snapshot();
    let kernel_snap = kernel.into_snapshot();
    assert_eq!(direct_snap.is_some(), kernel_snap.is_some(), "success diverged");
    if let (Some(old), Some(new)) = (direct_snap, kernel_snap) {
        assert_eq!(old, new, "snapshots diverged");
    }
}

#[test]
fn fixtures_have_full_parity() {
    for source in [GLASS_MOUNTAIN, INVENTION, COUNTERPOINT, TWINKLE, CANON] {
        assert_parity(source);
    }
}

#[test]
fn fixtures_have_errors_under_both_paths() {
    // Error and warning cases must behave identically, not just successes.
    let bad_motif_order = "piece \"x\" { motif a() { use b(); } motif b() { c4 1/4; } }";
    let unknown_motif = "piece \"x\" { score { part p { voice v { use nope(); } } } }";
    let double_accidental = "piece \"x\" { score { part p { voice v { transpose up P8 { transpose up P8 { transpose up m2 { css4 1/4; } } } } } } }";
    for source in [bad_motif_order, unknown_motif, double_accidental] {
        assert_parity(source);
    }
}

/// The direct lowerer is frozen (see `lower.rs`): the expressive notation
/// layer of prompt 27 lives only on the kernel path. Parity is kept by the
/// direct path *refusing* those constructs, not by re-implementing them, so
/// the boundary itself is the contract worth pinning.
#[test]
fn phase_two_constructs_are_kernel_only() {
    for (construct, body) in [
        ("slur", "slur { c4 1/4; d4 1/4; } rest 1/2;"),
        ("dynamic", "dynamic mf; c4 1;"),
        ("tuplet", "tuplet 3/2 { c4 1/8; d4 1/8; e4 1/8; } rest 3/4;"),
    ] {
        let source = format!("piece \"x\" {{ meter 4/4; score {{ part p {{ voice v {{ {body} }} }} }} }}");
        let document = SourceDocument::new(&source, "phase2.musa");
        let direct = compile(
            &document,
            &CompileOptions {
                elaboration: Elaboration::Direct,
                ..CompileOptions::default()
            },
        );
        // The refusal drops the construct's items, so the bar-length warning
        // fires too; the error is the fact under test.
        let errors: Vec<&str> = direct
            .diagnostics()
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(
            errors,
            vec![format!("`{construct}` needs the kernel elaboration path")],
            "the direct path must refuse `{construct}`, not mis-lower it"
        );
        let kernel = compile(
            &document,
            &CompileOptions {
                elaboration: Elaboration::Kernel,
                ..CompileOptions::default()
            },
        );
        assert!(
            kernel.diagnostics().is_empty(),
            "the kernel path accepts `{construct}`: {:?}",
            kernel.diagnostics()
        );
    }
}

#[test]
fn kernel_normal_forms_snapshot() {
    for (name, source) in [("twinkle", TWINKLE), ("canon", CANON), ("counterpoint", COUNTERPOINT)] {
        let form = kernel_normal_form(&SourceDocument::new(source, name)).expect("elaborates");
        insta::assert_snapshot!(name, form);
    }
}

/// The key and the meter are occurrences, and the snapshot's context maps are
/// a reading of them (prompt 40).
///
/// `fixtures_have_full_parity` already proves the reading reproduces the
/// direct lowerer's answer; this says where the answer now comes from, which
/// is the thing a refactor could quietly undo.
#[test]
fn the_key_and_the_meter_are_facts_of_the_timeline() {
    let source = "piece \"x\" { meter 3/4; key bf major; score { part p { voice v { c4 1/4; } } } }";
    let form = kernel_normal_form(&SourceDocument::new(source, "k")).expect("elaborates");
    assert!(form.contains("meter:3/4"), "meter is not an occurrence: {form}");
    assert!(form.contains("key:bf:major"), "key is not an occurrence: {form}");

    let snapshot = compile_with(source, Elaboration::Kernel)
        .into_snapshot()
        .expect("compiles");
    assert_eq!((snapshot.meter_map.numerator, snapshot.meter_map.denominator), (3, 4));
    assert_eq!(snapshot.key_map.map(|key| key.tonic.to_string()), Some("bf".to_owned()));
}

/// The elaborated kernel timelines are the same under semantic equality no
/// matter how the surface structured them: a written-out repeat equals its
/// unrolling.
#[test]
fn repeat_unrolls_to_the_same_kernel() {
    let repeated = "piece \"x\" { score { part p { voice v { repeat 3 { c4 1/4; d4 1/4; } } } } }";
    let unrolled = "piece \"x\" { score { part p { voice v { c4 1/4; d4 1/4; c4 1/4; d4 1/4; c4 1/4; d4 1/4; } } } }";
    let a = kernel_normal_form(&SourceDocument::new(repeated, "a")).expect("elaborates");
    let b = kernel_normal_form(&SourceDocument::new(unrolled, "b")).expect("elaborates");
    // Same temporal facts; provenance (and thus the snapshot) differs, which
    // is exactly the semantic quotient at work (course correction §20).
    assert_ne!(a, b);
    // Payload heads (identity, kind, pitch) and spans agree.
    let heads_and_spans = |form: &str| -> Vec<String> {
        form.lines()
            .map(|line| {
                let head = line.split('|').next().unwrap_or(line);
                let span = line.rsplit("from ").next().unwrap_or(line);
                format!("{head}|{span}")
            })
            .collect()
    };
    assert_eq!(heads_and_spans(&a), heads_and_spans(&b));
}

// --- Generated corpus -------------------------------------------------------

fn pitch_strategy() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["c4", "d4", "e4", "f4", "g4", "a4", "b4", "c5", "ef4", "fs4"])
}

fn duration_strategy() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["1/8", "1/4", "3/8", "1/2", "3/4", "1"])
}

fn item_strategy() -> impl Strategy<Value = String> {
    prop::sample::select(vec![0u8, 1, 2]).prop_flat_map(|kind| match kind {
        0 => (pitch_strategy(), duration_strategy())
            .prop_map(|(pitch, dur)| format!("{pitch} {dur};"))
            .boxed(),
        1 => duration_strategy().prop_map(|dur| format!("rest {dur};")).boxed(),
        _ => (pitch_strategy(), pitch_strategy(), duration_strategy())
            .prop_map(|(a, b, dur)| format!("chord ({a} {b}) {dur};"))
            .boxed(),
    })
}

fn voice_body_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(item_strategy(), 1..=8).prop_flat_map(|items| {
        let body = items.concat();
        // Optionally wrap a middle segment in repeat/transpose.
        prop::sample::select(vec![0u8, 1, 2]).prop_map(move |wrap| match wrap {
            0 => body.clone(),
            1 => format!("repeat 2 {{ {body} }}"),
            _ => format!("transpose up P5 {{ {body} }}"),
        })
    })
}

fn source_strategy() -> impl Strategy<Value = String> {
    (voice_body_strategy(), voice_body_strategy()).prop_map(|(upper, lower)| {
        format!(
            "piece \"gen\" {{ tempo 1/4 = 96; meter 4/4; key c major; score {{ part p {{ voice a {{ {upper} }} voice b {{ {lower} }} }} }} }}"
        )
    })
}

/// A source with a motif used through repeat and transpose.
fn motif_source_strategy() -> impl Strategy<Value = String> {
    (pitch_strategy(), pitch_strategy(), duration_strategy()).prop_map(|(root, other, dur)| {
        format!(
            "piece \"gen\" {{ meter 4/4; motif m(root: pitch = c4) {{ root {dur}; {other} {dur}; }} score {{ part p {{ voice v {{ use m({root}); repeat 2 {{ transpose down P5 {{ use m(); }} }} }} }} }} }}"
        )
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Parity on the generated corpus of plain items with repeat/transpose
    /// wrappers.
    #[test]
    fn generated_sources_have_parity(source in source_strategy()) {
        assert_parity(&source);
    }

    /// Parity with motifs, positional arguments, defaults, nesting.
    #[test]
    fn motif_sources_have_parity(source in motif_source_strategy()) {
        assert_parity(&source);
    }
}
