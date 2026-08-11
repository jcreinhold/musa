//! Parts in their own meter and at their own tempo.
//!
//! One claim, twice: **polymeter and polytempo are a scope argument, not a
//! mechanism.** `Meter` and `Tempo` already inherited by `Override`
//! (`scope.rs`), and `BarLines` was already built on an arbitrary sequence of
//! meters, so a part that states its own reads its own and everything else
//! reads the piece's. The tests below are the consequences: whose barlines a
//! bar is checked against, whose meter a groove swings in, whose clock a note
//! is scheduled on — and, the one that matters most, that none of it moves a
//! note that was not asked to move.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    CompileOptions, MusicalTime, PerformanceOptions, Scope, ScoreSnapshot, SourceDocument, compile, lower_performance,
};
use num_rational::Ratio;

const BULGARIAN: &str = include_str!("../../../examples/bulgarian.musa");
const HEMIOLA: &str = include_str!("../../../examples/hemiola.musa");
const CANON_X: &str = include_str!("../../../examples/canon-x.musa");

fn score_of(source: &str) -> ScoreSnapshot {
    let compilation = compile(
        &SourceDocument::new(source, "polymeter.musa"),
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
        &SourceDocument::new(source, "polymeter.musa"),
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

fn frames(score: &ScoreSnapshot) -> Vec<Vec<u64>> {
    lower_performance(score, &PerformanceOptions::default())
        .expect("schedules")
        .lanes()
        .iter()
        .map(|lane| {
            lane.events()
                .iter()
                .map(musa_compiler::PerformanceEvent::frame)
                .collect()
        })
        .collect()
}

/// The barlines diverge, and each part counts its own. Eight bars of 7/8 and
/// seven of 4/4 cover the same seven whole notes, and neither part is ever
/// asked what the other's barline is doing.
#[test]
fn each_part_counts_in_its_own_meter() {
    let score = score_of(BULGARIAN);
    let (kaval, tupan) = (Scope::Part { part: 0 }, Scope::Part { part: 1 });
    assert_eq!(score.meter_at(kaval, MusicalTime::ZERO).numerator(), 7);
    assert_eq!(score.meter_at(tupan, MusicalTime::ZERO).numerator(), 4);
    // The piece's own meter is untouched by either: `Override` says a part
    // states its meter, it does not restate the piece's.
    assert_eq!(score.meter_at(Scope::Piece, MusicalTime::ZERO).numerator(), 4);
    let whole = |n: i64| MusicalTime::new(Ratio::from_integer(n));
    // Seven whole notes in: the eighth bar of 7/8, the eighth of 4/4.
    assert_eq!(score.bars(kaval).at(whole(7)).measure, 9);
    assert_eq!(score.bars(tupan).at(whole(7)).measure, 8);
    // And they do agree there — 8 × 7/8 = 7 × 1 — which is the only place
    // in the piece they do.
    assert_eq!(
        score.bars(kaval).at(whole(7)).into,
        musa_compiler::MusicalDuration::ZERO
    );
}

/// The cheap shape: the same bar length beamed two ways. Both staves have the
/// same barlines, so nothing about the grid changes — the whole difference is
/// what a beam covers, and that is the meter, which each part has.
#[test]
fn the_same_bar_length_needs_no_second_grid() {
    let score = score_of(HEMIOLA);
    let (treble, bass) = (Scope::Part { part: 0 }, Scope::Part { part: 1 });
    let length = |scope| score.bars(scope).measure_at(MusicalTime::ZERO).length();
    assert_eq!(length(treble), length(bass), "6/8 and 3/4 are the same bar");
    assert_eq!(score.meter_at(treble, MusicalTime::ZERO).denominator(), 8);
    assert_eq!(score.meter_at(bass, MusicalTime::ZERO).denominator(), 4);
}

/// A `bar` asserts "this is one measure", and under polymeter *whose* measure
/// is the question. Seven eighths is a bar in the 7/8 part and one eighth too
/// long in the piece's 4/4, and the same text has to be accepted in one and
/// refused in the other or the assertion means nothing.
#[test]
fn a_bar_is_one_measure_of_the_part_it_is_written_in() {
    let seven = "bar { c5/8 d5/8 e5/8 f5/8 g5/8 a5/8 b5/8 }";
    let with = format!("piece \"P\" {{ meter 4/4; score {{ part p {{ meter 7/8; voice v {{ {seven} }} }} }} }}");
    let compilation = compile(&SourceDocument::new(&with, "bar.musa"), &CompileOptions::default());
    assert!(compilation.diagnostics().is_empty(), "{:?}", compilation.diagnostics());
    refuses(
        &format!("piece \"P\" {{ meter 4/4; score {{ part p {{ voice v {{ {seven} }} }} }} }}"),
        "this bar is 1/8 short",
    );
}

/// Polymeter moves no note. The barlines are notation and the clock is not
/// (roadmap §2), so the same music with and without a part meter is scheduled
/// at exactly the same frames.
#[test]
fn a_barline_that_moves_moves_no_note() {
    let notes = "c5/8 d5/8 e5/8 f5/8 g5/8 a5/8 b5/8 c6/8";
    let piece = |part: &str| {
        format!("piece \"P\" {{ tempo 1/4 = 60; meter 4/4; score {{ part p {{ {part} voice v {{ {notes} }} }} }} }}")
    };
    assert_eq!(frames(&score_of(&piece("meter 7/8;"))), frames(&score_of(&piece(""))));
}

/// `0/4` is not a meter, in a part exactly as in the header: a fraction here
/// has to name real measures, and `none` is how music with no barlines says
/// so.
#[test]
fn a_part_meter_of_no_beats_must_say_so() {
    refuses(
        "piece \"P\" { meter 4/4; score { part p { meter 0/4; voice v { c5/4 } } } }",
        "this meter cannot be read",
    );
}

/// Two parts at their own tempos are two clocks, and the proof is that they
/// disagree: the accelerating part's second note arrives later than the
/// decelerating part's, because it started slower.
#[test]
fn a_part_at_its_own_tempo_is_played_at_it() {
    let plan = lower_performance(&score_of(CANON_X), &PerformanceOptions::default()).expect("schedules");
    assert!(plan.is_polytempo(), "the parts state different tempos");
    let onset = |lane: usize| {
        plan.lanes()
            .get(lane)
            .expect("a lane")
            .events()
            .iter()
            .map(musa_compiler::PerformanceEvent::frame)
            .find(|frame| *frame > 0)
            .expect("a second event")
    };
    // Both parts open on the downbeat, whatever their speed.
    assert_eq!(
        plan.lanes()
            .iter()
            .filter_map(|lane| lane.events().first().map(musa_compiler::PerformanceEvent::frame))
            .collect::<Vec<_>>(),
        vec![0, 0]
    );
    // The rising part starts at 60 and the falling one at 180, so the first
    // quarter of the rising part is the longer of the two by a wide margin.
    assert!(onset(0) > onset(1), "{} vs {}", onset(0), onset(1));
}

/// A piece whose parts agree is not polytempo, and says so — the flag exists
/// for the exporters, and one that fired on every piece would make the MIDI
/// warning noise.
#[test]
fn one_tempo_is_not_polytempo() {
    let plan = lower_performance(&score_of(BULGARIAN), &PerformanceOptions::default()).expect("schedules");
    assert!(!plan.is_polytempo());
}
