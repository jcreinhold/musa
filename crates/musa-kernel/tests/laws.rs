//! The temporal-kernel law suite (docs/rules/kernel/04-algebraic-laws.md). Every
//! test name matches the law name in the spec; the non-laws X1–X2 are tested
//! as counterexamples. Equality is semantic equality (N4) throughout.

// Rational test arithmetic is exact and total (musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]
// Generators use expect() on in-bounds constructions: a failure is a bug in
// the generator, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_kernel::{Beat, Canonical as _, Occurrence, Progress, Span, Timeline, overlay, sequence, timeline};
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

/// A payload intentionally unlike the scalar used by the original law suite.
/// It exercises variable text, an exact rational, and a local-time curve.
#[derive(Clone, Debug, PartialEq, Eq)]
struct AdmissionProbe {
    label: String,
    ratio: Ratio<i64>,
    shape: Progress,
}

impl musa_kernel::Canonical for AdmissionProbe {
    const OWNER_TYPE_ID: &'static str = "musa.kernel.tests.AdmissionProbe";
    const QUOTIENT_VERSION: u32 = 1;

    fn canonical_key(&self) -> String {
        format!(
            "{}:{}|{}/{}|{}",
            self.label.len(),
            self.label,
            self.ratio.numer(),
            self.ratio.denom(),
            self.shape.canonical_key()
        )
    }
}

fn probe(value: u8) -> AdmissionProbe {
    AdmissionProbe {
        label: format!("probe\n{value}; from to"),
        ratio: Ratio::new(i64::from(value) + 1, 9),
        shape: Progress::piecewise([
            (Ratio::ZERO, Ratio::ZERO),
            (Ratio::new(1, 2), Ratio::new(i64::from(value), 8)),
            (Ratio::ONE, Ratio::ONE),
        ])
        .expect("ordered fixed probe"),
    }
}

fn probe_timeline(source: &Timeline<u8>) -> Timeline<AdmissionProbe> {
    source.map_payload(|value| probe(*value))
}

fn probe_observed(observation: &musa_kernel::Observation<'_, AdmissionProbe>) -> Vec<(Span, Span, String)> {
    observation
        .observed()
        .map(|(visible, occurrence)| (occurrence.span(), visible, occurrence.payload().canonical_key()))
        .collect()
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

    /// L20: coverage is observation, taken as narrowly as possible. An
    /// occurrence covers `t` exactly when every window containing `t`
    /// observes it — the two ways of asking cannot disagree.
    #[test]
    fn coverage_agrees_with_observation(m in arb_timeline(), at in 0i64..16) {
        let at = quarters(at);
        if at > m.extent() {
            return Ok(());
        }
        let covering: Vec<Span> = m.covering(at).map(Occurrence::span).collect();
        // Every window containing `at`, out of a family that includes the
        // tightest ones on both sides.
        for width in [1i64, 2, 4] {
            let start = Beat::new(at.as_ratio() - quarters(width).as_ratio());
            let start = if start.as_ratio() < num_rational::Ratio::ZERO { Beat::ZERO } else { start };
            let end = Beat::new(at.as_ratio() + quarters(width).as_ratio());
            let window = Span::new(start, end).expect("ordered");
            let observed: Vec<Span> = m.restrict(window).observed().map(|(_, o)| o.span()).collect();
            for span in &covering {
                prop_assert!(observed.contains(span), "a covering occurrence must be observed through {window}");
            }
        }
        // And nothing else covers `at`. Every occurrence that contains `at`
        // is visible through the tightest window starting there, so filtering
        // that observation by containment must reproduce `covering` exactly —
        // as a multiset, since equal occurrences are distinct facts (K6).
        let tight = Span::new(at, Beat::new(at.as_ratio() + quarters(1).as_ratio())).expect("ordered");
        let mut observed: Vec<(Span, u8)> = m
            .restrict(tight)
            .observed()
            .map(|(_, occurrence)| (occurrence.span(), *occurrence.payload()))
            .filter(|(span, _)| span.contains(at))
            .collect();
        let mut covered: Vec<(Span, u8)> = m
            .covering(at)
            .map(|occurrence| (occurrence.span(), *occurrence.payload()))
            .collect();
        observed.sort_by_key(|(span, payload)| (span.start(), span.end(), *payload));
        covered.sort_by_key(|(span, payload)| (span.start(), span.end(), *payload));
        prop_assert_eq!(observed, covered);
    }

    /// L21: coverage commutes with the algebra. Scaling moves the question
    /// with the music, and sequencing moves it by the first extent.
    #[test]
    fn coverage_is_stable_under_time_transformation(m in arb_timeline(), n in arb_timeline(), at in 1i64..12) {
        let at = quarters(at);
        if at > m.extent() {
            return Ok(());
        }
        let factor = num_rational::Ratio::new(3, 2);
        let scaled = m.scale(factor).expect("positive factor");
        let here: Vec<&u8> = m.covering(at).map(Occurrence::payload).collect();
        let there: Vec<&u8> = scaled.covering(Beat::new(at.as_ratio() * factor)).map(Occurrence::payload).collect();
        prop_assert_eq!(here, there);

        // Past the seam, a sequence answers with its second argument alone.
        // *At* the seam both may answer — a point at `m`'s extent and `n`'s
        // material at 0 share that instant — which is D2's boundary, not a
        // defect, so the law is stated strictly past it.
        let joined = sequence(vec![m.clone(), n.clone()]);
        let after = Beat::new(m.extent().as_ratio() + at.as_ratio());
        if at <= n.extent() {
            let inside: Vec<&u8> = n.covering(at).map(Occurrence::payload).collect();
            let outside: Vec<&u8> = joined.covering(after).map(Occurrence::payload).collect();
            prop_assert_eq!(inside, outside);
        }
    }

    /// L22: prevailing is the last selected start. Stated against an
    /// independent scan, so the law does not check the implementation
    /// against itself.
    #[test]
    fn prevailing_is_the_last_selected_start(m in arb_timeline(), at in 0i64..16) {
        let at = quarters(at);
        let expected = m
            .canonical_occurrences()
            .into_iter()
            .rfind(|occurrence| occurrence.span().start() <= at && occurrence.payload().is_multiple_of(2))
            .map(|occurrence| *occurrence.payload());
        let actual = m.prevailing(at, |payload: &u8| payload.is_multiple_of(2).then_some(*payload));
        prop_assert_eq!(actual, expected);
    }

    /// L23: prevailing is monotone in information. Facts that start after
    /// `at` cannot change what is in force at `at`, which is what makes it
    /// safe to build a piece a voice at a time.
    #[test]
    fn prevailing_ignores_facts_that_start_later(m in arb_timeline(), at in 0i64..8) {
        let at = quarters(at);
        let select = |payload: &u8| payload.is_multiple_of(2).then_some(*payload);
        let before = m.prevailing(at, select);
        // Everything in `later` starts strictly after `at`.
        let start = Beat::new(at.as_ratio() + quarters(1).as_ratio());
        let end = Beat::new(start.as_ratio() + quarters(2).as_ratio());
        let span = Span::new(start, end).expect("ordered");
        let later = timeline(end, vec![Occurrence::new(span, 2u8), Occurrence::new(span, 4u8)]).expect("in bounds");
        prop_assert_eq!(overlay(vec![m, later]).prevailing(at, select), before);
    }

    /// N6: the semantic hash agrees with semantic equality. Equal meaning,
    /// equal digest — the direction a caller relies on when an unequal digest
    /// makes it rebuild something.
    #[test]
    fn semantic_equality_implies_equal_hashes(m in arb_timeline(), n in arb_timeline()) {
        let shuffled: Vec<Occurrence<u8>> = m.occurrences().iter().rev().cloned().collect();
        let rebuilt = timeline(m.extent(), shuffled).expect("same spans");
        prop_assert!(m.semantic_eq(&rebuilt));
        prop_assert_eq!(m.semantic_hash(), rebuilt.semantic_hash());
        prop_assert_eq!(
            sequence(vec![m.clone(), n.clone()]).semantic_hash(),
            sequence(vec![rebuilt.clone(), n.clone()]).semantic_hash()
        );
        prop_assert_eq!(
            overlay(vec![m.clone(), n.clone()]).semantic_hash(),
            overlay(vec![rebuilt, n]).semantic_hash()
        );
        // And the contrapositive is what makes the digest worth computing:
        // a timeline that says something else digests differently.
        let stretched = Beat::new(m.extent().as_ratio() + quarters(1).as_ratio());
        let longer = timeline(stretched, m.occurrences().to_vec()).expect("wider extent still contains them");
        prop_assert_ne!(m.semantic_hash(), longer.semantic_hash());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// L1–L24 and N6 keep the same statements for an admitted structured
    /// payload. This is the second generic instantiation required by the
    /// payload-admission rule; no theory-specific premise is added.
    #[test]
    fn admitted_structured_payload_transports_the_temporal_laws(
        raw_m in arb_timeline(),
        raw_n in arb_timeline(),
        raw_p in arb_timeline(),
        synchronized_mn in arb_synchronized_pair(),
        synchronized_pq in arb_synchronized_pair(),
        windows in arb_timeline_with_two_windows(),
        at_quarters in 0i64..=12,
    ) {
        let m = probe_timeline(&raw_m);
        let n = probe_timeline(&raw_n);
        let p = probe_timeline(&raw_p);

        // L1–L6 and X1.
        prop_assert!(sequence(vec![sequence(vec![m.clone(), n.clone()]), p.clone()])
            .semantic_eq(&sequence(vec![m.clone(), sequence(vec![n.clone(), p.clone()])] )));
        let zero = timeline(Beat::ZERO, Vec::<Occurrence<AdmissionProbe>>::new()).expect("empty");
        prop_assert!(sequence(vec![zero.clone(), m.clone()]).semantic_eq(&m));
        prop_assert!(sequence(vec![m.clone(), zero]).semantic_eq(&m));
        prop_assert_eq!(
            sequence(vec![m.clone(), n.clone()]).extent().as_ratio(),
            m.extent().as_ratio() + n.extent().as_ratio()
        );
        prop_assert!(overlay(vec![overlay(vec![m.clone(), n.clone()]), p.clone()])
            .semantic_eq(&overlay(vec![m.clone(), overlay(vec![n.clone(), p])] )));
        prop_assert!(overlay(vec![m.clone(), n.clone()]).semantic_eq(&overlay(vec![n.clone(), m.clone()])));
        let empty = timeline(m.extent(), Vec::<Occurrence<AdmissionProbe>>::new()).expect("empty");
        prop_assert!(overlay(vec![m.clone(), empty]).semantic_eq(&m));
        if !m.occurrences().is_empty() {
            prop_assert!(!overlay(vec![m.clone(), m.clone()]).semantic_eq(&m));
        }

        // L9–L12.
        prop_assert!(m.map_payload(Clone::clone).semantic_eq(&m));
        let f = |value: &AdmissionProbe| format!("{}:{}", value.label, value.ratio);
        let g = |value: &String| format!("[{value}]");
        prop_assert!(m.map_payload(|value| g(&f(value))).semantic_eq(&m.map_payload(f).map_payload(g)));
        let rename = |value: &AdmissionProbe| format!("{}:{}", value.label, value.shape.canonical_key());
        prop_assert!(sequence(vec![m.clone(), n.clone()]).map_payload(rename)
            .semantic_eq(&sequence(vec![m.map_payload(rename), n.map_payload(rename)])));
        prop_assert!(overlay(vec![m.clone(), n.clone()]).map_payload(rename)
            .semantic_eq(&overlay(vec![m.map_payload(rename), n.map_payload(rename)])));

        // L13–L15.
        prop_assert!(m.scale(Ratio::ONE).expect("positive").semantic_eq(&m));
        let r = Ratio::new(3, 2);
        let s = Ratio::new(5, 4);
        prop_assert!(m.scale(s).and_then(|scaled| scaled.scale(r)).expect("positive")
            .semantic_eq(&m.scale(r * s).expect("positive")));
        prop_assert!(sequence(vec![m.clone(), n.clone()]).scale(r).expect("positive")
            .semantic_eq(&sequence(vec![m.scale(r).expect("positive"), n.scale(r).expect("positive")])));
        prop_assert!(overlay(vec![m.clone(), n.clone()]).scale(r).expect("positive")
            .semantic_eq(&overlay(vec![m.scale(r).expect("positive"), n.scale(r).expect("positive")])));

        // L16–L17.
        let full = Span::new(Beat::ZERO, m.extent()).expect("ordered");
        let observed = probe_observed(&m.restrict(full));
        let whole: Vec<(Span, Span, String)> = m.occurrences().iter().map(|occurrence| {
            (occurrence.span(), occurrence.span(), occurrence.payload().canonical_key())
        }).collect();
        prop_assert_eq!(observed, whole);
        let (window_source, (j0, j1), (k0, k1)) = windows;
        let window_source = probe_timeline(&window_source);
        let j = Span::new(quarters(j0), quarters(j1)).expect("ordered");
        let k = Span::new(quarters(k0), quarters(k1)).expect("ordered");
        let composed = window_source.restrict(j).restrict(k);
        if j0.max(k0) <= j1.min(k1) {
            let meet = Span::new(quarters(j0.max(k0)), quarters(j1.min(k1))).expect("ordered");
            prop_assert_eq!(probe_observed(&composed), probe_observed(&window_source.restrict(meet)));
        } else {
            prop_assert!(composed.is_empty());
        }

        // L18–L19.
        let (sm, sn) = synchronized_mn;
        let (sp, sq) = synchronized_pq;
        let (sm, sn, sp, sq) = (
            probe_timeline(&sm), probe_timeline(&sn), probe_timeline(&sp), probe_timeline(&sq),
        );
        prop_assert!(sequence(vec![overlay(vec![sm.clone(), sn.clone()]), overlay(vec![sp.clone(), sq.clone()])])
            .semantic_eq(&overlay(vec![sequence(vec![sm, sp]), sequence(vec![sn, sq])])));
        let reversed = timeline(
            m.extent(),
            m.occurrences().iter().rev().cloned().collect(),
        ).expect("same bounds");
        prop_assert!(m.semantic_eq(&reversed));
        prop_assert!(overlay(vec![m.clone(), n.clone()]).semantic_eq(&overlay(vec![reversed.clone(), n])));

        // L20–L23.
        let at = quarters(at_quarters);
        if at <= m.extent() {
            let mut covered: Vec<(Span, String)> = m.covering(at)
                .map(|occurrence| (occurrence.span(), occurrence.payload().canonical_key()))
                .collect();
            let tight_end = Beat::new((at.as_ratio() + quarters(1).as_ratio()).min(m.extent().as_ratio()));
            if tight_end > at {
                let tight = Span::new(at, tight_end).expect("ordered");
                let mut observed: Vec<(Span, String)> = m.restrict(tight).observed()
                    .map(|(_, occurrence)| (occurrence.span(), occurrence.payload().canonical_key()))
                    .filter(|(span, _)| span.contains(at))
                    .collect();
                covered.sort_by_key(|(span, key)| (span.start(), span.end(), key.clone()));
                observed.sort_by_key(|(span, key)| (span.start(), span.end(), key.clone()));
                prop_assert_eq!(covered, observed);
            }
            let scaled = m.scale(r).expect("positive");
            let here: Vec<String> = m.covering(at).map(|o| o.payload().canonical_key()).collect();
            let there: Vec<String> = scaled.covering(Beat::new(at.as_ratio() * r))
                .map(|o| o.payload().canonical_key()).collect();
            prop_assert_eq!(here, there);
        }
        let select = |value: &AdmissionProbe| (value.ratio.numer() % 2 == 0).then_some(value.label.clone());
        let expected = m.canonical_occurrences().into_iter()
            .rfind(|o| o.span().start() <= at && o.payload().ratio.numer() % 2 == 0)
            .map(|o| o.payload().label.clone());
        prop_assert_eq!(m.prevailing(at, select), expected);
        let later_start = Beat::new(at.as_ratio() + quarters(1).as_ratio());
        let later_end = Beat::new(later_start.as_ratio() + quarters(1).as_ratio());
        let later = timeline(later_end, vec![Occurrence::new(
            Span::new(later_start, later_end).expect("ordered"), probe(1),
        )]).expect("in bounds");
        prop_assert_eq!(overlay(vec![m.clone(), later]).prevailing(at, select), m.prevailing(at, select));

        // L24 and N6.
        let payload_keys: Vec<String> = m.occurrences().iter().map(|o| o.payload().canonical_key()).collect();
        let scaled_keys: Vec<String> = m.scale(r).expect("positive").occurrences().iter()
            .map(|o| o.payload().canonical_key()).collect();
        prop_assert_eq!(payload_keys, scaled_keys);
        prop_assert_eq!(m.semantic_hash(), reversed.semantic_hash());
    }
}

/// N6: the digest is over exact framed semantic bytes, independently rebuilt
/// here so a version/tag/field-order drift fails loudly.
#[test]
fn the_digest_is_the_framed_semantic_encoding_and_nothing_else() {
    let m = timeline(quarters(8), vec![occurrence_at(0, 4, 1), occurrence_at(4, 8, 2)]).expect("valid");
    let bytes = independently_frame_u8_timeline(&m);
    assert_eq!(m.semantic_hash().to_string(), fnv1a_128(&bytes));
    // Fixed here so a change of algorithm, offset basis, or byte order fails
    // loudly rather than silently invalidating every stored identity.
    assert_eq!(m.semantic_hash().to_string(), "48295d3fdbf5c741a806ce38516db1d9");
}

fn independently_frame_u8_timeline(timeline: &Timeline<u8>) -> Vec<u8> {
    fn bytes(out: &mut Vec<u8>, value: &[u8]) {
        out.extend_from_slice(&(value.len() as u64).to_be_bytes());
        out.extend_from_slice(value);
    }
    fn rational(out: &mut Vec<u8>, value: Beat) {
        let ratio = value.as_ratio();
        out.extend_from_slice(&ratio.numer().to_be_bytes());
        out.extend_from_slice(&ratio.denom().to_be_bytes());
    }

    let mut out = Vec::new();
    bytes(&mut out, b"musa.timeline.semantic");
    out.extend_from_slice(&2u32.to_be_bytes());
    bytes(&mut out, b"musa.kernel.u8");
    out.extend_from_slice(&1u32.to_be_bytes());
    rational(&mut out, timeline.extent());
    out.extend_from_slice(&(timeline.occurrences().len() as u64).to_be_bytes());
    for occurrence in timeline.canonical_occurrences() {
        rational(&mut out, occurrence.span().start());
        rational(&mut out, occurrence.span().end());
        bytes(&mut out, occurrence.payload().canonical_key().as_bytes());
    }
    out
}

#[test]
fn framed_identity_separates_the_old_display_collision() {
    let one = timeline(
        quarters(8),
        vec![Occurrence::new(
            Span::new(quarters(4), quarters(8)).expect("ordered"),
            "a from 0 to 1;\n  occurrence b".to_owned(),
        )],
    )
    .expect("in bounds");
    let two = timeline(
        quarters(8),
        vec![
            Occurrence::new(Span::new(quarters(0), quarters(4)).expect("ordered"), "a".to_owned()),
            Occurrence::new(Span::new(quarters(4), quarters(8)).expect("ordered"), "b".to_owned()),
        ],
    )
    .expect("in bounds");

    assert_eq!(
        one.to_string(),
        two.to_string(),
        "this is the verified old N5 collision"
    );
    assert!(!one.semantic_eq(&two));
    assert_ne!(one.semantic_hash(), two.semantic_hash());
}

#[derive(Clone)]
struct SameKeyOtherSchema;

impl musa_kernel::Canonical for SameKeyOtherSchema {
    const OWNER_TYPE_ID: &'static str = "musa.kernel.tests.OtherSchema";
    const QUOTIENT_VERSION: u32 = 7;

    fn canonical_key(&self) -> String {
        "1".to_owned()
    }
}

#[test]
fn framed_identity_covers_schema_and_multiplicity() {
    let scalar = timeline(quarters(4), vec![occurrence_at(0, 4, 1)]).expect("valid");
    let other = timeline(
        quarters(4),
        vec![Occurrence::new(
            Span::new(quarters(0), quarters(4)).expect("ordered"),
            SameKeyOtherSchema,
        )],
    )
    .expect("valid");
    let doubled = overlay(vec![scalar.clone(), scalar.clone()]);

    assert_ne!(scalar.semantic_hash(), other.semantic_hash());
    assert_ne!(scalar.semantic_hash(), doubled.semantic_hash());
}

proptest! {
    #[test]
    fn arbitrary_payload_delimiters_preserve_equality_hash_agreement(
        extent in 0i64..=16,
        payloads in prop::collection::vec(any::<String>(), 0..8),
    ) {
        let extent = quarters(extent);
        let occurrences: Vec<Occurrence<String>> = payloads.into_iter().map(|payload| {
            Occurrence::new(Span::new(Beat::ZERO, extent).expect("ordered"), payload)
        }).collect();
        let value = timeline(extent, occurrences).expect("in bounds");
        let shuffled = timeline(extent, value.occurrences().iter().rev().cloned().collect()).expect("same bounds");
        prop_assert!(value.semantic_eq(&shuffled));
        prop_assert_eq!(value.semantic_hash(), shuffled.semantic_hash());
    }
}

/// The published FNV-1a 128 parameters, written out independently of the
/// kernel's implementation so the test can disagree with it.
fn fnv1a_128(bytes: &[u8]) -> String {
    let mut state: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for byte in bytes {
        state ^= u128::from(*byte);
        state = state.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b);
    }
    format!("{state:032x}")
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

/// L24 — a curve-bearing occurrence transforms by its span alone.
///
/// `Progress` is indexed by normalized *local* time, so every kernel operation
/// moves or stretches the span and leaves the payload bytes untouched. This is
/// what makes a continuous shape a payload value rather than a kernel
/// operation (docs/rules/kernel/03 `Progress`, §32 Q4): if the curve were in
/// absolute time, `scale` and `sequence` would have to rewrite it, and the
/// kernel would be looking inside payloads (§12).
///
/// The test checks both halves: the bytes are identical after each operation,
/// and `at(u)` at corresponding *absolute* times agrees before and after —
/// which is the part that would fail for an absolute-time curve even if the
/// bytes happened to survive.
#[test]
fn a_curve_bearing_occurrence_transforms_by_its_span_alone() {
    let curve = Progress::piecewise([
        (Ratio::new(0, 1), Ratio::new(0, 1)),
        (Ratio::new(1, 2), Ratio::new(1, 4)),
        (Ratio::new(1, 1), Ratio::new(1, 1)),
    ])
    .expect("well formed");
    let key = curve.canonical_key();
    let span = Span::new(quarters(4), quarters(12)).expect("ordered");
    let m = timeline(quarters(16), vec![Occurrence::new(span, curve.clone())]).expect("in bounds");

    // A probe at the absolute instant one quarter of the way through the
    // occurrence: u = 1/4 before and after every operation.
    let u = Ratio::new(1, 4);
    let expected = curve.at(u);

    let scaled = m.scale(Ratio::new(3, 1)).expect("positive");
    let delayed = sequence(vec![timeline(quarters(8), vec![]).expect("empty"), m.clone()]);
    let stacked = overlay(vec![m.clone(), timeline(quarters(16), vec![]).expect("empty")]);
    let observed = m.restrict(Span::new(Beat::ZERO, quarters(16)).expect("ordered"));

    for (name, moved) in [("scale", &scaled), ("sequence", &delayed), ("overlay", &stacked)] {
        let occurrence = moved.occurrences().first().expect("one occurrence");
        assert_eq!(occurrence.payload().canonical_key(), key, "{name} rewrote the payload");
        let local = occurrence.span();
        let width = local.end().as_ratio() - local.start().as_ratio();
        let probe = local.start().as_ratio() + width * u;
        let recovered = (probe - local.start().as_ratio()) / width;
        assert_eq!(occurrence.payload().at(recovered), expected, "{name} moved the curve");
    }
    let (_, seen) = observed.observed().next().expect("observable");
    assert_eq!(seen.payload().canonical_key(), key, "restrict rewrote the payload");
}
