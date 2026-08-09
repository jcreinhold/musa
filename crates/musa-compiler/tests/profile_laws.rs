//! Interpretation profiles (docs/prompts/28).
//!
//! The contract this file protects is the separation the roadmap's §2 table
//! states: a written mark is not a number until a profile says so. Three
//! claims carry it —
//!
//! - **neutrality**: a piece with no profile schedules exactly as it did
//!   before profiles existed (full gate, amplitude 1). This is the one that
//!   keeps the golden audio golden;
//! - **interpretation**: a gate of `g` on a written value `v` sounds for
//!   exactly `g·v`, for every ratio the language admits;
//! - **locality**: what a mark means depends on the part's profile and
//!   nothing else, so the same written score under two profiles is two
//!   performances.

// Test helpers use expect()/panic! on statically-valid inputs: a failure is a
// bug in the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::unwrap_used)]
// Frame arithmetic over small integers.
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{
    CompileOptions, PerformanceEvent, PerformanceLane, PerformanceOptions, ScoreSnapshot, Severity, SourceDocument,
    compile, lower_performance,
};
use proptest::prelude::*;

const PROFILE_FIXTURE: &str = include_str!("../../../examples/profile-fixture.musa");

fn score_of(text: &str) -> ScoreSnapshot {
    let compilation = compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default());
    let messages: Vec<String> = compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    compilation
        .into_snapshot()
        .unwrap_or_else(|| panic!("expected a snapshot; got {}", messages.join("; ")))
}

fn errors_of(text: &str) -> Vec<String> {
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

/// `(on, off, amplitude)` per note of one lane, in schedule order.
///
/// A note-on and its note-off are two events joined by an instance id; every
/// assertion here is about the span between them, so the pairing is done once.
fn sounded(lane: &PerformanceLane) -> Vec<(u64, u64, f32)> {
    let mut notes = Vec::new();
    for event in lane.events() {
        let PerformanceEvent::NoteOn { frame, note, instance } = event else {
            continue;
        };
        let off = lane
            .events()
            .iter()
            .find_map(|other| {
                let PerformanceEvent::NoteOff { frame, instance: id } = other else {
                    return None;
                };
                (id == instance).then_some(*frame)
            })
            .unwrap_or(*frame);
        notes.push((*frame, off, note.amplitude));
    }
    notes
}

/// `(on, off, amplitude)` per note of the first part, in schedule order.
fn notes_of(snapshot: &ScoreSnapshot) -> Vec<(u64, u64, f32)> {
    let plan = lower_performance(snapshot, &PerformanceOptions::default()).expect("lowers");
    sounded(plan.lanes().first().expect("one lane"))
}

/// A one-part piece whose profile block and voice body are both supplied.
fn piece(profiles: &str, part_profile: &str, body: &str) -> String {
    format!(
        "piece \"x\" {{ tempo 1/4 = 60; meter 4/4; performance {{ {profiles} }} \
         score {{ part p {{ {part_profile} voice v {{ {body} }} }} }} }}"
    )
}

// --- Neutrality -------------------------------------------------------------

#[test]
fn a_piece_without_profiles_is_scheduled_neutrally() {
    let snapshot =
        score_of("piece \"x\" { tempo 1/4 = 60; meter 4/4; score { part p { voice v { c4/1 staccato } } } }");
    assert!(snapshot.profiles().is_empty());
    let notes = notes_of(&snapshot);
    // A whole note at quarter=60 is 4 seconds; the staccato is written but
    // uninterpreted, so it still sounds its full value.
    assert_eq!(notes, vec![(0, 4 * 48_000, 1.0)]);
}

#[test]
fn a_part_without_a_profile_is_neutral_even_when_the_piece_declares_one() {
    let source = piece("profile violin { mark staccato { gate = 0.5; } }", "", "c4/1 staccato");
    let notes = notes_of(&score_of(&source));
    assert_eq!(notes, vec![(0, 4 * 48_000, 1.0)], "declared is not the same as chosen");
}

// --- Interpretation ---------------------------------------------------------

#[test]
fn a_gate_shortens_the_sounding_value_and_leaves_the_onset_alone() {
    let source = piece(
        "profile violin { mark staccato { gate = 0.25; } }",
        "profile violin;",
        "c4/2 staccato c4/2",
    );
    let notes = notes_of(&score_of(&source));
    let half = 2 * 48_000;
    assert_eq!(
        notes,
        vec![(0, half / 4, 1.0), (half, half + half, 1.0)],
        "the second note starts where it is written, not where the first stopped sounding"
    );
}

/// A gate may be written as a ratio, and it has to *arrive*.
///
/// `gate = 1/2` used to parse as a decimal, fail, and resolve to nothing —
/// the note performed at full length and the piece compiled clean, so the
/// only symptom was the sound. The assertion is on the sounded value for
/// that reason: a test that the rule was stored would have passed throughout.
#[test]
fn a_gate_written_as_a_ratio_shortens_the_note() {
    let source = piece(
        "profile violin { mark staccato { gate = 1/4; } }",
        "profile violin;",
        "c4/1 staccato",
    );
    let notes = notes_of(&score_of(&source));
    assert_eq!(notes, vec![(0, 48_000, 1.0)], "a quarter of four seconds");
}

/// A fermata lengthens the note and leaves the next one where it was written.
///
/// The bar does not wait: a hold that stopped the clock would be a tempo fact,
/// which musa cannot yet state.
#[test]
fn a_hold_lengthens_the_note_without_moving_the_next() {
    let source = piece(
        "profile organ { mark fermata { hold = 2/1; } }",
        "profile organ;",
        "c4/2 fermata c4/2",
    );
    let notes = notes_of(&score_of(&source));
    let half = 2 * 48_000;
    assert_eq!(
        notes,
        vec![(0, 2 * half, 1.0), (half, half + half, 1.0)],
        "the first rings on under the second, which starts where it is written"
    );
}

#[test]
fn gates_multiply_when_a_note_carries_two_realized_marks() {
    let source = piece(
        "profile violin { mark staccato { gate = 0.5; } mark accent { gate = 0.5; } }",
        "profile violin;",
        "c4/1 staccato>",
    );
    let notes = notes_of(&score_of(&source));
    assert_eq!(notes.first().map(|note| note.1), Some(48_000));
}

#[test]
fn a_dynamic_sets_the_amplitude_from_its_note_onward() {
    let source = piece(
        "profile violin { dynamic p { amplitude = 0.25; } dynamic f { amplitude = 1; } }",
        "profile violin;",
        "c4/4 dynamic p; d4/4 e4/4 dynamic f; g4/4",
    );
    let amplitudes: Vec<f32> = notes_of(&score_of(&source)).iter().map(|note| note.2).collect();
    assert_eq!(amplitudes, vec![1.0, 0.25, 0.25, 1.0]);
}

#[test]
fn an_undeclared_mark_is_neutral_rather_than_an_error() {
    // A profile is a partial reading: what it says nothing about, it does
    // nothing to. Requiring every mark would make profiles unusable.
    let source = piece(
        "profile violin { mark staccato { gate = 0.5; } }",
        "profile violin;",
        "c4/1 tenuto dynamic ff; c4/1",
    );
    let snapshot = score_of(&source);
    let notes = notes_of(&snapshot);
    assert_eq!(notes.iter().map(|note| note.2).collect::<Vec<_>>(), vec![1.0, 1.0]);
    assert_eq!(notes.first().map(|note| note.1), Some(4 * 48_000));
}

#[test]
fn the_same_score_under_two_profiles_is_two_performances() {
    let snapshot = score_of(PROFILE_FIXTURE);
    let plan = lower_performance(&snapshot, &PerformanceOptions::default()).expect("lowers");
    // Both parts open with a staccato quarter. The flute reads it through
    // `winds` (gate 0.7, `p` = 0.45), the cello through `strings` (gate 0.5,
    // `mf` = 0.6) — the same written mark, two different sounds.
    let first: Vec<(u64, f32)> = plan
        .lanes()
        .iter()
        .map(|lane| {
            let (on, off, amplitude) = *sounded(lane).first().expect("each part sounds");
            (off - on, amplitude)
        })
        .collect();
    // A quarter at 100 bpm is 0.6 s = 28800 frames.
    assert_eq!(first, vec![(20_160, 0.45), (14_400, 0.6)]);
    insta::assert_snapshot!("profile_fixture", format!("{snapshot:#?}"));
}

// --- Diagnostics ------------------------------------------------------------

#[test]
fn a_part_naming_an_undeclared_profile_is_an_error() {
    let source = piece(
        "profile violin { dynamic p { amplitude = 0.5; } }",
        "profile viola;",
        "c4/1",
    );
    assert_eq!(errors_of(&source), vec!["cannot find profile `viola`".to_string()]);
}

#[test]
fn rules_reject_marks_the_language_does_not_have() {
    let source = piece("profile violin { mark sideways { gate = 0.5; } }", "", "c4/1");
    assert_eq!(errors_of(&source), vec!["`sideways` is not a mark".to_string()]);
    let source = piece("profile violin { dynamic loud { amplitude = 0.5; } }", "", "c4/1");
    assert_eq!(errors_of(&source), vec!["`loud` is not a dynamic marking".to_string()]);
}

#[test]
fn settings_are_checked_by_name_range_and_unit() {
    let cases = [
        (
            "profile v { mark staccato { swing = 0.5; } }",
            "a mark has no setting called `swing`",
        ),
        ("profile v { mark staccato { gate = 2; } }", "`gate` is outside 0 to 1"),
        (
            "profile v { mark staccato { gate = 8 ms; } }",
            "`gate` does not take a unit",
        ),
        (
            "profile v { mark staccato { attack = 8; } }",
            "`attack` is a length of time",
        ),
        (
            "profile v { dynamic p { level = 0.5; } }",
            "a dynamic has no setting called `level`",
        ),
        ("profile v { dynamic p { } }", "this dynamic rule says nothing"),
        // A hold is a multiple, not a fraction, so it has the opposite bound.
        ("profile v { mark fermata { hold = 1/2; } }", "`hold` is less than 1"),
        (
            "profile v { mark fermata { hold = 2 ms; } }",
            "`hold` does not take a unit",
        ),
        // A word where a quantity belongs. The grammar admits it — some
        // settings really are words — so the reader is what has to say no.
        ("profile v { mark fermata { hold = wide; } }", "`wide` is not a hold"),
        // The setting that used to vanish: a value the reader cannot parse is
        // refused out loud rather than leaving the rule silently neutral.
        (
            "profile v { mark staccato { gate = half; } }",
            "`half` is not a fraction of the written value",
        ),
    ];
    for (profiles, expected) in cases {
        let errors = errors_of(&piece(profiles, "", "c4/1"));
        assert!(errors.contains(&expected.to_string()), "for {profiles}: got {errors:?}");
    }
}

#[test]
fn a_duplicate_profile_is_an_error() {
    let source = piece(
        "profile v { dynamic p { amplitude = 0.5; } } profile v { dynamic f { amplitude = 0.9; } }",
        "",
        "c4/1",
    );
    assert_eq!(errors_of(&source), vec!["profile `v` is declared twice".to_string()]);
}

// --- Laws -------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// A gate of `n/d` on a written value sounds for exactly that fraction
    /// of it — measured in frames, which is where rounding could hide.
    #[test]
    fn a_gate_scales_the_sounding_value_exactly(numerator in 1u32..=20, denominator in 1u32..=20, beats in 1u32..=4) {
        prop_assume!(numerator <= denominator);
        // Written as a decimal, because that is what the language accepts.
        let gate = f64::from(numerator) / f64::from(denominator);
        let written = format!("{gate:.4}");
        let source = piece(
            &format!("profile v {{ mark staccato {{ gate = {written}; }} }}"),
            "profile v;",
            &format!("c4 {beats}/4 staccato rest {}/4", 4 - beats),
        );
        let notes = notes_of(&score_of(&source));
        let (on, off, _) = *notes.first().ok_or_else(|| TestCaseError::fail("no note"))?;
        // One beat is one second at quarter = 60, so the frame count is the
        // written value's seconds times the gate, rounded once.
        let seconds = f64::from(beats) * written.parse::<f64>().unwrap_or(0.0);
        let expected = (seconds * 48_000.0).round() as u64;
        prop_assert_eq!(off - on, expected);
    }

    /// Neutrality is not "close to" the unprofiled schedule; it is the same
    /// schedule. Any articulation, any dynamic, no profile: identical.
    #[test]
    fn writing_marks_without_a_profile_changes_no_frame(
        articulation in prop::sample::select(vec!["staccato", "tenuto", "accent", "marcato"]),
        dynamic in prop::sample::select(vec!["pp", "p", "mf", "f", "ff"]),
    ) {
        let marked = piece("", "", &format!("dynamic {dynamic}; c4 1/2 {articulation} d4/2"));
        let plain = piece("", "", "c4/2 d4/2");
        prop_assert_eq!(notes_of(&score_of(&marked)), notes_of(&score_of(&plain)));
    }
}
