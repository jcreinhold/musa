//! Compiler contract tests: snapshot shape, diagnostics, and the algebraic
//! laws of roadmap §5.1–§5.2 as properties.

// The monoid laws below exercise the time operators directly; those ops are
// total for musa's magnitudes (see `musa-compiler/src/time.rs`).
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{Compilation, CompileOptions, MusicalDuration, Severity, SourceDocument, compile};
use num_rational::Ratio;
use proptest::prelude::*;

const GLASS_MOUNTAIN: &str = include_str!("../../../examples/glass-mountain.musa");
const INVENTION: &str = include_str!("../../../examples/invention.musa");
const COUNTERPOINT: &str = include_str!("../../../examples/counterpoint.musa");

fn compile_source(text: &str) -> Compilation {
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
}

fn messages(compilation: &Compilation) -> Vec<String> {
    compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| format!("{:?}: {}", diagnostic.severity, diagnostic.message))
        .collect()
}

#[test]
fn examples_compile_with_snapshots() {
    for source in [GLASS_MOUNTAIN, INVENTION, COUNTERPOINT] {
        let compilation = compile_source(source);
        assert!(
            !compilation.has_errors(),
            "errors: {}",
            messages(&compilation).join("\n")
        );
        assert!(compilation.snapshot().is_some());
    }
}

#[test]
fn counterpoint_snapshot_shape() {
    let compilation = compile_source(COUNTERPOINT);
    let snapshot = compilation.snapshot();
    assert!(snapshot.is_some());
    let Some(snapshot) = snapshot else { return };
    insta::assert_snapshot!(format!("{snapshot:#?}"));
}

#[test]
fn motif_application_warns_and_is_skipped() {
    let compilation = compile_source(GLASS_MOUNTAIN);
    let warnings = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .count();
    assert!(
        warnings > 0,
        "expected skip warnings: {}",
        messages(&compilation).join("\n")
    );
    // The violin's lead voice exists but is empty: its items were skipped.
    let snapshot = compilation.snapshot();
    let Some(snapshot) = snapshot else { return };
    let violin = snapshot
        .parts
        .iter()
        .find(|(_, part)| part.name == "violin")
        .map(|(_, part)| part);
    let Some(violin) = violin else { return };
    let lead = violin.voices.values().next();
    assert_eq!(lead.map(|voice| voice.events.len()), Some(0));
    // The strings part lowered fully: 8 events, total span 4 whole notes.
    let strings = snapshot
        .parts
        .iter()
        .find(|(_, part)| part.name == "strings")
        .map(|(_, part)| part);
    let Some(strings) = strings else { return };
    let total: usize = strings.voices.values().map(|voice| voice.events.len()).sum();
    assert_eq!(total, 8);
}

#[test]
fn unknown_clef_is_an_error() {
    let compilation = compile_source("piece \"x\" { score { part p { clef soprano; voice v { c5 1; } } } }");
    assert!(compilation.has_errors());
    assert!(
        messages(&compilation)
            .iter()
            .any(|message| message.contains("unknown clef"))
    );
    assert!(compilation.snapshot().is_none());
}

#[test]
fn duplicate_part_is_an_error() {
    let compilation =
        compile_source("piece \"x\" { score { part p { voice v { c5 1; } } part p { voice w { d5 1; } } } }");
    assert!(compilation.has_errors());
    assert!(
        messages(&compilation)
            .iter()
            .any(|message| message.contains("duplicate part"))
    );
}

#[test]
fn duplicate_motif_is_an_error() {
    let compilation = compile_source(
        "piece \"x\" { motif m() { c5 1; } motif m() { d5 1; } score { part p { voice v { c5 1; } } } }",
    );
    assert!(compilation.has_errors());
    assert!(
        messages(&compilation)
            .iter()
            .any(|message| message.contains("duplicate motif"))
    );
}

#[test]
fn provenance_points_back_to_source() {
    let compilation = compile_source(COUNTERPOINT);
    let Some(snapshot) = compilation.snapshot() else { return };
    let violin = snapshot
        .parts
        .iter()
        .find(|(_, part)| part.name == "violin")
        .map(|(_, part)| part);
    let Some(violin) = violin else { return };
    let Some(first) = violin.voices.values().next().and_then(|voice| voice.events.first()) else {
        return;
    };
    // The first event's origin span covers `d4 1/2;` in the source.
    let span = first.origin.source_span;
    let covered = COUNTERPOINT.get(span.start as usize..span.end as usize);
    assert_eq!(covered, Some("d4 1/2;"));
    assert!(first.origin.expansion_path.is_empty());
}

// --- Laws ------------------------------------------------------------------

fn duration() -> impl Strategy<Value = MusicalDuration> {
    (0i64..=64, 1i64..=16).prop_map(|(numerator, denominator)| MusicalDuration::new(Ratio::new(numerator, denominator)))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn duration_monoid_zero_is_identity(a in duration()) {
        prop_assert_eq!(a + MusicalDuration::ZERO, a);
        prop_assert_eq!(MusicalDuration::ZERO + a, a);
    }

    #[test]
    fn duration_monoid_addition_is_associative(a in duration(), b in duration(), c in duration()) {
        prop_assert_eq!((a + b) + c, a + (b + c));
    }

    #[test]
    fn duration_monoid_addition_is_commutative(a in duration(), b in duration()) {
        prop_assert_eq!(a + b, b + a);
    }

    /// §5.2's span law at the snapshot level: lowering a sequence of notes
    /// yields span(a then b) = span(a) + span(b) and cumulative onsets.
    #[test]
    fn lowering_span_is_additive(
        durations in prop::collection::vec(
            prop::sample::select(vec!["1/4", "1/8", "1/2", "3/8", "1"]),
            1..=12
        )
    ) {
        let mut source = String::from("piece \"x\" { score { part p { voice v {\n");
        let mut expected = Ratio::from_integer(0);
        for duration_text in &durations {
            source.push_str("c4 ");
            source.push_str(duration_text);
            source.push_str(";\n");
            let (numerator, denominator) = duration_text.split_once('/').unwrap_or((duration_text, "1"));
            let n: i64 = numerator.parse().unwrap_or(0);
            let d: i64 = denominator.parse().unwrap_or(1);
            expected += Ratio::new(n, d);
        }
        source.push_str("} } } }");
        let compilation = compile_source(&source);
        assert!(!compilation.has_errors(), "errors: {}", messages(&compilation).join("\n"));
        if let Some(snapshot) = compilation.snapshot() {
            if let Some(voice) =
                snapshot.parts.iter().next().and_then(|(_, part)| part.voices.values().next())
            {
                assert_eq!(voice.span().as_ratio(), expected);
                // Onsets are cumulative: each event starts where the previous ended.
                let mut cursor = Ratio::from_integer(0);
                for event in &voice.events {
                    assert_eq!(event.onset.as_ratio(), cursor);
                    cursor += event.notated_duration.value.as_ratio();
                }
            } else {
                prop_assert!(false, "no voice in snapshot");
            }
        } else {
            prop_assert!(false, "no snapshot");
        }
    }
}
