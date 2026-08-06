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

/// A timeline with nested observation windows K ⊆ J ⊆ extent.
fn arb_timeline_with_nested_windows() -> impl Strategy<Value = (Timeline<u8>, (i64, i64), (i64, i64))> {
    (0i64..=16)
        .prop_flat_map(|extent| (arb_timeline_at(extent), arb_window(extent)))
        .prop_flat_map(|(m, j)| {
            (
                Just(m),
                Just(j),
                (j.0..=j.1).prop_flat_map(move |start| (Just(start), start..=j.1)),
            )
        })
}

/// Observations of `view` re-restricted to `k` (docs/kernel/04 L17): keep
/// what intersects `k`, re-clip the visible span, never move the whole span.
/// `extent` is the underlying timeline's extent, for the point-at-extent
/// boundary rule.
fn restrict_view<A>(view: &musa_kernel::RestrictedView<'_, A>, k: Span, extent: Beat) -> Vec<(Span, Span)> {
    view.observed()
        .iter()
        .filter(|observed| {
            let visible = observed.visible_span();
            let is_point = visible.start() == visible.end();
            let point_at_windows_extent_end = is_point && visible.start() == k.end();
            visible.visible_through(k) || (point_at_windows_extent_end && k.end() == extent)
        })
        .map(|observed| (observed.whole_span(), observed.visible_span().clip(k)))
        .collect()
}

fn observed_pairs<A>(view: &musa_kernel::RestrictedView<'_, A>) -> Vec<(Span, Span)> {
    view.observed()
        .iter()
        .map(|observed| (observed.whole_span(), observed.visible_span()))
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

    /// L7a: extend_{d,d} = id.
    #[test]
    fn extend_identity(m in arb_timeline()) {
        let same = m.extend(m.extent()).expect("equal extents");
        prop_assert!(same.semantic_eq(&m));
    }

    /// L7b: extend_{e,f} ∘ extend_{d,e} = extend_{d,f} for d ≤ e ≤ f.
    #[test]
    fn extend_composition(m in arb_timeline(), extra in 0i64..=8, more in 0i64..=8) {
        let d = m.extent().as_ratio();
        let e = d + Ratio::new(extra, QUARTER);
        let f = e + Ratio::new(more, QUARTER);
        let stepwise = m
            .extend(Beat::new(e))
            .and_then(|mid| mid.extend(Beat::new(f)))
            .expect("growing");
        let direct = m.extend(Beat::new(f)).expect("growing");
        prop_assert!(stepwise.semantic_eq(&direct));
    }

    /// L8: (extend_{d,e} M) ⊕ N = M ⊕ N when N has extent e ≥ d.
    #[test]
    fn overlay_respects_extension(m in arb_timeline(), n in arb_timeline()) {
        let e = m.extent().max(n.extent());
        let grown_m = m.extend(e).expect("growing");
        let grown_n = n.extend(e).expect("growing");
        let left = overlay(vec![grown_m, grown_n]);
        let right = overlay(vec![m, n]);
        prop_assert!(left.semantic_eq(&right));
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

    /// L16: restrict at the full extent is the identity on observations.
    #[test]
    fn restrict_identity(m in arb_timeline()) {
        let full = Span::new(Beat::ZERO, m.extent()).expect("ordered");
        let view = m.restrict(full);
        prop_assert_eq!(view.observed().len(), m.occurrences().len());
        for (observed, occurrence) in view.observed().iter().zip(m.occurrences()) {
            prop_assert_eq!(observed.whole_span(), occurrence.span());
            prop_assert_eq!(observed.visible_span(), occurrence.span());
        }
    }

    /// L17: restrict_K ∘ restrict_J = restrict_K for K ⊆ J ⊆ I, whole spans
    /// preserved.
    #[test]
    fn restrict_composition(mjw in arb_timeline_with_nested_windows()) {
        let (m, (j0, j1), (k0, k1)) = mjw;
        let j = Span::new(quarters(j0), quarters(j1)).expect("ordered");
        let k = Span::new(quarters(k0), quarters(k1)).expect("ordered");
        prop_assert_eq!(observed_pairs(&m.restrict(k)), restrict_view(&m.restrict(j), k, m.extent()));
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

/// L17 with a strictly nested window K ⊂ J ⊂ I.
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
    let direct = observed_pairs(&m.restrict(k));
    // Restrict the J-observation to K: keep observations whose visible part
    // intersects K, re-clip, keep whole spans.
    let via_j = restrict_view(&m.restrict(j), k, m.extent());
    assert_eq!(direct, via_j);
}

fn occurrence_at(start: i64, end: i64, payload: u8) -> Occurrence<u8> {
    let span = Span::new(quarters(start), quarters(end)).expect("ordered");
    Occurrence::new(span, payload)
}
