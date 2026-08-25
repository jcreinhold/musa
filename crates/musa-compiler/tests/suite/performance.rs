//! Exact performance lowering laws before checked frame scheduling.

#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_score::{ScoreSnapshot, Tuning, lower_gestures};
use num_rational::Ratio;
use proptest::prelude::*;
use std::fmt::Write as _;

use super::performance_support::{ExactNote, notes_of};

const COUNTERPOINT: &str = include_str!("../../../../examples/counterpoint.musa");

fn score_of(text: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles")
}

fn plan_of(text: &str) -> Vec<Vec<ExactNote>> {
    notes_of(&score_of(text))
}

#[test]
fn exact_physical_scheduling_stops_before_frames() {
    let text = "piece \"x\" { tempo 1/4 = 72; meter 4/4; score { part p { voice v { a4/4 c5/4 } } } }";
    let plan = plan_of(text);
    let notes = plan.first().map_or(&[][..], Vec::as_slice);
    assert_eq!(notes.len(), 2);
    let a = notes.first().expect("the first exact gesture");
    let c = notes.get(1).expect("the second exact gesture");
    assert_eq!(a.on_seconds, Ratio::ZERO);
    assert_eq!(a.off_seconds, Ratio::new(5, 6));
    assert_eq!(c.on_seconds, Ratio::new(5, 6));
    assert_eq!(c.off_seconds, Ratio::new(5, 3));
    assert!((Tuning::default().frequency(&a.pitch) - 440.0).abs() < f64::EPSILON);
}

#[test]
fn chords_are_multiplicity_and_rests_are_absence() {
    let text = "piece \"x\" { tempo 1/4 = 60; score { part p { voice v { [c4 e4 g4]/4 rest/4 } } } }";
    assert_eq!(plan_of(text).first().map_or(0, Vec::len), 3);
}

#[test]
fn adjacent_gestures_share_one_exact_boundary() {
    let text = "piece \"x\" { tempo 1/4 = 60; score { part p { voice v { c4/4 c4/4 } } } }";
    let plan = plan_of(text);
    let notes = plan.first().map_or(&[][..], Vec::as_slice);
    assert_eq!(notes.len(), 2);
    let first = notes.first().expect("the first gesture");
    let second = notes.get(1).expect("the second gesture");
    assert_eq!(first.off_seconds, second.on_seconds);
}

#[test]
fn counterpoint_performance_snapshot() {
    let score = score_of(COUNTERPOINT);
    let plan = lower_gestures(&score).expect("lowers");
    let mut dump = String::new();
    for lane in plan.lanes() {
        let _ = writeln!(dump, "lane {}:", lane.name());
        for note in super::performance_support::notes_in(lane) {
            let _ = writeln!(
                dump,
                "  {}..{} {} amplitude={} event-{:x}",
                note.on_performed, note.off_performed, note.pitch, note.amplitude, note.event.0
            );
        }
    }
    insta::assert_snapshot!(dump);
}

fn item_strategy() -> impl Strategy<Value = String> {
    (
        prop::sample::select(vec!["c4", "d4", "e4", "f4", "g4"]),
        prop::sample::select(vec!["1/8", "1/4", "1/2"]),
    )
        .prop_map(|(pitch, dur)| format!("{pitch} {dur} "))
}

fn voice_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(item_strategy(), 1..=6).prop_map(|items| items.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn sequence_schedules_in_order(body in voice_strategy()) {
        let text = format!("piece \"x\" {{ tempo 1/4 = 120; score {{ part p {{ voice v {{ {body} }} }} }} }}");
        let plan = plan_of(&text);
        let notes = plan.first().map_or(&[][..], Vec::as_slice);
        prop_assert!(notes
            .iter()
            .zip(notes.iter().skip(1))
            .all(|(left, right)| left.on_seconds <= right.on_seconds));
        prop_assert!(notes.iter().all(|note| note.on_seconds <= note.off_seconds));
    }

    #[test]
    fn overlay_preserves_exact_spans(body in voice_strategy()) {
        let single = format!("piece \"x\" {{ tempo 1/4 = 120; score {{ part p {{ voice a {{ {body} }} }} }} }}");
        let doubled = format!(
            "piece \"x\" {{ tempo 1/4 = 120; score {{ part p {{ voice a {{ {body} }} voice b {{ {body} }} }} }} }}"
        );
        let single_plan = plan_of(&single);
        let doubled_plan = plan_of(&doubled);
        let single_notes = single_plan.first().map_or(&[][..], Vec::as_slice);
        let doubled_notes = doubled_plan.first().map_or(&[][..], Vec::as_slice);
        prop_assert_eq!(doubled_notes.len(), single_notes.len() * 2);
        for note in single_notes {
            prop_assert!(doubled_notes.iter().any(|other|
                other.on_seconds == note.on_seconds && other.off_seconds == note.off_seconds
            ));
        }
    }
}
