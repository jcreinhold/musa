//! The event-track law suite (docs/rules/events/04-algebraic-laws.md). Every
//! test name matches the law name in the spec; the non-laws X1–X2 are tested
//! as counterexamples. Equality is semantic equality (N4) throughout.
//!
//! Everything here is in one coordinate, because that is what a law is stated
//! in: `follow` and `together` take two tracks in the *same* coordinate, and
//! the cross-coordinate case is not a law that fails but a program that does
//! not compile. What *is* testable about the coordinate is what it does to
//! exact identity, and `the_coordinate_is_part_of_exact_identity` tests that.

// Rational test arithmetic is exact and total (musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]
// Generators use expect() on in-bounds constructions: a failure is a bug in
// the generator, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_events::{
    Canonical as _, Duration, EventTrack, Occurrence, PerformedTime, Position, Progress, Span, WrittenTime, empty,
    follow, together, track,
};
use num_rational::Ratio;
use proptest::prelude::*;

const QUARTER: i64 = 4;

/// The suite's track type. The coordinate is written out rather than inferred
/// so a reader sees which time these laws are about, and so that introducing a
/// second coordinate into a test is a visible edit rather than an accident.
type Track<A> = EventTrack<WrittenTime, A>;

/// An instant, in quarters of a beat.
fn quarters(q: i64) -> Position<WrittenTime> {
    Position::new(Ratio::new(q, QUARTER))
}

/// An amount of time, in quarters of a beat.
fn beats(q: i64) -> Duration<WrittenTime> {
    Duration::new(Ratio::new(q, QUARTER)).expect("nonnegative")
}

fn arb_occurrence(duration_quarters: i64) -> impl Strategy<Value = Occurrence<WrittenTime, u8>> {
    (0..=duration_quarters).prop_flat_map(move |start| {
        (start..=duration_quarters, 0u8..8).prop_map(move |(end, payload)| {
            let span = Span::new(quarters(start), quarters(end)).expect("ordered");
            Occurrence::new(span, payload)
        })
    })
}

fn arb_track_at(duration_quarters: i64) -> impl Strategy<Value = Track<u8>> {
    prop::collection::vec(arb_occurrence(duration_quarters), 0..5)
        .prop_map(move |occurrences| track(beats(duration_quarters), occurrences).expect("in bounds"))
}

fn arb_track() -> impl Strategy<Value = Track<u8>> {
    (0i64..=16).prop_flat_map(arb_track_at)
}

/// A payload intentionally unlike the scalar used by the original law suite.
/// It exercises variable text, an exact rational, and a local-time curve.
#[derive(Clone, Debug, PartialEq, Eq)]
struct AdmissionProbe {
    label: String,
    ratio: Ratio<i64>,
    shape: Progress,
}

impl musa_events::Canonical for AdmissionProbe {
    const OWNER_TYPE_ID: &'static str = "musa.events.tests.AdmissionProbe";
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

fn probe_track(source: &Track<u8>) -> Track<AdmissionProbe> {
    source.map_payloads(|value| probe(*value))
}

fn probe_observed(
    observation: &musa_events::Observation<'_, WrittenTime, AdmissionProbe>,
) -> Vec<(Span<WrittenTime>, Span<WrittenTime>, String)> {
    observation
        .observed()
        .map(|(visible, occurrence)| (occurrence.span(), visible, occurrence.payload().canonical_key()))
        .collect()
}

/// Two tracks of independently generated content but equal durations — the
/// synchronization precondition of L18.
fn arb_synchronized_pair() -> impl Strategy<Value = (Track<u8>, Track<u8>)> {
    (0i64..=12).prop_flat_map(|duration| (arb_track_at(duration), arb_track_at(duration)))
}

fn arb_window(duration_quarters: i64) -> impl Strategy<Value = (i64, i64)> {
    (0..=duration_quarters).prop_flat_map(move |start| (Just(start), start..=duration_quarters))
}

/// A track with two independent observation windows — *not* nested, since
/// narrowing intersects and L17 is therefore claimed for all windows.
fn arb_track_with_two_windows() -> impl Strategy<Value = (Track<u8>, (i64, i64), (i64, i64))> {
    (0i64..=16).prop_flat_map(|duration| (arb_track_at(duration), arb_window(duration), arb_window(duration)))
}

/// Each observation as `(whole span, visible span)`, the pair D6 reports.
fn observed_pairs<A>(
    observation: &musa_events::Observation<'_, WrittenTime, A>,
) -> Vec<(Span<WrittenTime>, Span<WrittenTime>)> {
    observation
        .observed()
        .map(|(visible, occurrence)| (occurrence.span(), visible))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// L1: (M ; N) ; P = M ; (N ; P).
    #[test]
    fn follow_associativity(m in arb_track(), n in arb_track(), p in arb_track()) {
        let left = follow(vec![follow(vec![m.clone(), n.clone()]), p.clone()]);
        let right = follow(vec![m, follow(vec![n, p])]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L2: 0 ; M = M = M ; 0 with 0 = (0, ∅).
    #[test]
    fn follow_zero_identity(m in arb_track()) {
        let zero = empty::<WrittenTime, u8>(Duration::ZERO);
        prop_assert!(follow(vec![zero.clone(), m.clone()]).semantic_eq(&m));
        prop_assert!(follow(vec![m.clone(), zero]).semantic_eq(&m));
    }

    /// L3: duration(M ; N) = duration(M) + duration(N).
    #[test]
    fn follow_duration_additivity(m in arb_track(), n in arb_track()) {
        let joined = follow(vec![m.clone(), n.clone()]);
        prop_assert_eq!(
            joined.duration().as_ratio(),
            m.duration().as_ratio() + n.duration().as_ratio()
        );
    }

    /// L4: (M ⊕ N) ⊕ P = M ⊕ (N ⊕ P).
    #[test]
    fn together_associativity(m in arb_track(), n in arb_track(), p in arb_track()) {
        let left = together(vec![together(vec![m.clone(), n.clone()]), p.clone()]);
        let right = together(vec![m, together(vec![n, p])]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L5: M ⊕ N = N ⊕ M.
    #[test]
    fn together_commutativity(m in arb_track(), n in arb_track()) {
        prop_assert!(together(vec![m.clone(), n.clone()]).semantic_eq(&together(vec![n, m])));
    }

    /// L6: at a fixed duration d, (d, ∅) is the `together` identity.
    #[test]
    fn together_fixed_duration_identity(m in arb_track()) {
        let empty = empty::<WrittenTime, u8>(m.duration());
        prop_assert!(together(vec![m.clone(), empty.clone()]).semantic_eq(&m));
        prop_assert!(together(vec![empty, m.clone()]).semantic_eq(&m));
    }

    /// L9: EventTrack(id) = id.
    #[test]
    fn map_identity(m in arb_track()) {
        prop_assert!(m.map_payloads(|p| *p).semantic_eq(&m));
    }

    /// L10: EventTrack(g ∘ f) = EventTrack(g) ∘ EventTrack(f).
    #[test]
    fn map_composition(m in arb_track()) {
        let f = |p: &u8| format!("n{p}");
        let g = |s: &String| format!("[{s}]");
        let composed = m.map_payloads(|p| g(&f(p)));
        let stepwise = m.map_payloads(f).map_payloads(g);
        prop_assert!(composed.semantic_eq(&stepwise));
    }

    /// L11: EventTrack(f) preserves `follow`.
    #[test]
    fn map_preserves_follow(m in arb_track(), n in arb_track()) {
        let f = |p: &u8| p.saturating_add(10);
        let left = follow(vec![m.clone(), n.clone()]).map_payloads(f);
        let right = follow(vec![m.map_payloads(f), n.map_payloads(f)]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L12: EventTrack(f) preserves `together`.
    #[test]
    fn map_preserves_together(m in arb_track(), n in arb_track()) {
        let f = |p: &u8| p.saturating_add(10);
        let left = together(vec![m.clone(), n.clone()]).map_payloads(f);
        let right = together(vec![m.map_payloads(f), n.map_payloads(f)]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L13a: scale_1 = id.
    #[test]
    fn scale_identity(m in arb_track()) {
        let same = m.scale(Ratio::from_integer(1)).expect("positive");
        prop_assert!(same.semantic_eq(&m));
    }

    /// L13b: scale_r ∘ scale_s = scale_{r·s}.
    #[test]
    fn scale_composition(m in arb_track(), r in 1i64..=4, s in 1i64..=4, sd in 1i64..=4) {
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
    fn scale_preserves_follow(m in arb_track(), n in arb_track(), r in 1i64..=4) {
        let factor = Ratio::new(r, 2);
        let left = follow(vec![m.clone(), n.clone()]).scale(factor).expect("positive");
        let right = follow(vec![
            m.scale(factor).expect("positive"),
            n.scale(factor).expect("positive"),
        ]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L15: scale preserves overlay.
    #[test]
    fn scale_preserves_together(m in arb_track(), n in arb_track(), r in 1i64..=4) {
        let factor = Ratio::new(r, 2);
        let left = together(vec![m.clone(), n.clone()]).scale(factor).expect("positive");
        let right = together(vec![
            m.scale(factor).expect("positive"),
            n.scale(factor).expect("positive"),
        ]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// L16: restrict at the full duration is the identity on observations —
    /// every occurrence is reported, and its visible span is its whole span.
    #[test]
    fn restrict_identity(m in arb_track()) {
        let full = Span::new(Position::ZERO, m.duration().reach()).expect("ordered");
        let observed = observed_pairs(&m.restrict(full));
        let whole: Vec<(Span<WrittenTime>, Span<WrittenTime>)> = m
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
    fn restrict_composition(mjk in arb_track_with_two_windows()) {
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
        let left = follow(vec![together(vec![m.clone(), n.clone()]), together(vec![p.clone(), q.clone()])]);
        let right = together(vec![follow(vec![m, p]), follow(vec![n, q])]);
        prop_assert!(left.semantic_eq(&right));
    }

    /// X1: M ⊕ M ≠ M when M has occurrences — multiplicity doubles (§8).
    #[test]
    fn together_not_idempotent(m in arb_track()) {
        prop_assume!(!m.occurrences().is_empty());
        let doubled = together(vec![m.clone(), m.clone()]);
        prop_assert_eq!(doubled.occurrences().len(), 2 * m.occurrences().len());
        prop_assert!(!doubled.semantic_eq(&m));
    }

    /// L19: semantic equality is a congruence — swapping an argument for a
    /// semantically equal one preserves results.
    #[test]
    fn semantic_equality_is_congruence(m in arb_track(), n in arb_track()) {
        let shuffled: Vec<Occurrence<WrittenTime, u8>> = m.occurrences().iter().rev().cloned().collect();
        let rebuilt = track(m.duration(), shuffled).expect("same spans");
        prop_assert!(m.semantic_eq(&rebuilt));
        prop_assert!(follow(vec![m.clone(), n.clone()]).semantic_eq(&follow(vec![rebuilt.clone(), n.clone()])));
        prop_assert!(together(vec![m, n.clone()]).semantic_eq(&together(vec![rebuilt, n])));
    }

    /// L20: coverage is observation, taken as narrowly as possible. An
    /// occurrence covers `t` exactly when every window containing `t`
    /// observes it — the two ways of asking cannot disagree.
    #[test]
    fn coverage_agrees_with_observation(m in arb_track(), at in 0i64..16) {
        let at = quarters(at);
        if at > m.duration().reach() {
            return Ok(());
        }
        let covering: Vec<Span<WrittenTime>> = m.covering(at).map(Occurrence::span).collect();
        // Every window containing `at`, out of a family that includes the
        // tightest ones on both sides.
        for width in [1i64, 2, 4] {
            let start = Position::new(at.as_ratio() - quarters(width).as_ratio());
            let start = if start.as_ratio() < Ratio::ZERO { Position::ZERO } else { start };
            let end = Position::new(at.as_ratio() + quarters(width).as_ratio());
            let window = Span::new(start, end).expect("ordered");
            let observed: Vec<Span<WrittenTime>> = m.restrict(window).observed().map(|(_, o)| o.span()).collect();
            for span in &covering {
                prop_assert!(observed.contains(span), "a covering occurrence must be observed through {window}");
            }
        }
        // And nothing else covers `at`. Every occurrence that contains `at`
        // is visible through the tightest window starting there, so filtering
        // that observation by containment must reproduce `covering` exactly —
        // as a multiset, since equal occurrences are distinct facts (K6).
        let tight = Span::new(at, at.plus(beats(1))).expect("ordered");
        let mut observed: Vec<(Span<WrittenTime>, u8)> = m
            .restrict(tight)
            .observed()
            .map(|(_, occurrence)| (occurrence.span(), *occurrence.payload()))
            .filter(|(span, _)| span.contains(at))
            .collect();
        let mut covered: Vec<(Span<WrittenTime>, u8)> = m
            .covering(at)
            .map(|occurrence| (occurrence.span(), *occurrence.payload()))
            .collect();
        observed.sort_by_key(|(span, payload)| (span.start(), span.end(), *payload));
        covered.sort_by_key(|(span, payload)| (span.start(), span.end(), *payload));
        prop_assert_eq!(observed, covered);
    }

    /// L21: coverage commutes with the algebra. Scaling moves the question
    /// with the music, and `follow` moves it by the first duration.
    #[test]
    fn coverage_is_stable_under_time_transformation(m in arb_track(), n in arb_track(), at in 1i64..12) {
        let at = quarters(at);
        if at > m.duration().reach() {
            return Ok(());
        }
        let factor = num_rational::Ratio::new(3, 2);
        let scaled = m.scale(factor).expect("positive factor");
        let here: Vec<&u8> = m.covering(at).map(Occurrence::payload).collect();
        let there: Vec<&u8> = scaled.covering(Position::new(at.as_ratio() * factor)).map(Occurrence::payload).collect();
        prop_assert_eq!(here, there);

        // Past the seam, a sequence answers with its second argument alone.
        // *At* the seam both may answer — a point at `m`'s duration and `n`'s
        // material at 0 share that instant — which is D2's boundary, not a
        // defect, so the law is stated strictly past it.
        let joined = follow(vec![m.clone(), n.clone()]);
        let after = m.duration().reach().plus(at.since(Position::ZERO).expect("nonnegative"));
        if at <= n.duration().reach() {
            let inside: Vec<&u8> = n.covering(at).map(Occurrence::payload).collect();
            let outside: Vec<&u8> = joined.covering(after).map(Occurrence::payload).collect();
            prop_assert_eq!(inside, outside);
        }
    }

    /// L22: prevailing is the last selected start. Stated against an
    /// independent scan, so the law does not check the implementation
    /// against itself.
    #[test]
    fn prevailing_is_the_last_selected_start(m in arb_track(), at in 0i64..16) {
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
    fn prevailing_ignores_facts_that_start_later(m in arb_track(), at in 0i64..8) {
        let at = quarters(at);
        let select = |payload: &u8| payload.is_multiple_of(2).then_some(*payload);
        let before = m.prevailing(at, select);
        // Everything in `later` starts strictly after `at`.
        let start = at.plus(beats(1));
        let end = start.plus(beats(2));
        let span = Span::new(start, end).expect("ordered");
        let later = track(
            end.since(Position::ZERO).expect("nonnegative"),
            vec![Occurrence::new(span, 2u8), Occurrence::new(span, 4u8)],
        )
        .expect("in bounds");
        prop_assert_eq!(together(vec![m, later]).prevailing(at, select), before);
    }

    /// N6: the semantic hash agrees with semantic equality. Equal meaning,
    /// equal digest — the direction a caller relies on when an unequal digest
    /// makes it rebuild something.
    #[test]
    fn semantic_equality_implies_equal_hashes(m in arb_track(), n in arb_track()) {
        let shuffled: Vec<Occurrence<WrittenTime, u8>> = m.occurrences().iter().rev().cloned().collect();
        let rebuilt = track(m.duration(), shuffled).expect("same spans");
        prop_assert!(m.semantic_eq(&rebuilt));
        prop_assert_eq!(m.semantic_hash(), rebuilt.semantic_hash());
        prop_assert_eq!(
            follow(vec![m.clone(), n.clone()]).semantic_hash(),
            follow(vec![rebuilt.clone(), n.clone()]).semantic_hash()
        );
        prop_assert_eq!(
            together(vec![m.clone(), n.clone()]).semantic_hash(),
            together(vec![rebuilt, n]).semantic_hash()
        );
        // And the contrapositive is what makes the digest worth computing:
        // a track that says something else digests differently.
        let stretched = m.duration().plus(beats(1));
        let longer = track(stretched, m.occurrences().to_vec()).expect("a longer track still contains them");
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
        raw_m in arb_track(),
        raw_n in arb_track(),
        raw_p in arb_track(),
        synchronized_mn in arb_synchronized_pair(),
        synchronized_pq in arb_synchronized_pair(),
        windows in arb_track_with_two_windows(),
        at_quarters in 0i64..=12,
    ) {
        let m = probe_track(&raw_m);
        let n = probe_track(&raw_n);
        let p = probe_track(&raw_p);

        // L1–L6 and X1.
        prop_assert!(follow(vec![follow(vec![m.clone(), n.clone()]), p.clone()])
            .semantic_eq(&follow(vec![m.clone(), follow(vec![n.clone(), p.clone()])] )));
        let zero = empty::<WrittenTime, AdmissionProbe>(Duration::ZERO);
        prop_assert!(follow(vec![zero.clone(), m.clone()]).semantic_eq(&m));
        prop_assert!(follow(vec![m.clone(), zero]).semantic_eq(&m));
        prop_assert_eq!(
            follow(vec![m.clone(), n.clone()]).duration().as_ratio(),
            m.duration().as_ratio() + n.duration().as_ratio()
        );
        prop_assert!(together(vec![together(vec![m.clone(), n.clone()]), p.clone()])
            .semantic_eq(&together(vec![m.clone(), together(vec![n.clone(), p])] )));
        prop_assert!(together(vec![m.clone(), n.clone()]).semantic_eq(&together(vec![n.clone(), m.clone()])));
        let empty = empty::<WrittenTime, AdmissionProbe>(m.duration());
        prop_assert!(together(vec![m.clone(), empty]).semantic_eq(&m));
        if !m.occurrences().is_empty() {
            prop_assert!(!together(vec![m.clone(), m.clone()]).semantic_eq(&m));
        }

        // L9–L12.
        prop_assert!(m.map_payloads(Clone::clone).semantic_eq(&m));
        let f = |value: &AdmissionProbe| format!("{}:{}", value.label, value.ratio);
        let g = |value: &String| format!("[{value}]");
        prop_assert!(m.map_payloads(|value| g(&f(value))).semantic_eq(&m.map_payloads(f).map_payloads(g)));
        let rename = |value: &AdmissionProbe| format!("{}:{}", value.label, value.shape.canonical_key());
        prop_assert!(follow(vec![m.clone(), n.clone()]).map_payloads(rename)
            .semantic_eq(&follow(vec![m.map_payloads(rename), n.map_payloads(rename)])));
        prop_assert!(together(vec![m.clone(), n.clone()]).map_payloads(rename)
            .semantic_eq(&together(vec![m.map_payloads(rename), n.map_payloads(rename)])));

        // L13–L15.
        prop_assert!(m.scale(Ratio::ONE).expect("positive").semantic_eq(&m));
        let r = Ratio::new(3, 2);
        let s = Ratio::new(5, 4);
        prop_assert!(m.scale(s).and_then(|scaled| scaled.scale(r)).expect("positive")
            .semantic_eq(&m.scale(r * s).expect("positive")));
        prop_assert!(follow(vec![m.clone(), n.clone()]).scale(r).expect("positive")
            .semantic_eq(&follow(vec![m.scale(r).expect("positive"), n.scale(r).expect("positive")])));
        prop_assert!(together(vec![m.clone(), n.clone()]).scale(r).expect("positive")
            .semantic_eq(&together(vec![m.scale(r).expect("positive"), n.scale(r).expect("positive")])));

        // L16–L17.
        let full = Span::new(Position::ZERO, m.duration().reach()).expect("ordered");
        let observed = probe_observed(&m.restrict(full));
        let whole: Vec<(Span<WrittenTime>, Span<WrittenTime>, String)> = m.occurrences().iter().map(|occurrence| {
            (occurrence.span(), occurrence.span(), occurrence.payload().canonical_key())
        }).collect();
        prop_assert_eq!(observed, whole);
        let (window_source, (j0, j1), (k0, k1)) = windows;
        let window_source = probe_track(&window_source);
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
            probe_track(&sm), probe_track(&sn), probe_track(&sp), probe_track(&sq),
        );
        prop_assert!(follow(vec![together(vec![sm.clone(), sn.clone()]), together(vec![sp.clone(), sq.clone()])])
            .semantic_eq(&together(vec![follow(vec![sm, sp]), follow(vec![sn, sq])])));
        let reversed = track(
            m.duration(),
            m.occurrences().iter().rev().cloned().collect(),
        ).expect("same bounds");
        prop_assert!(m.semantic_eq(&reversed));
        prop_assert!(together(vec![m.clone(), n.clone()]).semantic_eq(&together(vec![reversed.clone(), n])));

        // L20–L23.
        let at = quarters(at_quarters);
        if at <= m.duration().reach() {
            let mut covered: Vec<(Span<WrittenTime>, String)> = m.covering(at)
                .map(|occurrence| (occurrence.span(), occurrence.payload().canonical_key()))
                .collect();
            let tight_end = Position::new(at.plus(beats(1)).as_ratio().min(m.duration().as_ratio()));
            if tight_end > at {
                let tight = Span::new(at, tight_end).expect("ordered");
                let mut observed: Vec<(Span<WrittenTime>, String)> = m.restrict(tight).observed()
                    .map(|(_, occurrence)| (occurrence.span(), occurrence.payload().canonical_key()))
                    .filter(|(span, _)| span.contains(at))
                    .collect();
                covered.sort_by_key(|(span, key)| (span.start(), span.end(), key.clone()));
                observed.sort_by_key(|(span, key)| (span.start(), span.end(), key.clone()));
                prop_assert_eq!(covered, observed);
            }
            let scaled = m.scale(r).expect("positive");
            let here: Vec<String> = m.covering(at).map(|o| o.payload().canonical_key()).collect();
            let there: Vec<String> = scaled.covering(Position::new(at.as_ratio() * r))
                .map(|o| o.payload().canonical_key()).collect();
            prop_assert_eq!(here, there);
        }
        let select = |value: &AdmissionProbe| (value.ratio.numer() % 2 == 0).then_some(value.label.clone());
        let expected = m.canonical_occurrences().into_iter()
            .rfind(|o| o.span().start() <= at && o.payload().ratio.numer() % 2 == 0)
            .map(|o| o.payload().label.clone());
        prop_assert_eq!(m.prevailing(at, select), expected);
        let later_start = at.plus(beats(1));
        let later_end = later_start.plus(beats(1));
        let later = track(later_end.since(Position::ZERO).expect("nonnegative"), vec![Occurrence::new(
            Span::new(later_start, later_end).expect("ordered"), probe(1),
        )]).expect("in bounds");
        prop_assert_eq!(together(vec![m.clone(), later]).prevailing(at, select), m.prevailing(at, select));

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
    let m = track(beats(8), vec![occurrence_at(0, 4, 1), occurrence_at(4, 8, 2)]).expect("valid");
    let bytes = independently_frame_a_written_u8_track(&m);
    assert_eq!(m.semantic_hash().to_string(), fnv1a_128(&bytes));
    // Fixed here so a change of algorithm, offset basis, or byte order fails
    // loudly rather than silently invalidating every stored identity. This
    // value belongs to encoding version 3 — the version that frames the
    // coordinate — and differs from version 2's for exactly that reason. It
    // moved once more when the events vocabulary amendment renamed this
    // payload's owner id from `musa.kernel.u8` to `musa.events.u8`, which the
    // frame includes by construction (`docs/rules/events/05-normalization.md`).
    assert_eq!(m.semantic_hash().to_string(), "668c2a70c18aba513487e41ebd488c0f");
}

fn independently_frame_a_written_u8_track(value: &Track<u8>) -> Vec<u8> {
    fn bytes(out: &mut Vec<u8>, value: &[u8]) {
        out.extend_from_slice(&(value.len() as u64).to_be_bytes());
        out.extend_from_slice(value);
    }
    fn rational(out: &mut Vec<u8>, ratio: Ratio<i64>) {
        out.extend_from_slice(&ratio.numer().to_be_bytes());
        out.extend_from_slice(&ratio.denom().to_be_bytes());
    }

    // The field order of `across-stages/04-identity-and-realization.md` §3:
    // domain, version, coordinate, payload owner, quotient version, duration,
    // count, occurrences.
    let mut out = Vec::new();
    bytes(&mut out, b"musa.event-track.semantic");
    out.extend_from_slice(&3u32.to_be_bytes());
    bytes(&mut out, b"WrittenTime");
    bytes(&mut out, b"musa.events.u8");
    out.extend_from_slice(&1u32.to_be_bytes());
    rational(&mut out, value.duration().as_ratio());
    out.extend_from_slice(&(value.occurrences().len() as u64).to_be_bytes());
    for occurrence in value.canonical_occurrences() {
        rational(&mut out, occurrence.span().start().as_ratio());
        rational(&mut out, occurrence.span().end().as_ratio());
        bytes(&mut out, occurrence.payload().canonical_key().as_bytes());
    }
    out
}

#[test]
fn framed_identity_separates_the_old_display_collision() {
    let one = track(
        beats(8),
        vec![Occurrence::new(
            Span::new(quarters(4), quarters(8)).expect("ordered"),
            "a from 0 to 1;\n  occurrence b".to_owned(),
        )],
    )
    .expect("in bounds");
    let two = track(
        beats(8),
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

impl musa_events::Canonical for SameKeyOtherSchema {
    const OWNER_TYPE_ID: &'static str = "musa.events.tests.OtherSchema";
    const QUOTIENT_VERSION: u32 = 7;

    fn canonical_key(&self) -> String {
        "1".to_owned()
    }
}

#[test]
fn framed_identity_covers_schema_and_multiplicity() {
    let scalar = track(beats(4), vec![occurrence_at(0, 4, 1)]).expect("valid");
    let other = track(
        beats(4),
        vec![Occurrence::new(
            Span::new(quarters(0), quarters(4)).expect("ordered"),
            SameKeyOtherSchema,
        )],
    )
    .expect("valid");
    let doubled = together(vec![scalar.clone(), scalar.clone()]);

    assert_ne!(scalar.semantic_hash(), other.semantic_hash());
    assert_ne!(scalar.semantic_hash(), doubled.semantic_hash());
}

proptest! {
    #[test]
    fn arbitrary_payload_delimiters_preserve_equality_hash_agreement(
        quarters in 0i64..=16,
        payloads in prop::collection::vec(any::<String>(), 0..8),
    ) {
        let duration = beats(quarters);
        let occurrences: Vec<Occurrence<WrittenTime, String>> = payloads.into_iter().map(|payload| {
            Occurrence::new(Span::new(Position::ZERO, duration.reach()).expect("ordered"), payload)
        }).collect();
        let value = track(duration, occurrences).expect("in bounds");
        let shuffled = track(duration, value.occurrences().iter().rev().cloned().collect()).expect("same bounds");
        prop_assert!(value.semantic_eq(&shuffled));
        prop_assert_eq!(value.semantic_hash(), shuffled.semantic_hash());
    }
}

/// The published FNV-1a 128 parameters, written out independently of the
/// event track's implementation so the test can disagree with it.
fn fnv1a_128(bytes: &[u8]) -> String {
    let mut state: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for byte in bytes {
        state ^= u128::from(*byte);
        state = state.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b);
    }
    format!("{state:032x}")
}

/// X2: `follow` does not distribute over `together` (§10). The left side has
/// one copy of M's occurrence; the right side has two.
#[test]
fn follow_does_not_distribute_over_together() {
    let m = track(beats(4), vec![occurrence_at(0, 4, 1)]).expect("valid");
    let n = track(beats(4), vec![occurrence_at(0, 4, 2)]).expect("valid");
    let p = track(beats(4), vec![occurrence_at(0, 4, 3)]).expect("valid");
    let left = follow(vec![m.clone(), together(vec![n.clone(), p.clone()])]);
    let right = together(vec![follow(vec![m.clone(), n]), follow(vec![m, p])]);
    assert!(!left.semantic_eq(&right));
}

/// K1: a track refuses an occurrence that leaves its ambient region.
///
/// The bound is the whole of what makes `(d, E)` well formed, and it is
/// checked at construction rather than asked about later — an occurrence past
/// the duration is not a track that behaves oddly, it is not a track. Both
/// edges are exercised: ending exactly at `d` is legal, one quarter past it is
/// not.
#[test]
fn a_track_refuses_an_occurrence_outside_its_duration() {
    assert!(
        track(beats(4), vec![occurrence_at(0, 4, 1)]).is_ok(),
        "ending at d is inside"
    );
    let escaping = track(beats(4), vec![occurrence_at(0, 5, 1)]);
    assert!(
        matches!(escaping, Err(musa_events::EventsError::OccurrenceOutOfBounds { .. })),
        "an occurrence past the duration was admitted: {escaping:?}"
    );
    // A point at the duration is the boundary case D6 turns on, and it is in.
    assert!(track(beats(4), vec![occurrence_at(4, 4, 1)]).is_ok());
}

/// A duration is the nonnegative half of exact time, so a negative one is
/// refused where it is written rather than carried into a track.
#[test]
fn a_negative_duration_is_refused_at_construction() {
    let backwards = Duration::<WrittenTime>::new(Ratio::new(-1, 4));
    assert!(matches!(
        backwards,
        Err(musa_events::EventsError::NegativeDuration { .. })
    ));
    // And two positions subtract to a duration only in the order that makes
    // one: the reverse is an error, not a negative amount of time.
    assert!(quarters(8).since(quarters(4)).is_ok());
    assert!(quarters(4).since(quarters(8)).is_err());
}

/// D3, in the case the ledger keeps off the deletion list: `together` of
/// unequal durations takes the longer one and pads nothing.
///
/// Stated as a property because the temptation it guards against is
/// generic — an implementation that "aligned" its arguments would pass every
/// equal-duration case and be wrong exactly here.
#[test]
fn together_takes_the_longer_duration_and_inserts_nothing() {
    let short = track(beats(4), vec![occurrence_at(0, 4, 1)]).expect("valid");
    let long = track(beats(12), vec![occurrence_at(0, 4, 2)]).expect("valid");
    let stacked = together(vec![short.clone(), long.clone()]);
    assert_eq!(stacked.duration(), beats(12));
    assert_eq!(stacked.occurrences().len(), 2, "nothing was inserted to fill the tail");
    // Commutative in duration as well as in content.
    assert_eq!(together(vec![long, short]).duration(), beats(12));
}

/// K6: occurrences are a multiset. Two identical facts are two facts.
///
/// The one property a set-backed implementation would silently break, and the
/// one a musician would notice first — two players in unison are not one.
#[test]
fn equal_occurrences_are_kept_apart_as_a_multiset() {
    let unison = track(beats(4), vec![occurrence_at(0, 4, 1), occurrence_at(0, 4, 1)]).expect("valid");
    assert_eq!(unison.occurrences().len(), 2);
    assert_eq!(unison.canonical_occurrences().len(), 2, "canonical order retains both");
    assert_eq!(unison.covering(quarters(0)).count(), 2);
    let single = track(beats(4), vec![occurrence_at(0, 4, 1)]).expect("valid");
    assert!(!unison.semantic_eq(&single));
    assert_ne!(unison.semantic_hash(), single.semantic_hash());
}

/// D7: `map_payloads` is a functor on payloads *only* — spans, count, and
/// order are D7's to preserve, which is what L11–L15 rest on.
#[test]
fn mapping_payloads_moves_no_time() {
    let m = track(
        beats(16),
        vec![occurrence_at(0, 4, 1), occurrence_at(4, 4, 2), occurrence_at(6, 15, 3)],
    )
    .expect("valid");
    let mapped = m.map_payloads(|value| format!("v{value}"));
    assert_eq!(mapped.duration(), m.duration());
    let before: Vec<Span<WrittenTime>> = m.occurrences().iter().map(Occurrence::span).collect();
    let after: Vec<Span<WrittenTime>> = mapped.occurrences().iter().map(Occurrence::span).collect();
    assert_eq!(before, after, "a payload map moved a span");
}

/// D0/D10: support is half-open, and a point is the stated exception.
///
/// This is the convention `follow` depends on — the second track's first
/// instant is the first one's end, and one instant must not be inside both —
/// so it is tested directly rather than only through the laws that assume it.
#[test]
fn support_is_half_open_and_a_point_contains_its_own_instant() {
    let positive = Span::new(quarters(4), quarters(8)).expect("ordered");
    assert!(positive.contains(quarters(4)), "the start instant is inside");
    assert!(positive.contains(quarters(7)));
    assert!(
        !positive.contains(quarters(8)),
        "the end instant belongs to what follows"
    );

    let point = Span::new(quarters(4), quarters(4)).expect("ordered");
    assert!(point.contains(quarters(4)), "a point is present at its own instant");
    assert!(!point.contains(quarters(5)));

    // And the seam: following two tracks puts the second's first instant
    // exactly where the first's support ended, so nothing covers it twice.
    let first = track(beats(4), vec![occurrence_at(0, 4, 1)]).expect("valid");
    let second = track(beats(4), vec![occurrence_at(0, 4, 2)]).expect("valid");
    let joined = follow(vec![first, second]);
    let at_the_seam: Vec<u8> = joined.covering(quarters(4)).map(|o| *o.payload()).collect();
    assert_eq!(at_the_seam, vec![2]);
}

/// The coordinate is part of exact identity, even though it is carried as a
/// type (`across-stages/04-identity-and-realization.md` §2).
///
/// Two tracks in different coordinates are different types, so *combining*
/// them is a compile error and cannot be tested here — the whole point of the
/// tag. What can be tested is the consequence for bytes: identity is a digest
/// over an encoding, an encoding has no type parameters, and without the tag
/// in the encoding a written-time track and a performed-time track with the
/// same rationals would be indistinguishable to a cache.
#[test]
fn the_coordinate_is_part_of_exact_identity() {
    let written = track(beats(8), vec![occurrence_at(0, 4, 1), occurrence_at(4, 8, 2)]).expect("valid");
    let performed: EventTrack<PerformedTime, u8> = track(
        Duration::new(Ratio::new(8, QUARTER)).expect("nonnegative"),
        vec![
            Occurrence::new(
                Span::new(
                    Position::new(Ratio::new(0, QUARTER)),
                    Position::new(Ratio::new(4, QUARTER)),
                )
                .expect("ordered"),
                1u8,
            ),
            Occurrence::new(
                Span::new(
                    Position::new(Ratio::new(4, QUARTER)),
                    Position::new(Ratio::new(8, QUARTER)),
                )
                .expect("ordered"),
                2u8,
            ),
        ],
    )
    .expect("valid");
    assert_ne!(
        written.semantic_hash(),
        performed.semantic_hash(),
        "the same rationals in two coordinates must not share an identity"
    );
}

/// L18 fails without the synchronization preconditions.
#[test]
fn interchange_fails_without_synchronization() {
    // duration(M) = 1 beat ≠ duration(N) = 2 beats.
    let m = track(beats(4), vec![occurrence_at(0, 4, 1)]).expect("valid");
    let n = track(beats(8), vec![occurrence_at(0, 8, 2)]).expect("valid");
    let p = track(beats(4), vec![occurrence_at(0, 4, 3)]).expect("valid");
    let q = track(beats(4), vec![occurrence_at(0, 4, 4)]).expect("valid");
    let left = follow(vec![
        together(vec![m.clone(), n.clone()]),
        together(vec![p.clone(), q.clone()]),
    ]);
    let right = together(vec![follow(vec![m, p]), follow(vec![n, q])]);
    assert!(!left.semantic_eq(&right));
}

/// L17 with a strictly nested window K ⊂ J ⊂ I: the case the law was first
/// stated for, kept as a worked example alongside the general property.
#[test]
fn restrict_composition_strictly_nested() {
    let m = track(
        beats(16),
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

fn occurrence_at(start: i64, end: i64, payload: u8) -> Occurrence<WrittenTime, u8> {
    let span = Span::new(quarters(start), quarters(end)).expect("ordered");
    Occurrence::new(span, payload)
}

/// L24 — a curve-bearing occurrence transforms by its span alone.
///
/// `Progress` is indexed by normalized *local* time, so every event-track operation
/// moves or stretches the span and leaves the payload bytes untouched. This is
/// what makes a continuous shape a payload value rather than an event track
/// operation (docs/rules/events/03 `Progress`, §32 Q4): if the curve were in
/// absolute time, `scale` and `sequence` would have to rewrite it, and the
/// event track would be looking inside payloads (§12).
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
    let m = track(beats(16), vec![Occurrence::new(span, curve.clone())]).expect("in bounds");

    // A probe at the absolute instant one quarter of the way through the
    // occurrence: u = 1/4 before and after every operation.
    let u = Ratio::new(1, 4);
    let expected = curve.at(u);

    let scaled = m.scale(Ratio::new(3, 1)).expect("positive");
    let delayed = follow(vec![track(beats(8), vec![]).expect("empty"), m.clone()]);
    let stacked = together(vec![m.clone(), track(beats(16), vec![]).expect("empty")]);
    let observed = m.restrict(Span::new(Position::ZERO, quarters(16)).expect("ordered"));

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
