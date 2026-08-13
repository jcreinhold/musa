//! Unit tests for the kernel constructors and operations. The algebraic law
//! suite lives in `tests/laws.rs`.

// Rational test arithmetic is exact and total (musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]
// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_kernel::{Canonical, Duration, KernelError, Occurrence, Position, Span, WrittenTime, together, track};
use num_rational::Ratio;

/// An instant in written time — a [`Position`], because it answers *when*.
fn at(n: i64, d: i64) -> Position<WrittenTime> {
    Position::new(Ratio::new(n, d))
}

/// An amount of written time — a [`Duration`], because it answers *how much*.
/// Negative input is a bug in the test, so it panics rather than returning.
fn len(n: i64, d: i64) -> Duration<WrittenTime> {
    Duration::new(Ratio::new(n, d)).expect("test duration is nonnegative")
}

fn span(s: (i64, i64), e: (i64, i64)) -> Span<WrittenTime> {
    Span::new(at(s.0, s.1), at(e.0, e.1)).expect("test span is valid")
}

fn occurrence(s: (i64, i64), e: (i64, i64), payload: u8) -> Occurrence<WrittenTime, u8> {
    Occurrence::new(span(s, e), payload)
}

#[test]
fn construction_checks_bounds() {
    let good = track(len(2, 1), vec![occurrence((0, 1), (2, 1), 1)]);
    assert!(good.is_ok());
    let bad = track(len(1, 1), vec![occurrence((0, 1), (3, 2), 1)]);
    assert!(matches!(bad, Err(KernelError::OccurrenceOutOfBounds { .. })));
}

/// A negative ambient duration is now unrepresentable rather than rejected by
/// the constructor: `Duration` is the nonnegative half of exact time, so the
/// refusal happens where the amount is written.
#[test]
fn a_negative_duration_never_reaches_a_track() {
    let backwards = Duration::<WrittenTime>::new(Ratio::new(-1, 1));
    assert!(matches!(backwards, Err(KernelError::NegativeDuration { .. })));
}

#[test]
fn follow_translates_and_adds_durations() {
    let a = track(len(1, 1), vec![occurrence((0, 1), (1, 1), 1)]).expect("valid");
    let b = track(len(2, 1), vec![occurrence((1, 2), (2, 1), 2)]).expect("valid");
    let joined = musa_kernel::follow(vec![a, b]);
    assert_eq!(joined.duration(), len(3, 1));
    let spans: Vec<Span<WrittenTime>> = joined.occurrences().iter().map(Occurrence::span).collect();
    assert_eq!(spans.len(), 2);
    assert_eq!(spans.first().map(|span| span.start()), Some(at(0, 1)));
    assert_eq!(spans.get(1).map(|span| span.start()), Some(at(3, 2)));
}

#[test]
fn together_takes_max_duration_and_keeps_multiplicity() {
    let make = || track(len(1, 1), vec![occurrence((0, 1), (1, 1), 7)]).expect("valid");
    let both = together(vec![make(), make()]);
    // Same occurrence twice: the multiset keeps both (X1).
    assert_eq!(both.occurrences().len(), 2);
    let wide = track(len(5, 2), vec![]).expect("valid");
    let with_empty = together(vec![both, wide]);
    assert_eq!(with_empty.duration(), len(5, 2));
    assert_eq!(with_empty.occurrences().len(), 2);
}

#[test]
fn restrict_reports_whole_and_visible_spans() {
    let base = track(len(8, 1), vec![occurrence((3, 1), (6, 1), 1)]).expect("valid");
    let observation = base.restrict(span((5, 1), (8, 1)));
    let seen: Vec<(Span<WrittenTime>, Span<WrittenTime>)> = observation
        .observed()
        .map(|(visible, occurrence)| (occurrence.span(), visible))
        .collect();
    // The whole support is [3, 6): cropping must not claim the occurrence
    // began at 5 (docs/rules/kernel/03 D6).
    assert_eq!(seen, vec![(span((3, 1), (6, 1)), span((5, 1), (6, 1)))]);
}

#[test]
fn point_occurrences_are_observable() {
    let point = span((3, 1), (3, 1));
    let base = track(len(8, 1), vec![Occurrence::new(point, 1u8)]).expect("valid");
    assert_eq!(base.restrict(span((2, 1), (5, 1))).observed().count(), 1);
    assert!(base.restrict(span((4, 1), (8, 1))).is_empty());
}

/// The final instant of a track is observable (docs/rules/kernel/03 D6), and a
/// narrowed observation keeps it: the rule is about the track's duration,
/// not about the window it is first seen through.
#[test]
fn the_final_instant_survives_narrowing() {
    let end = span((8, 1), (8, 1));
    let base = track(len(8, 1), vec![Occurrence::new(end, 1u8)]).expect("valid");
    let full = base.restrict(span((0, 1), (8, 1)));
    assert_eq!(full.observed().count(), 1);
    assert_eq!(full.restrict(span((7, 1), (8, 1))).observed().count(), 1);
    assert!(full.restrict(span((0, 1), (7, 1))).is_empty());
}

#[test]
fn scale_requires_positive_factor() {
    let base = track(len(1, 1), vec![occurrence((0, 1), (1, 2), 1)]).expect("valid");
    let doubled = base.scale(Ratio::new(2, 1)).expect("positive factor");
    assert_eq!(doubled.duration(), len(2, 1));
    let spans: Vec<Span<WrittenTime>> = doubled.occurrences().iter().map(Occurrence::span).collect();
    assert_eq!(spans.first().map(|span| span.end()), Some(at(1, 1)));
    assert!(base.scale(Ratio::ZERO).is_err());
}

#[test]
fn canonical_order_is_start_end_payload() {
    let mutated = track(
        len(4, 1),
        vec![
            occurrence((1, 1), (2, 1), 9),
            occurrence((0, 1), (1, 1), 9),
            occurrence((0, 1), (1, 1), 3),
        ],
    )
    .expect("valid");
    let ordered = mutated.normalize();
    let keys: Vec<u8> = ordered.occurrences().iter().map(|o| *o.payload()).collect();
    assert_eq!(keys, vec![3, 9, 9]);
}

#[test]
fn serialization_is_deterministic_regardless_of_construction() {
    let a = track(
        len(2, 1),
        vec![occurrence((1, 1), (2, 1), 1), occurrence((0, 1), (1, 1), 2)],
    )
    .expect("valid");
    let b = track(
        len(2, 1),
        vec![occurrence((0, 1), (1, 1), 2), occurrence((1, 1), (2, 1), 1)],
    )
    .expect("valid");
    assert_eq!(format!("{a}"), format!("{b}"));
    assert!(a.semantic_eq(&b));
    assert!(format!("{a}").starts_with("track 2 {"));
}

#[test]
fn map_payloads_preserves_support() {
    let base = track(len(2, 1), vec![occurrence((0, 1), (1, 1), 2u8)]).expect("valid");
    let mapped = base.map_payloads(|p| format!("note-{p}"));
    assert_eq!(mapped.duration(), base.duration());
    let spans: Vec<Span<WrittenTime>> = mapped.occurrences().iter().map(Occurrence::span).collect();
    let base_spans: Vec<Span<WrittenTime>> = base.occurrences().iter().map(Occurrence::span).collect();
    assert_eq!(spans, base_spans);
    let first = mapped.occurrences().first().map(|o| o.payload().canonical_key());
    assert_eq!(first.as_deref(), Some("note-2"));
}
