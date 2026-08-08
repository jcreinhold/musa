//! `LilyPond` backend tests: snapshots, determinism, duration spelling, and
//! name sanitization.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_render::{NotationOptions, NotationTarget, render_notation};

const EXAMPLES: [(&str, &str); 11] = [
    ("glass_mountain", include_str!("../../../examples/glass-mountain.musa")),
    ("invention", include_str!("../../../examples/invention.musa")),
    ("counterpoint", include_str!("../../../examples/counterpoint.musa")),
    ("twinkle", include_str!("../../../examples/twinkle.musa")),
    ("canon", include_str!("../../../examples/canon.musa")),
    ("tuplet_fixture", include_str!("../../../examples/tuplet-fixture.musa")),
    (
        "profile_fixture",
        include_str!("../../../examples/profile-fixture.musa"),
    ),
    ("annotated", include_str!("../../../examples/annotated.musa")),
    ("repeats", include_str!("../../../examples/repeats.musa")),
    ("modulation", include_str!("../../../examples/modulation.musa")),
    ("clef_change", include_str!("../../../examples/clef-change.musa")),
];

fn lilypond_of(text: &str) -> String {
    let score = compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles");
    render_notation(&score, NotationTarget::LilyPond, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string()
}

#[test]
fn example_lilypond_snapshots() {
    for (name, source) in EXAMPLES {
        insta::assert_snapshot!(name, lilypond_of(source));
    }
}

#[test]
fn output_is_deterministic() {
    assert_eq!(lilypond_of(EXAMPLES[2].1), lilypond_of(EXAMPLES[2].1));
}

/// Duration spelling: plain and dotted values map to `LilyPond` tokens; ties
/// use `~` and every piece spells from its own value, never the source
/// spelling of the whole event.
#[test]
fn duration_spelling_and_ties() {
    let text = "piece \"x\" { meter 2/4; score { part p { voice v { c4 1; d4 3/8; e4 1/16; } } } }";
    let out = lilypond_of(text);
    // Whole note in 2/4 splits into two tied halves — spelled `2`, not `1`.
    assert!(out.contains("c'2~ % event:0"), "first tied half:\n{out}");
    assert!(out.contains("c'2 % event:0"), "second piece shares the event:\n{out}");
    // Dotted eighth and sixteenth spell from their values.
    assert!(out.contains("d'4."), "dotted quarter:\n{out}");
    assert!(out.contains("e'16"), "sixteenth:\n{out}");
}

/// Pitches render absolute-octave English names; chords angle-bracket.
#[test]
fn pitch_and_chord_spelling() {
    let text =
        "piece \"x\" { meter 4/4; score { part p { voice v { css5 1/4; bff3 1/4; chord [c4, ef4, g4] 1/2; } } } }";
    let out = lilypond_of(text);
    assert!(out.contains("css''4"), "double sharp octave 5:\n{out}");
    assert!(out.contains("bff4"), "double flat octave 3 (no marks):\n{out}");
    assert!(out.contains("<c' ef' g'>2"), "chord:\n{out}");
}

/// Part names sanitize into stable variable names.
#[test]
fn variable_names_are_sanitized() {
    let text = "piece \"x\" { score { part horn_in_f { voice v { c4 1; } } } }";
    let out = lilypond_of(text);
    assert!(out.contains("partHornInF = {"), "sanitized:\n{out}");
    assert!(out.contains("\\new Staff \\partHornInF"), "score ref:\n{out}");
}

/// A voice with empty measures renders spacer skips (notation decision for
/// the uncovered region), and multi-voice parts emit simultaneous blocks.
#[test]
fn spacer_skips_and_simultaneous_voices() {
    let text = "piece \"x\" { meter 4/4; score { part p { voice a { c4 2; } voice b { rest 1/2; e4 1/2; } } } }";
    let out = lilypond_of(text);
    assert!(out.contains("<<"), "simultaneous block:\n{out}");
    assert!(out.contains("\\\\"), "voice separator:\n{out}");
    // voice b's measure 2 is uncovered: spacer whole skip.
    assert!(out.contains("s1"), "spacer skip:\n{out}");
}
