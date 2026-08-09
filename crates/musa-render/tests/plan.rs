//! Notation plan contract tests: measure splitting, beaming, tie
//! decomposition, and snapshots for the examples.

// Piece sums and measure bounds use exact rational arithmetic (total for
// musa's magnitudes; see musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{CompileOptions, ScoreSnapshot, SourceDocument, compile};
use musa_render::{NotationOptions, NotationPlan, plan_notation};
use num_rational::Ratio;
use proptest::prelude::*;

const GLASS_MOUNTAIN: &str = include_str!("../../../examples/glass-mountain.musa");
const INVENTION: &str = include_str!("../../../examples/invention.musa");
const COUNTERPOINT: &str = include_str!("../../../examples/counterpoint.musa");
const TUPLET_FIXTURE: &str = include_str!("../../../examples/tuplet-fixture.musa");
const ANNOTATED: &str = include_str!("../../../examples/annotated.musa");
const REPEATS: &str = include_str!("../../../examples/repeats.musa");

fn compile_score(text: &str) -> Option<ScoreSnapshot> {
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default()).into_snapshot()
}

fn plan(text: &str) -> Option<NotationPlan> {
    let score = compile_score(text)?;
    plan_notation(&score, &NotationOptions::default()).ok()
}

#[test]
fn example_plans_snapshot() {
    for (name, source) in [
        ("glass_mountain", GLASS_MOUNTAIN),
        ("invention", INVENTION),
        ("counterpoint", COUNTERPOINT),
        ("tuplet_fixture", TUPLET_FIXTURE),
        ("annotated", ANNOTATED),
        ("repeats", REPEATS),
    ] {
        let Some(rendered) = plan(source) else { return };
        insta::assert_snapshot!(name, format!("{rendered:#?}"));
    }
}

#[test]
fn whole_note_splits_across_measures_with_ties() {
    let source = "piece \"x\" { meter 2/4; score { part p { voice v { c4/1 } } } }";
    let Some(planned) = plan(source) else { return };
    let Some(staff) = planned.staves().first() else { return };
    assert_eq!(staff.measures().len(), 2);
    let mut pieces = Vec::new();
    for measure in staff.measures() {
        let Some(lane) = measure.lanes().first() else { continue };
        pieces.extend(lane.items().iter());
    }
    assert_eq!(pieces.len(), 2);
    // Both pieces point at the same event and form one tie chain.
    assert_eq!(
        pieces.first().map(|item| item.event()),
        pieces.get(1).map(|item| item.event())
    );
    assert!(pieces.first().is_some_and(|item| item.tie_start()));
    assert!(!pieces.first().is_some_and(|item| item.tie_stop()));
    assert!(pieces.get(1).is_some_and(|item| item.tie_stop()));
    assert!(!pieces.get(1).is_some_and(|item| item.tie_start()));
}

#[test]
fn beaming_follows_the_meter() {
    let Some(four_four) =
        plan("piece \"x\" { meter 4/4; score { part p { voice v { c4/8 c4/8 c4/8 c4/8 c4/8 c4/8 c4/8 c4/8 } } } }")
    else {
        return;
    };
    let Some(six_eight) =
        plan("piece \"x\" { meter 6/8; score { part p { voice v { c4/8 c4/8 c4/8 c4/8 c4/8 c4/8 } } } }")
    else {
        return;
    };
    let groups = |planned: &NotationPlan| -> Vec<Option<u32>> {
        planned
            .staves()
            .first()
            .and_then(|staff| staff.measures().first())
            .and_then(|measure| measure.lanes().first())
            .map(|lane| lane.items().iter().map(|item| item.beam().map(|beam| beam.0)).collect())
            .unwrap_or_default()
    };
    assert_eq!(
        groups(&four_four),
        vec![Some(0), Some(0), Some(1), Some(1), Some(2), Some(2), Some(3), Some(3)]
    );
    assert_eq!(
        groups(&six_eight),
        vec![Some(0), Some(0), Some(0), Some(1), Some(1), Some(1)]
    );
}

/// An irregular meter beams the way it is counted. 7/8 is 2+2+3 — the
/// grouping a player hears — and before this fact lived at the bottom of the
/// graph, a bar of it beamed as seven separate eighths.
#[test]
fn an_irregular_meter_beams_in_the_groups_it_is_counted_in() {
    let eighths = "c4/8 c4/8 c4/8 c4/8 c4/8 c4/8 c4/8";
    let Some(seven_eight) = plan(&std::format!(
        "piece \"x\" {{ meter 7/8; score {{ part p {{ voice v {{ {eighths} }} }} }} }}"
    )) else {
        return;
    };
    let groups: Vec<Option<u32>> = seven_eight
        .staves()
        .first()
        .and_then(|staff| staff.measures().first())
        .and_then(|measure| measure.lanes().first())
        .map(|lane| lane.items().iter().map(|item| item.beam().map(|beam| beam.0)).collect())
        .unwrap_or_default();
    assert_eq!(
        groups,
        vec![Some(0), Some(0), Some(1), Some(1), Some(2), Some(2), Some(2)]
    );
}

#[test]
fn non_binary_durations_need_tuplets() {
    let source = "piece \"x\" { meter 4/4; score { part p { voice v { c4/3 } } } }";
    let Some(score) = compile_score(source) else { return };
    let planned = plan_notation(&score, &NotationOptions::default());
    assert!(planned.is_err());
    let Err(error) = planned else { return };
    assert!(error.to_string().contains("tuplets"));
}

// --- Laws ------------------------------------------------------------------

fn voice_source(durations: &[&str], meter: &str) -> String {
    let mut source = format!("piece \"x\" {{ meter {meter}; score {{ part p {{ voice v {{\n");
    for duration in durations {
        source.push_str("c4 ");
        source.push_str(duration);
        source.push_str(";\n");
    }
    source.push_str("} } } }");
    source
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Tie decomposition is lossless: the pieces of every event sum to the
    /// event's duration, and every piece fits inside one measure.
    #[test]
    fn decomposition_is_lossless_and_measure_bounded(
        durations in prop::collection::vec(
            prop::sample::select(vec!["1/8", "1/4", "3/8", "1/2", "3/4", "1"]),
            1..=8
        ),
        meter in prop::sample::select(vec!["2/4", "3/4", "4/4", "6/8"])
    ) {
        let source = voice_source(&durations, meter);
        let Some(score) = compile_score(&source) else { return Ok(()) };
        let planned = plan_notation(&score, &NotationOptions::default());
        assert!(planned.is_ok(), "planning failed: {planned:?}");
        let Some(planned) = planned.ok() else { return Ok(()) };
        let measure_len = score.bars(musa_compiler::Scope::Piece).measure_at(musa_compiler::MusicalTime::ZERO).length().as_ratio();

        let mut sums: std::collections::HashMap<u64, Ratio<i64>> = std::collections::HashMap::new();
        for staff in planned.staves() {
            for measure in staff.measures() {
                for lane in measure.lanes() {
                    for item in lane.items() {
                        *sums.entry(item.event().0).or_insert(Ratio::ZERO) +=
                            item.duration().value.as_ratio();
                        let end = item.onset_in_measure().as_ratio() + item.duration().value.as_ratio();
                        assert!(end <= measure_len, "piece crosses the measure");
                    }
                }
            }
        }
        for (_, part) in score.parts().iter() {
            for voice in part.voices().map(|(_, voice)| voice) {
                for event in voice.events() {
                    let total = sums.get(&event.id.0).copied().unwrap_or(Ratio::ZERO);
                    assert_eq!(total, event.notated_duration.value.as_ratio());
                }
            }
        }
    }
}

/// The page's barlines and the performance's stop being the same barlines.
///
/// A repeat prints its body once and plays it twice, so a meter change written
/// after one sits at measure 3 on the page and measure 5 in the performance.
/// Prompt 61 built the two `BarLines` instances and could not tell them apart,
/// because until the meter could change there was nothing for them to disagree
/// about. This is that disagreement, asserted from both sides.
#[test]
#[expect(clippy::expect_used, reason = "a fixture that does not compile is a failed test")]
fn a_meter_change_after_a_repeat_is_numbered_twice() {
    let source = "piece \"p\" { meter 4/4; score { part a { voice b { \
                  repeat 2 { bar { c4/1 } bar { d4/1 } } \
                  meter 3/4; bar { e4/2. } } } } }";
    let score = compile_score(source).expect("it compiles");
    let played = score.bars(musa_compiler::Scope::Piece);
    let at = musa_compiler::MusicalTime::new(Ratio::from_integer(4));

    // Performed: two passes of two measures, so the change opens measure 5.
    assert_eq!(played.at(at).measure, 5);
    assert_eq!(played.meter_at(at).numerator(), 3);

    // Printed: the body is on the page once, so it opens measure 3.
    let plan = plan(source).expect("it plans");
    let staff = plan.staves().first().expect("one part");
    let changed: Vec<u32> = staff
        .measures()
        .iter()
        .filter(|measure| measure.time_signature().is_some())
        .map(musa_render::MeasurePlan::number)
        .collect();
    assert_eq!(changed, vec![1, 3], "the opening meter and the change");
    assert_eq!(staff.measures().len(), 3);
}
