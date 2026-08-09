//! Where the barlines fall.
//!
//! Eight call sites across three crates depend on `at` and `time_of` being
//! inverses, and none of them checked it before this file existed. The
//! generators build arbitrary meters rather than fixed ones, so the property
//! already exercises the general case that mid-piece meter will make
//! reachable.

#![allow(clippy::expect_used)]

use musa_compiler::{BarLines, MusicalDuration, MusicalTime};
use num_rational::Ratio;
use proptest::prelude::*;

/// The meters a real piece is written in, plus the degenerate ones.
fn meter() -> impl Strategy<Value = (u32, u32)> {
    (0u32..16, prop::sample::select(vec![1u32, 2, 4, 8, 16]))
}

fn bars_of(numerator: u32, denominator: u32) -> BarLines {
    // A meter with no beats is spelled `none`: the degenerate case has a real
    // name now, and `0/4` is refused rather than quietly meaning it.
    let written = if numerator == 0 {
        "none".to_owned()
    } else {
        format!("{numerator}/{denominator}")
    };
    let source = format!("piece \"p\" {{ meter {written}; score {{ part a {{ voice b {{ rest 1; }} }} }} }}");
    let compiled = musa_compiler::compile(
        &musa_compiler::SourceDocument::new(&source, "bars.musa"),
        &musa_compiler::CompileOptions::default(),
    );
    compiled.snapshot().expect("a piece with a meter compiles").bars()
}

proptest! {
    /// `at` inverts `time_of` at every downbeat.
    #[test]
    fn at_inverts_time_of_on_downbeats((numerator, denominator) in meter(), measure in 1u32..64) {
        let bars = bars_of(numerator, denominator);
        let at = bars.time_of(measure, Ratio::ONE).expect("1:1 and after are in the coordinate system");
        let back = bars.at(at);
        if bars.meter_at(MusicalTime::ZERO).is_measured() {
            prop_assert_eq!(back.measure, measure);
            prop_assert_eq!(back.beat, Ratio::ONE);
            prop_assert_eq!(back.into, MusicalDuration::ZERO);
        } else {
            // An unmeasured piece is one measure, and `at` says so rather than
            // dividing by nothing.
            prop_assert_eq!(back.measure, 1);
        }
    }

    /// `at` inverts `time_of` off the downbeat too, in the meter's beat unit.
    #[test]
    fn at_inverts_time_of_within_a_measure(
        (numerator, denominator) in meter(),
        measure in 1u32..32,
        beat in 1i64..8,
    ) {
        let bars = bars_of(numerator, denominator);
        prop_assume!(bars.meter_at(MusicalTime::ZERO).is_measured());
        let beat = Ratio::from_integer(beat);
        // Only positions that land inside their own measure are positions.
        prop_assume!(beat <= Ratio::from_integer(i64::from(numerator)));
        let at = bars.time_of(measure, beat).expect("inside the coordinate system");
        let back = bars.at(at);
        prop_assert_eq!(back.measure, measure);
        prop_assert_eq!(back.beat, beat);
    }

    /// A moment always falls inside the measure that claims it.
    #[test]
    fn measure_at_contains_the_moment((numerator, denominator) in meter(), eighths in 0i64..256) {
        let bars = bars_of(numerator, denominator);
        prop_assume!(bars.meter_at(MusicalTime::ZERO).is_measured());
        let at = MusicalTime::new(Ratio::new(eighths, 8));
        let measure = bars.measure_at(at);
        prop_assert!(measure.start <= at);
        prop_assert!(at < measure.end);
        prop_assert_eq!(measure.number, bars.at(at).measure);
    }

    /// `closing` differs from `at` exactly on a barline, which is the whole
    /// reason both exist: a tuplet ending on a barline has not crossed it.
    #[test]
    fn closing_and_at_agree_off_the_barline((numerator, denominator) in meter(), eighths in 1i64..256) {
        let bars = bars_of(numerator, denominator);
        prop_assume!(bars.meter_at(MusicalTime::ZERO).is_measured());
        let at = MusicalTime::new(Ratio::new(eighths, 8));
        let position = bars.at(at);
        if position.into == MusicalDuration::ZERO {
            prop_assert_eq!(bars.closing(at), position.measure.saturating_sub(1).max(1));
        } else {
            prop_assert_eq!(bars.closing(at), position.measure);
        }
    }
}

/// The fixed cases the deleted helpers were written against, so the
/// replacements are compared with what they replaced rather than with
/// themselves.
#[test]
fn four_four_numbers_measures_the_way_the_old_helpers_did() {
    let bars = bars_of(4, 4);
    let quarter = |n: i64| MusicalTime::new(Ratio::new(n, 4));

    // `measure_of`: floor, 1-based.
    assert_eq!(bars.at(quarter(0)).measure, 1);
    assert_eq!(bars.at(quarter(3)).measure, 1);
    assert_eq!(bars.at(quarter(4)).measure, 2);
    assert_eq!(bars.at(quarter(9)).measure, 3);

    // `last_measure_of`: ceil, never below 1.
    assert_eq!(bars.closing(quarter(0)), 1);
    assert_eq!(bars.closing(quarter(4)), 1);
    assert_eq!(bars.closing(quarter(5)), 2);
    assert_eq!(bars.closing(quarter(8)), 2);

    // `positioned`: the offset into the measure, in whole notes.
    assert_eq!(bars.at(quarter(5)).into, MusicalDuration::new(Ratio::new(1, 4)));

    // `resolve_position`: `3:2` in 4/4 is two whole notes plus a quarter.
    assert_eq!(
        bars.time_of(3, Ratio::from_integer(2)),
        Some(MusicalTime::new(Ratio::new(9, 4)))
    );
}

/// A meter with no length makes the piece unmeasured, and the five scattered
/// zero guards this replaced all returned measure 1.
#[test]
fn a_meter_with_no_length_is_one_unbounded_measure() {
    let bars = bars_of(0, 4);
    assert!(!bars.meter_at(MusicalTime::ZERO).is_measured());
    assert_eq!(bars.at(MusicalTime::new(Ratio::from_integer(9))).measure, 1);
    // How far in is still a real quantity — it is what an engraver spaces an
    // unmeasured passage by, and the only coordinate it has.
    assert_eq!(
        bars.at(MusicalTime::new(Ratio::from_integer(9))).into,
        MusicalDuration::new(Ratio::from_integer(9))
    );
    assert_eq!(bars.closing(MusicalTime::new(Ratio::from_integer(9))), 1);
    assert_eq!(
        bars.measures_through(MusicalDuration::new(Ratio::from_integer(9)))
            .count(),
        1
    );
}

/// A piece with no notes still has a page.
#[test]
fn an_empty_piece_still_has_one_measure() {
    let bars = bars_of(4, 4);
    assert_eq!(bars.measures_through(MusicalDuration::ZERO).count(), 1);
}

/// `measures_through` covers the span and stops: the count `plan_staff`
/// computed by hand.
#[test]
fn measures_through_covers_the_span() {
    let bars = bars_of(4, 4);
    let whole = |n: i64| MusicalDuration::new(Ratio::from_integer(n));
    assert_eq!(bars.measures_through(whole(1)).count(), 1);
    assert_eq!(bars.measures_through(whole(2)).count(), 2);
    assert_eq!(bars.measures_through(MusicalDuration::new(Ratio::new(5, 4))).count(), 2);
    let numbers: Vec<u32> = bars.measures_through(whole(3)).map(|measure| measure.number).collect();
    assert_eq!(numbers, vec![1, 2, 3]);
}

/// The coordinate system starts at `1:1`; nothing before it is a position.
#[test]
fn positions_before_the_first_downbeat_are_not_positions() {
    let bars = bars_of(4, 4);
    assert_eq!(bars.time_of(0, Ratio::ONE), None);
    assert_eq!(bars.time_of(1, Ratio::ZERO), None);
    assert!(bars.time_of(1, Ratio::ONE).is_some());
}
