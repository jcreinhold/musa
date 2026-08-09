//! Music with no barlines (docs/prompts/74).
//!
//! One claim: unmeasured is a **value of the meter**, not a mechanism beside
//! it. Everything here is a consequence of that — the barlines stop, the
//! clock does not; the passage is one measure, so the numbering does not
//! advance across it; and every check that is about barlines asks the meter
//! where it stands rather than assuming there is one.
//!
//! The measure-numbering law is the one worth reading twice. A cadenza inside
//! measure 5 is measure 5, and the movement resumes at 6 — which is how a
//! conductor's score numbers it, and is the reason the design keeps
//! `BarLines` contiguous instead of putting gaps in it.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    CompileOptions, MusicalTime, PerformanceOptions, ScoreSnapshot, SourceDocument, compile, lower_performance,
};
use num_rational::Ratio;

const CADENZA: &str = include_str!("../../../examples/cadenza.musa");
const CHANT: &str = include_str!("../../../examples/chant.musa");

fn score_of(source: &str) -> ScoreSnapshot {
    let compilation = compile(
        &SourceDocument::new(source, "unmeasured.musa"),
        &CompileOptions::default(),
    );
    let messages: Vec<String> = compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    compilation
        .into_snapshot()
        .unwrap_or_else(|| panic!("expected a snapshot; diagnostics: {messages:?}"))
}

fn refuses(source: &str, expected: &str) {
    let compilation = compile(
        &SourceDocument::new(source, "unmeasured.musa"),
        &CompileOptions::default(),
    );
    let messages: Vec<String> = compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert!(
        messages.iter().any(|message| message.contains(expected)),
        "expected `{expected}`, got {messages:?}"
    );
}

fn piece(header: &str, voice: &str) -> String {
    format!("piece \"P\" {{ {header} key c major; score {{ part p {{ voice v {{ {voice} }} }} }} }}")
}

/// The numbering law. Four measured bars, one unmeasured stretch however long
/// it runs, and the music after it is measure 6.
#[test]
fn a_cadenza_inside_a_measure_is_that_measure() {
    let score = score_of(CADENZA);
    let bars = score.bars(musa_compiler::Scope::Piece);
    let whole = |n: i64| MusicalTime::new(Ratio::from_integer(n));
    // Bars 1–4 are 4/4, so the fifth whole note is where measure 5 opens.
    assert_eq!(bars.at(whole(4)).measure, 5);
    assert!(!bars.meter_at(whole(4)).is_measured(), "and it is the cadenza");
    // The cadenza runs 1/8 × 8 + 1/16 × 4 + 1/4 + 1/2 = 2 whole notes, and
    // stays measure 5 the whole way.
    assert_eq!(bars.at(whole(5)).measure, 5);
    assert_eq!(bars.at(whole(6)).measure, 6, "the movement resumes at 6");
    assert!(bars.meter_at(whole(6)).is_measured());
}

/// A piece that is unmeasured from the first note is one measure throughout —
/// and nothing complains, because there is no barline for anything to be out
/// of step with.
#[test]
fn a_chant_is_one_measure_and_no_complaint() {
    let compilation = compile(&SourceDocument::new(CHANT, "chant.musa"), &CompileOptions::default());
    assert!(compilation.diagnostics().is_empty(), "{:?}", compilation.diagnostics());
    let score = compilation.into_snapshot().expect("compiles");
    let bars = score.bars(musa_compiler::Scope::Piece);
    assert!(!bars.meter_at(MusicalTime::ZERO).is_measured());
    assert_eq!(bars.at(MusicalTime::new(Ratio::from_integer(3))).measure, 1);
}

/// The clock does not stop when the barlines do. The same notes with and
/// without a meter are scheduled at the same frames, because "unmeasured" is
/// a fact about notation and the performance layer is not notation
/// (roadmap §2).
#[test]
fn unmeasured_music_is_performed_exactly() {
    let notes = "c5/8 d5/8 e5/4 f5/2";
    let free = score_of(&piece("tempo 1/4 = 60; meter none;", notes));
    let measured = score_of(&piece("tempo 1/4 = 60; meter 4/4;", notes));
    let frames = |score: &ScoreSnapshot| {
        lower_performance(score, &PerformanceOptions::default())
            .expect("schedules")
            .lanes()
            .iter()
            .flat_map(|lane| lane.events().iter().map(musa_compiler::PerformanceEvent::frame))
            .collect::<Vec<_>>()
    };
    assert_eq!(frames(&free), frames(&measured));
    assert_eq!(
        frames(&free),
        vec![0, 24_000, 24_000, 48_000, 48_000, 96_000, 96_000, 192_000]
    );
}

/// A `bar` asserts "this is one measure", and inside unmeasured music there
/// is no measure for it to be one of. Refused rather than ignored: an
/// assertion nobody checks is worse than no assertion.
#[test]
fn a_bar_inside_unmeasured_music_is_refused() {
    refuses(
        &piece("meter 4/4;", "senza { bar { c5/4 } }"),
        "a `bar` here has no measure to be one of",
    );
}

/// A groove displaces the beat inside a cell the meter names, and `meter
/// none` names none. Straightening it silently would drop the composer's feel
/// without a word.
#[test]
fn a_groove_needs_a_meter_to_swing_against() {
    let source = "piece \"P\" {
        meter none;
        performance { profile p { groove swing { ratio = 0.66; } } }
        score { part a { profile p; voice v { c5/8 d5/8 } } }
    }";
    refuses(source, "this groove has no beat to lay itself over");
}

/// `0/4` is not a meter with no beats — it is `none`, spelled the way the
/// grammar spells it. A fraction here has to name real measures, or the
/// barlines would fall nowhere by accident.
#[test]
fn a_meter_of_no_beats_must_say_so() {
    refuses(&piece("meter 0/4;", "c5/4"), "this meter cannot be read");
    refuses(&piece("meter 4/0;", "c5/4"), "this meter cannot be read");
}

/// `senza { ... }` is the two meter changes a composer could write by hand,
/// with the second impossible to forget. The proof is that writing them by
/// hand gives the same piece.
#[test]
fn senza_is_the_meter_changes_written_out() {
    // Not the semantic hash: that carries provenance, and the two spellings
    // are written at different places on purpose. What must agree is the
    // music — where the meters change, and where the notes fall.
    let shape = |voice: &str| {
        let score = score_of(&piece("meter 4/4;", voice));
        let meters: Vec<(MusicalTime, bool)> = score
            .meters()
            .changes(musa_compiler::Scope::Piece)
            .map(|(at, meter)| (at, meter.is_measured()))
            .collect();
        let onsets: Vec<MusicalTime> = score
            .parts()
            .iter()
            .flat_map(|(_, part)| part.voices())
            .flat_map(|(_, voice)| voice.events().iter().map(|event| event.onset))
            .collect();
        (meters, onsets)
    };
    assert_eq!(
        shape("c5/1 senza { d5/8 e5/8 } f5/1"),
        shape("c5/1 meter none; d5/8 e5/8 meter 4/4; f5/1"),
        "the same piece, said twice"
    );
}
