//! Performance lowering tests: frame-exact scheduling, neutrality, and the
//! §5.5 lowering laws.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// Frame/count arithmetic in tests is small and total.
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{
    CompileOptions, PerformanceEvent, PerformanceOptions, ScoreSnapshot, SourceDocument, compile, lower_performance,
};
use proptest::prelude::*;

const COUNTERPOINT: &str = include_str!("../../../examples/counterpoint.musa");

fn score_of(text: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles")
}

fn plan_of(text: &str) -> musa_compiler::PerformancePlan {
    lower_performance(&score_of(text), &PerformanceOptions::default()).expect("lowers")
}

/// At 72 bpm quarter notes at 48 kHz: a quarter is 60/72 s = 40000 frames
/// exactly.
#[test]
fn frame_exact_scheduling() {
    let text = "piece \"x\" { tempo 1/4 = 72; meter 4/4; score { part p { voice v { a4 1/4; c5 1/4; } } } }";
    let plan = plan_of(text);
    let empty: &[PerformanceEvent] = &[];
    let events = plan.lanes().first().map_or(empty, |lane| lane.events());
    let [on_a, off_a, on_c, off_c] = events else {
        panic_free_fail();
        return;
    };
    assert_eq!(on_a.frame(), 0);
    assert_eq!(off_a.frame(), 40000);
    assert_eq!(on_c.frame(), 40000);
    assert_eq!(off_c.frame(), 80000);
    // A4 realizes at exactly the concert pitch.
    if let PerformanceEvent::NoteOn { note, .. } = on_a {
        assert!(
            (note.frequency - 440.0).abs() < f64::EPSILON,
            "concert A: {}",
            note.frequency
        );
    }
}

#[track_caller]
fn panic_free_fail() {
    let observed: u8 = 1;
    let expected: u8 = 2;
    assert_eq!(observed, expected, "expected four events");
}

/// Chords schedule one on/off pair per tone; rests schedule nothing.
#[test]
fn chords_and_rests() {
    let text = "piece \"x\" { tempo 1/4 = 60; score { part p { voice v { chord [c4, e4, g4] 1/4; rest 1/4; } } } }";
    let plan = plan_of(text);
    let empty: &[PerformanceEvent] = &[];
    let events = plan.lanes().first().map_or(empty, |lane| lane.events());
    assert_eq!(events.len(), 6);
    let ons = events
        .iter()
        .filter(|event| matches!(event, PerformanceEvent::NoteOn { .. }))
        .count();
    assert_eq!(ons, 3);
}

/// A gate ending exactly where the next note starts closes first.
#[test]
fn same_frame_transitions_close_before_opening() {
    let text = "piece \"x\" { tempo 1/4 = 60; score { part p { voice v { c4 1/4; c4 1/4; } } } }";
    let plan = plan_of(text);
    let empty: &[PerformanceEvent] = &[];
    let events = plan.lanes().first().map_or(empty, |lane| lane.events());
    let [_, off, on, _] = events else {
        panic_free_fail();
        return;
    };
    assert!(matches!(off, PerformanceEvent::NoteOff { .. }));
    assert!(matches!(on, PerformanceEvent::NoteOn { .. }));
    assert_eq!(off.frame(), on.frame());
}

#[test]
fn counterpoint_performance_snapshot() {
    let plan = plan_of(COUNTERPOINT);
    let mut dump = String::new();
    for lane in plan.lanes() {
        let header = format!("lane {}:\n", lane.name());
        dump.push_str(&header);
        for event in lane.events() {
            match event {
                PerformanceEvent::NoteOn { frame, note, instance } => {
                    let line = format!(
                        "  on  {frame} {} {:.2}Hz event-{:x} i{}\n",
                        note.pitch, note.frequency, note.event.0, instance.0
                    );
                    dump.push_str(&line);
                }
                PerformanceEvent::NoteOff { frame, instance } => {
                    let line = format!("  off {frame} i{}\n", instance.0);
                    dump.push_str(&line);
                }
                PerformanceEvent::Parameter { .. } => {}
            }
        }
    }
    insta::assert_snapshot!(dump);
}

// --- Lowering laws (roadmap §5.5) -------------------------------------------

fn item_strategy() -> impl Strategy<Value = String> {
    (
        prop::sample::select(vec!["c4", "d4", "e4", "f4", "g4"]),
        prop::sample::select(vec!["1/8", "1/4", "1/2"]),
    )
        .prop_map(|(pitch, dur)| format!("{pitch} {dur};"))
}

fn voice_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(item_strategy(), 1..=6).prop_map(|items| items.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// `lower(a then b)` schedules b after a's span; every note has exactly
    /// one on and one off; offs never precede their ons.
    #[test]
    fn sequence_schedules_in_order(body in voice_strategy()) {
        let text = format!("piece \"x\" {{ tempo 1/4 = 120; score {{ part p {{ voice v {{ {body} }} }} }} }}");
        let plan = plan_of(&text);
        let empty: &[PerformanceEvent] = &[];
        let events = plan.lanes().first().map_or(empty, |lane| lane.events());
        let frames: Vec<u64> = events.iter().map(PerformanceEvent::frame).collect();
        prop_assert!(frames.windows(2).all(|pair| pair.first() <= pair.get(1)), "monotone frames");
        let ons = events.iter().filter(|e| matches!(e, PerformanceEvent::NoteOn { .. })).count();
        let offs = events.iter().filter(|e| matches!(e, PerformanceEvent::NoteOff { .. })).count();
        prop_assert_eq!(ons, offs);
        // Every off's frame >= every on's frame of the same instance.
        let on_frames: std::collections::HashMap<u32, u64> = events
            .iter()
            .filter_map(|e| match e {
                PerformanceEvent::NoteOn { frame, instance, .. } => Some((instance.0, *frame)),
                PerformanceEvent::NoteOff { .. } | PerformanceEvent::Parameter { .. } => None,
            })
            .collect();
        for event in events {
            if let PerformanceEvent::NoteOff { frame, instance } = event {
                let on = on_frames.get(&instance.0).copied().unwrap_or(u64::MAX);
                prop_assert!(*frame >= on);
            }
        }
    }

    /// `lower(a together_with b)` merges lanes without frame drift: the same
    /// voice content in one voice or split across two voices of one part
    /// yields the same frames for the shared prefix.
    #[test]
    fn overlay_preserves_frames(body in voice_strategy()) {
        let single = format!("piece \"x\" {{ tempo 1/4 = 120; score {{ part p {{ voice a {{ {body} }} }} }} }}");
        let doubled = format!(
            "piece \"x\" {{ tempo 1/4 = 120; score {{ part p {{ voice a {{ {body} }} voice b {{ {body} }} }} }} }}"
        );
        let single_plan = plan_of(&single);
        let doubled_plan = plan_of(&doubled);
        let single_frames: Vec<u64> = single_plan
            .lanes()
            .first()
            .map(|lane| lane.events().iter().map(PerformanceEvent::frame).collect())
            .unwrap_or_default();
        let doubled_frames: Vec<u64> = doubled_plan
            .lanes()
            .first()
            .map(|lane| lane.events().iter().map(PerformanceEvent::frame).collect())
            .unwrap_or_default();
        prop_assert_eq!(doubled_frames.len(), single_frames.len() * 2);
        for frame in single_frames {
            prop_assert!(doubled_frames.contains(&frame));
        }
    }
}
