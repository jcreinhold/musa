//! The temporal-kernel law suite (docs/kernel/04-algebraic-laws.md). Every
//! test name matches the law name in the spec; the non-laws X1–X2 are tested
//! as counterexamples. Equality is semantic equality (N4) throughout.

// Rational test arithmetic is exact and total (musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]
// Generators use expect() on in-bounds constructions: a failure is a bug in
// the generator, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_kernel::{Beat, Occurrence, Span, Timeline, overlay, sequence, timeline};
use num_rational::Ratio;
use proptest::prelude::*;

const QUARTER: i64 = 4;

fn quarters(q: i64) -> Beat {
    Beat::new(Ratio::new(q, QUARTER))
}

fn arb_occurrence(extent_quarters: i64) -> impl Strategy<Value = Occurrence<u8>> {
    (0..=extent_quarters).prop_flat_map(move |start| {
        (start..=extent_quarters, 0u8..8).prop_map(move |(end, payload)| {
            let span = Span::new(quarters(start), quarters(end)).expect("ordered");
            Occurrence::new(span, payload)
        })
    })
}

fn arb_timeline_at(extent_quarters: i64) -> impl Strategy<Value = Timeline<u8>> {
    prop::collection::vec(arb_occurrence(extent_quarters), 0..5)
        .prop_map(move |occurrences| timeline(quarters(extent_quarters), occurrences).expect("in bounds"))
}

fn arb_timeline() -> impl Strategy<Value = Timeline<u8>> {
    (0i64..=16).prop_flat_map(arb_timeline_at)
}

/// Two timelines of independently generated content but equal extents — the
/// synchronization precondition of L18.
fn arb_synchronized_pair() -> impl Strategy<Value = (Timeline<u8>, Timeline<u8>)> {
    (0i64..=12).prop_flat_map(|extent| (arb_timeline_at(extent), arb_timeline_at(extent)))
}

fn arb_window(extent_quarters: i64) -> impl Strategy<Value = (i64, i64)> {
    (0..=extent_quarters).prop_flat_map(move |start| (Just(start), start..=extent_quarters))
}

/// A timeline with two independent observation windows — *not* nested, since
/// narrowing intersects and L17 is therefore claimed for all windows.
fn arb_timeline_with_two_windows() -> impl Strategy<Value = (Timeline<u8>, (i64, i64), (i64, i64))> {
    (0i64..=16).prop_flat_map(|extent| (arb_timeline_at(extent), arb_window(extent), arb_window(extent)))
}

/// Each observation as `(whole span, visible span)`, the pair D6 reports.
fn observed_pairs<A>(observation: &musa_kernel::Observation<'_, A>) -> Vec<(Span, Span)> {
    observation
        .observed()
        .map(|(visible, occurrence)| (occurrence.span(), visible))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// L1: (M ; N) ; P = M ; (N ; P).
    #[test]
    fn seq_associativity(m in arb_timeline(), n in arb_timeline(), p in arb_timeline()) {
        let left = sequence(vec![sequence(vec![m.clone(), n.clone()]), p.clone()]);
        let right = sequence(vec![m, sequence(vec![n, p])]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L2: 0 ; M = M = M ; 0 with 0 = (0, ∅).
    #[test]
    fn seq_zero_identity(m in arb_timeline()) {
        let zero = timeline(Beat::ZERO, Vec::<Occurrence<u8>>::new()).expect("empty");
        prop_assert!(sequence(vec![zero.clone(), m.clone()]).semantic_eq(&m));
        prop_assert!(sequence(vec![m.clone(), zero]).semantic_eq(&m));
    }

    /// L3: duration(M ; N) = duration(M) + duration(N).
    #[test]
    fn seq_duration_additivity(m in arb_timeline(), n in arb_timeline()) {
        let joined = sequence(vec![m.clone(), n.clone()]);
        prop_assert_eq!(
            joined.extent().as_ratio(),
            m.extent().as_ratio() + n.extent().as_ratio()
        );
    }

    /// L4: (M ⊕ N) ⊕ P = M ⊕ (N ⊕ P).
    #[test]
    fn overlay_associativity(m in arb_timeline(), n in arb_timeline(), p in arb_timeline()) {
        let left = overlay(vec![overlay(vec![m.clone(), n.clone()]), p.clone()]);
        let right = overlay(vec![m, overlay(vec![n, p])]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L5: M ⊕ N = N ⊕ M.
    #[test]
    fn overlay_commutativity(m in arb_timeline(), n in arb_timeline()) {
        prop_assert!(overlay(vec![m.clone(), n.clone()]).semantic_eq(&overlay(vec![n, m])));
    }

    /// L6: at fixed extent d, (d, ∅) is the overlay identity.
    #[test]
    fn overlay_fixed_duration_identity(m in arb_timeline()) {
        let empty = timeline(m.extent(), Vec::<Occurrence<u8>>::new()).expect("empty");
        prop_assert!(overlay(vec![m.clone(), empty.clone()]).semantic_eq(&m));
        prop_assert!(overlay(vec![empty, m.clone()]).semantic_eq(&m));
    }

    /// L9: Timeline(id) = id.
    #[test]
    fn map_identity(m in arb_timeline()) {
        prop_assert!(m.map_payload(|p| *p).semantic_eq(&m));
    }

    /// L10: Timeline(g ∘ f) = Timeline(g) ∘ Timeline(f).
    #[test]
    fn map_composition(m in arb_timeline()) {
        let f = |p: &u8| format!("n{p}");
        let g = |s: &String| format!("[{s}]");
        let composed = m.map_payload(|p| g(&f(p)));
        let stepwise = m.map_payload(f).map_payload(g);
        prop_assert!(composed.semantic_eq(&stepwise));
    }

    /// L11: Timeline(f) preserves sequence.
    #[test]
    fn map_preserves_sequence(m in arb_timeline(), n in arb_timeline()) {
        let f = |p: &u8| p.saturating_add(10);
        let left = sequence(vec![m.clone(), n.clone()]).map_payload(f);
        let right = sequence(vec![m.map_payload(f), n.map_payload(f)]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L12: Timeline(f) preserves overlay.
    #[test]
    fn map_preserves_overlay(m in arb_timeline(), n in arb_timeline()) {
        let f = |p: &u8| p.saturating_add(10);
        let left = overlay(vec![m.clone(), n.clone()]).map_payload(f);
        let right = overlay(vec![m.map_payload(f), n.map_payload(f)]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L13a: scale_1 = id.
    #[test]
    fn scale_identity(m in arb_timeline()) {
        let same = m.scale(Ratio::from_integer(1)).expect("positive");
        prop_assert!(same.semantic_eq(&m));
    }

    /// L13b: scale_r ∘ scale_s = scale_{r·s}.
    #[test]
    fn scale_composition(m in arb_timeline(), r in 1i64..=4, s in 1i64..=4, sd in 1i64..=4) {
        let r_ratio = Ratio::new(r, 1);
        let s_ratio = Ratio::new(s, sd);
        let stepwise = m
            .scale(s_ratio)
            .and_then(|scaled| scaled.scale(r_ratio))
            .expect("positive");
        let direct = m.scale(r_ratio * s_ratio).expect("positive");
        prop_assert!(stepwise.semantic_eq(&direct));
    }

    /// L14: scale preserves sequence.
    #[test]
    fn scale_preserves_sequence(m in arb_timeline(), n in arb_timeline(), r in 1i64..=4) {
        let factor = Ratio::new(r, 2);
        let left = sequence(vec![m.clone(), n.clone()]).scale(factor).expect("positive");
        let right = sequence(vec![
            m.scale(factor).expect("positive"),
            n.scale(factor).expect("positive"),
        ]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L15: scale preserves overlay.
    #[test]
    fn scale_preserves_overlay(m in arb_timeline(), n in arb_timeline(), r in 1i64..=4) {
        let factor = Ratio::new(r, 2);
        let left = overlay(vec![m.clone(), n.clone()]).scale(factor).expect("positive");
        let right = overlay(vec![
            m.scale(factor).expect("positive"),
            n.scale(factor).expect("positive"),
        ]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L16: restrict at the full extent is the identity on observations —
    /// every occurrence is reported, and its visible span is its whole span.
    #[test]
    fn restrict_identity(m in arb_timeline()) {
        let full = Span::new(Beat::ZERO, m.extent()).expect("ordered");
        let observed = observed_pairs(&m.restrict(full));
        let whole: Vec<(Span, Span)> = m
            .occurrences()
            .iter()
            .map(|occurrence| (occurrence.span(), occurrence.span()))
            .collect();
        prop_assert_eq!(observed, whole);
    }

    /// L17: restrict_K ∘ restrict_J = restrict_{J ∩ K}, for *any* two windows.
    /// Narrowing intersects, so the law needs no nesting precondition; windows
    /// that do not meet observe nothing.
    #[test]
    fn restrict_composition(mjk in arb_timeline_with_two_windows()) {
        let (m, (j0, j1), (k0, k1)) = mjk;
        let j = Span::new(quarters(j0), quarters(j1)).expect("ordered");
        let k = Span::new(quarters(k0), quarters(k1)).expect("ordered");
        let composed = m.restrict(j).restrict(k);
        if j0.max(k0) <= j1.min(k1) {
            let meet = Span::new(quarters(j0.max(k0)), quarters(j1.min(k1))).expect("ordered");
            prop_assert_eq!(observed_pairs(&composed), observed_pairs(&m.restrict(meet)));
        } else {
            prop_assert!(composed.is_empty());
        }
    }

    /// L18: (M ⊕ N) ; (P ⊕ Q) = (M ; P) ⊕ (N ; Q) under synchronization.
    #[test]
    fn synchronized_interchange(
        mn in arb_synchronized_pair(),
        pq in arb_synchronized_pair(),
    ) {
        let (m, n) = mn;
        let (p, q) = pq;
        let left = sequence(vec![overlay(vec![m.clone(), n.clone()]), overlay(vec![p.clone(), q.clone()])]);
        let right = overlay(vec![sequence(vec![m, p]), sequence(vec![n, q])]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// X1: M ⊕ M ≠ M when M has occurrences — multiplicity doubles (§8).
    #[test]
    fn overlay_not_idempotent(m in arb_timeline()) {
        prop_assume!(!m.occurrences().is_empty());
        let doubled = overlay(vec![m.clone(), m.clone()]);
        prop_assert_eq!(doubled.occurrences().len(), 2 * m.occurrences().len());
        prop_assert!(!doubled.semantic_eq(&m));
    }

    /// L19: semantic equality is a congruence — swapping an argument for a
    /// semantically equal one preserves results.
    #[test]
    fn semantic_equality_is_congruence(m in arb_timeline(), n in arb_timeline()) {
        let shuffled: Vec<Occurrence<u8>> = m.occurrences().iter().rev().cloned().collect();
        let rebuilt = timeline(m.extent(), shuffled).expect("same spans");
        prop_assert!(m.semantic_eq(&rebuilt));
        prop_assert!(sequence(vec![m.clone(), n.clone()]).semantic_eq(&sequence(vec![rebuilt.clone(), n.clone()])));
        prop_assert!(overlay(vec![m, n.clone()]).semantic_eq(&overlay(vec![rebuilt, n])));
    }
}

/// X2: sequence does not distribute over overlay (§10). The left side has one
/// copy of M's occurrence; the right side has two.
#[test]
fn sequence_does_not_distribute_over_overlay() {
    let m = timeline(quarters(4), vec![occurrence_at(0, 4, 1)]).expect("valid");
    let n = timeline(quarters(4), vec![occurrence_at(0, 4, 2)]).expect("valid");
    let p = timeline(quarters(4), vec![occurrence_at(0, 4, 3)]).expect("valid");
    let left = sequence(vec![m.clone(), overlay(vec![n.clone(), p.clone()])]);
    let right = overlay(vec![sequence(vec![m.clone(), n]), sequence(vec![m, p])]);
    assert!(!left.semantic_eq(&right));
}

/// L18 fails without the synchronization preconditions.
#[test]
fn interchange_fails_without_synchronization() {
    // duration(M) = 1 beat ≠ duration(N) = 2 beats.
    let m = timeline(quarters(4), vec![occurrence_at(0, 4, 1)]).expect("valid");
    let n = timeline(quarters(8), vec![occurrence_at(0, 8, 2)]).expect("valid");
    let p = timeline(quarters(4), vec![occurrence_at(0, 4, 3)]).expect("valid");
    let q = timeline(quarters(4), vec![occurrence_at(0, 4, 4)]).expect("valid");
    let left = sequence(vec![
        overlay(vec![m.clone(), n.clone()]),
        overlay(vec![p.clone(), q.clone()]),
    ]);
    let right = overlay(vec![sequence(vec![m, p]), sequence(vec![n, q])]);
    assert!(!left.semantic_eq(&right));
}

/// L17 with a strictly nested window K ⊂ J ⊂ I: the case the law was first
/// stated for, kept as a worked example alongside the general property.
#[test]
fn restrict_composition_strictly_nested() {
    let m = timeline(
        quarters(16),
        vec![
            occurrence_at(1, 6, 1),
            occurrence_at(5, 12, 2),
            occurrence_at(10, 15, 3),
        ],
    )
    .expect("valid");
    let j = Span::new(quarters(2), quarters(14)).expect("ordered");
    let k = Span::new(quarters(4), quarters(10)).expect("ordered");
    assert_eq!(
        observed_pairs(&m.restrict(j).restrict(k)),
        observed_pairs(&m.restrict(k))
    );
}

fn occurrence_at(start: i64, end: i64, payload: u8) -> Occurrence<u8> {
    let span = Span::new(quarters(start), quarters(end)).expect("ordered");
    Occurrence::new(span, payload)
}
