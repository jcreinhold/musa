//! Gradual tempo change.
//!
//! One claim, in three pieces. A *rit.* written in the score is a **shape**,
//! not a series of speeds: what is stored is where it arrives and how far it
//! takes, and the elapsed time is integrated from that exactly. The
//! arithmetic is checkable by hand, which is what these tests do.
//!
//! The decision under all of it is that the ramp is linear in **seconds per
//! beat** rather than in beats per minute. Interpolating bpm makes each
//! beat's duration a reciprocal, so the integral leaves the rationals and the
//! frame numbers stop being exact. Interpolating duration keeps everything
//! rational — and orchestral practice hears an even slowdown as even in
//! duration, so the representable answer is the musical one.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{IntegratedTempoMap, MusicalTime, Scope, ScoreSnapshot};
use num_rational::Ratio;
use proptest::prelude::*;

const RATE: u32 = 48_000;

fn score_of(source: &str) -> ScoreSnapshot {
    let compilation = compile(&SourceDocument::new(source, "ramp.musa"), &CompileOptions::default());
    let messages: Vec<String> = compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    compilation
        .into_snapshot()
        .unwrap_or_else(|| panic!("expected a snapshot; diagnostics: {messages:?}"))
}

/// A piece whose header states `header` and whose one voice plays `voice`.
fn piece(header: &str, voice: &str) -> String {
    format!("piece \"P\" {{ {header} meter 4/4; key c major; score {{ part p {{ voice v {{ {voice} }} }} }} }}")
}

/// Sixteen whole notes, so any position up to 16 has a note at it.
const SIXTEEN_WHOLES: &str = "c5/1 c5/1 c5/1 c5/1 c5/1 c5/1 c5/1 c5/1
     c5/1 c5/1 c5/1 c5/1 c5/1 c5/1 c5/1 c5/1";

fn frames_at(score: &ScoreSnapshot, whole_notes: i64) -> u64 {
    IntegratedTempoMap::new(score, Scope::Piece).frames(MusicalTime::new(Ratio::from_integer(whole_notes)), RATE)
}

/// The arithmetic, on a case a reader can do in their head.
///
/// A quarter at 60 is one second, so a whole note is four. A quarter at 30 is
/// two seconds, so a whole note is eight. Slowing evenly from one to the
/// other across four whole notes takes the *average* of the two durations at
/// every point, so the whole ramp lasts `4 × (4 + 8) / 2 = 24` seconds — and
/// halfway through, at two whole notes, `2 × (4 + 6) / 2 = 10`.
///
/// Both are whole seconds, which is the tell: linear in duration integrates
/// to a rational, and there is no rounding anywhere for it to hide in.
#[test]
fn a_ramp_takes_the_average_of_the_durations_it_passes_through() {
    let score = score_of(&piece("tempo 1/4 = 60 to 30 over 4/1;", SIXTEEN_WHOLES));
    assert_eq!(frames_at(&score, 0), 0);
    assert_eq!(frames_at(&score, 2), u64::from(RATE) * 10, "halfway");
    assert_eq!(frames_at(&score, 4), u64::from(RATE) * 24, "the end of the ramp");
}

/// The speed after a ramp is the speed it reached, and it stays there. Four
/// more whole notes at a quarter = 30 is thirty-two seconds more.
#[test]
fn the_tempo_after_a_ramp_is_the_one_it_arrived_at() {
    let score = score_of(&piece("tempo 1/4 = 60 to 30 over 4/1;", SIXTEEN_WHOLES));
    assert_eq!(frames_at(&score, 8), u64::from(RATE) * (24 + 32));
}

/// A ramp that never changes speed is the constant path, exactly — the two
/// implementations checking each other rather than one checking itself.
#[test]
fn a_ramp_that_goes_nowhere_is_the_tempo_it_started_at() {
    let ramped = score_of(&piece("tempo 1/4 = 96 to 96 over 3/1;", SIXTEEN_WHOLES));
    let plain = score_of(&piece("tempo 1/4 = 96;", SIXTEEN_WHOLES));
    for whole_notes in 0..12 {
        assert_eq!(
            frames_at(&ramped, whole_notes),
            frames_at(&plain, whole_notes),
            "at {whole_notes} whole notes"
        );
    }
}

/// A ramp cut short by the next marking is clamped by intersection rather
/// than by a rule of its own: nothing ever asks it past the cut, because the
/// next marking answers from there.
#[test]
fn the_next_marking_ends_a_ramp_early() {
    let source = piece(
        "tempo 1/4 = 60 to 30 over 8/1;",
        "c5/1 c5/1 tempo 1/4 = 60; c5/1 c5/1 c5/1 c5/1",
    );
    let score = score_of(&source);
    // Two whole notes into an 8-whole ramp from 4 s to 8 s per whole: the
    // duration reached is 5 s, so the elapsed time is 2 × (4 + 5) / 2 = 9.
    assert_eq!(frames_at(&score, 2), u64::from(RATE) * 9);
    // And from there it is 60 again: four seconds a whole note, flat.
    assert_eq!(frames_at(&score, 3), u64::from(RATE) * 13);
    assert_eq!(frames_at(&score, 4), u64::from(RATE) * 17);
}

/// A ramp that says only a word prints and moves nothing — the same law a
/// text-only marking states, which says the two halves really are separate
/// rather than separate-looking.
#[test]
fn a_worded_ramp_moves_no_clock() {
    let worded = score_of(&piece(
        "tempo 1/4 = 60;",
        "c5/1 tempo \"rit.\" over 2/1; c5/1 c5/1 c5/1",
    ));
    let plain = score_of(&piece("tempo 1/4 = 60;", "c5/1 c5/1 c5/1 c5/1"));
    for whole_notes in 0..4 {
        assert_eq!(frames_at(&worded, whole_notes), frames_at(&plain, whole_notes));
    }
    let marking = worded
        .tempo_at(Scope::Piece, MusicalTime::new(Ratio::from_integer(1)))
        .expect("the word is in the score");
    assert_eq!(marking.text.as_deref(), Some("rit."));
    assert!(marking.ramp.is_some(), "and it is a ramp, with a reach");
}

/// Half a ramp is two halves of it: the shape is normative, so a consumer
/// that asks about the middle gets the same answer as one that walks there.
///
/// The property that would fail if the integration were done in bpm: the
/// elapsed time at `u` would not be rational, so no two consumers rounding
/// independently would agree.
#[test]
fn a_ramp_is_the_sum_of_its_parts_at_every_point() {
    let score = score_of(&piece("tempo 1/4 = 72 to 144 over 6/1;", SIXTEEN_WHOLES));
    let map = IntegratedTempoMap::new(&score, Scope::Piece);
    let at = |sixths: i64| map.frames(MusicalTime::new(Ratio::new(sixths, 6)), RATE);
    let mut previous = 0;
    for sixths in 1..=36 {
        let now = at(sixths);
        assert!(now > previous, "the map is monotone at {sixths}/6");
        previous = now;
    }
    // 72 bpm is 10/3 s a whole note, 144 is 5/3. The average over the ramp is
    // 5/2, so six whole notes take fifteen seconds exactly.
    assert_eq!(at(36), u64::from(RATE) * 15);
}

/// The two halves of a gradual change only mean anything together, so one
/// without the other is refused rather than guessed at.
#[test]
fn a_ramp_needs_both_a_destination_and_a_reach() {
    for (voice, expected) in [
        ("tempo 1/4 = 60 to 30; c5/1", "this gradual tempo change has no reach"),
        (
            "tempo 1/4 = 60 over 4/1; c5/1",
            "this gradual tempo change goes nowhere",
        ),
    ] {
        let compilation = compile(
            &SourceDocument::new(piece("tempo 1/4 = 60;", voice), "ramp.musa"),
            &CompileOptions::default(),
        );
        let messages: Vec<String> = compilation
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect();
        assert!(
            messages.iter().any(|message| message.contains(expected)),
            "`{voice}` should be refused with `{expected}`, got {messages:?}"
        );
    }
}

proptest! {
    /// The cross-check, over every speed and every reach rather than the
    /// tidy ones: a ramp whose destination is where it started is the
    /// constant path, exactly.
    ///
    /// Two implementations checking each other. The constant path is the one
    /// every existing test already pins, so a ramp that agrees with it
    /// everywhere has not quietly changed what an unramped piece sounds like.
    #[test]
    fn a_constant_ramp_agrees_with_the_constant_path(bpm in 20u32..=240, reach in 1i64..=8) {
        let ramped = score_of(&piece(
            &format!("tempo 1/4 = {bpm} to {bpm} over {reach}/1;"),
            SIXTEEN_WHOLES,
        ));
        let plain = score_of(&piece(&format!("tempo 1/4 = {bpm};"), SIXTEEN_WHOLES));
        for whole_notes in 0..16 {
            prop_assert_eq!(frames_at(&ramped, whole_notes), frames_at(&plain, whole_notes));
        }
    }
}
