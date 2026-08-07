//! The term-calculus theorem suite (docs/kernel/10-term-calculus.md T1–T5),
//! plus the transported algebra: L1/L4/L5/L18 asked at the term level.
//!
//! Every test name matches the theorem's `Test:` line in the specification.
//! Equality is semantic equality (N4) throughout, because that is the only
//! equality the kernel has and the whole point of T3 is that terms do not get
//! a second one.

// Rational test arithmetic is exact and total (musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]
// Generators use expect() on in-bounds constructions: a failure is a bug in
// the generator, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_kernel::{Beat, Occurrence, Span, Term, Timeline, evaluate, overlay, sequence, timeline};
use num_rational::Ratio;
use proptest::prelude::*;

const QUARTER: i64 = 4;

fn quarters(q: i64) -> Beat {
    Beat::new(Ratio::new(q, QUARTER))
}

fn arb_literal() -> impl Strategy<Value = Timeline<u8>> {
    (0i64..=12).prop_flat_map(|extent| {
        prop::collection::vec((0..=extent, 0..=extent, 0u8..8), 0..4).prop_map(move |raw| {
            let occurrences = raw
                .into_iter()
                .map(|(a, b, payload)| {
                    let span = Span::new(quarters(a.min(b)), quarters(a.max(b))).expect("ordered");
                    Occurrence::new(span, payload)
                })
                .collect();
            timeline(quarters(extent), occurrences).expect("in bounds")
        })
    })
}

/// Closed, well-formed terms with a bounded depth. Terms are finite, so the
/// generator is too: the recursion stops at `depth`, and every `Var` is
/// generated *inside* the `Let` that binds it, so the strategy cannot produce
/// a free name.
fn arb_term(depth: u32) -> BoxedStrategy<Term<u8>> {
    let leaf = arb_literal().prop_map(Term::literal).boxed();
    if depth == 0 {
        return leaf;
    }
    let inner = || arb_term(depth - 1);
    prop_oneof![
        3 => leaf,
        2 => prop::collection::vec(inner(), 1..3).prop_map(|parts| Term::seq(parts).expect("non-empty")),
        2 => prop::collection::vec(inner(), 1..3).prop_map(|parts| Term::over(parts).expect("non-empty")),
        1 => (0i64..8, inner()).prop_map(|(by, body)| Term::shift(quarters(by), body).expect("non-negative")),
        1 => (1i64..4, 1i64..4, inner())
            .prop_map(|(n, d, body)| Term::scale(Ratio::new(n, d), body).expect("positive")),
        1 => (0i64..12, 0i64..12, inner()).prop_map(|(a, b, body)| {
            let window = Span::new(quarters(a.min(b)), quarters(a.max(b))).expect("ordered");
            Term::restrict(window, body)
        }),
        2 => (inner(), inner()).prop_map(move |(value, body)| {
            // The reference is generated inside the binder, so the term is
            // closed by construction — and used twice, which is the case
            // sharing exists for. The name carries the depth because
            // shadowing is rejected (K7) and nesting would otherwise collide.
            let name = format!("x{depth}");
            let body = Term::seq(vec![Term::var(&name), body, Term::var(&name)]).expect("non-empty");
            Term::bind(name, value, body)
        }),
    ]
    .boxed()
}

/// D6 as a value, mirroring the evaluator's own materialization: the
/// occurrences a window shows, keeping their whole spans, in a timeline of the
/// observed extent. Written here independently so T1's `restrict` case
/// compares two implementations rather than one against itself.
fn observed_value(value: &Timeline<u8>, window: Span) -> Timeline<u8> {
    let observation = value.restrict(window);
    let occurrences = observation.observed().map(|(_, o)| o.clone()).collect();
    timeline(value.extent(), occurrences).expect("a subset of a valid timeline")
}

proptest! {
    /// T1 — the constructors are a homomorphism. Building a term and
    /// evaluating it is the same as evaluating the parts and combining the
    /// values, for every combining form.
    #[test]
    fn term_constructors_are_a_homomorphism(
        left in arb_term(2),
        right in arb_term(2),
        n in 1i64..4,
        d in 1i64..4,
        a in 0i64..12,
        b in 0i64..12,
    ) {
        let (lv, rv) = (evaluate(&left), evaluate(&right));

        let seq_term = Term::seq(vec![left.clone(), right.clone()]).expect("non-empty");
        prop_assert!(evaluate(&seq_term).semantic_eq(&sequence(vec![lv.clone(), rv.clone()])));

        let over_term = Term::over(vec![left.clone(), right]).expect("non-empty");
        prop_assert!(evaluate(&over_term).semantic_eq(&overlay(vec![lv.clone(), rv])));

        let factor = Ratio::new(n, d);
        let scaled = Term::scale(factor, left.clone()).expect("positive");
        prop_assert!(evaluate(&scaled).semantic_eq(&lv.scale(factor).expect("positive")));

        let window = Span::new(quarters(a.min(b)), quarters(a.max(b))).expect("ordered");
        let restricted = Term::restrict(window, left);
        prop_assert!(evaluate(&restricted).semantic_eq(&observed_value(&lv, window)));
    }

    /// T2 — `let` is transparent: a shared term and the same term with the
    /// binding used once denote the same timeline. Sharing changes cost,
    /// never meaning.
    #[test]
    fn let_is_transparent(value in arb_term(2), body in arb_term(1)) {
        // `let x = v in seq(x, body, x)` against `seq(v, body, v)` — the
        // expansion T2 states, written out.
        let shared = Term::bind(
            "x",
            value.clone(),
            Term::seq(vec![Term::var("x"), body.clone(), Term::var("x")]).expect("non-empty"),
        );
        // `u[t/x]` is built directly rather than computed by a substituting
        // evaluator: `Term` is opaque, so the substituted term is written out
        // here, which is the same reference and a shorter one.
        let expanded = Term::seq(vec![value.clone(), body, value]).expect("non-empty");
        prop_assert!(evaluate(&shared).semantic_eq(&evaluate(&expanded)));
    }

    /// T3 — evaluation is normalization: normalizing an evaluated term is the
    /// canonical form, and two terms are semantically equal exactly when
    /// their canonical forms — and therefore their hashes — agree. There is
    /// one equality in this kernel.
    #[test]
    fn evaluation_agrees_with_normalization(left in arb_term(2), right in arb_term(2)) {
        let (lv, rv) = (evaluate(&left), evaluate(&right));
        prop_assert!(lv.normalize().semantic_eq(&lv), "normalize preserves meaning");
        prop_assert_eq!(
            lv.semantic_eq(&rv),
            lv.normalize().to_string() == rv.normalize().to_string(),
            "semantic equality is equality of canonical forms"
        );
        if lv.semantic_eq(&rv) {
            prop_assert_eq!(lv.semantic_hash(), rv.semantic_hash());
        }
    }

    /// T4 — totality: every closed well-formed term checks and evaluates, in
    /// finitely many steps. A diverging or panicking evaluation fails here.
    #[test]
    fn every_well_formed_term_evaluates(term in arb_term(3)) {
        prop_assert!(term.check().is_ok(), "the generator produces closed terms");
        let value = evaluate(&term);
        prop_assert!(value.extent() >= Beat::ZERO);
        // Deterministic: the rules are syntax-directed, one per form.
        prop_assert!(evaluate(&term).semantic_eq(&value));
    }

    /// T5 — observation commutes with sharing: a restriction may be pushed
    /// through a binding without changing the answer. This is what makes
    /// deferred observation sound, and prompt 50 depends on it.
    #[test]
    fn restriction_commutes_with_sharing(
        value in arb_term(2),
        body in arb_term(1),
        a in 0i64..12,
        b in 0i64..12,
    ) {
        let window = Span::new(quarters(a.min(b)), quarters(a.max(b))).expect("ordered");
        let inner = Term::seq(vec![Term::var("x"), body, Term::var("x")]).expect("non-empty");
        let outside = Term::restrict(window, Term::bind("x", value.clone(), inner.clone()));
        let inside = Term::bind("x", value, Term::restrict(window, inner));
        prop_assert!(evaluate(&outside).semantic_eq(&evaluate(&inside)));
    }

    /// The algebra transports: L1 (sequence associativity), L4/L5 (overlay
    /// commutativity and associativity) and L18 (synchronized interchange)
    /// hold of terms because T1 says the constructors are a homomorphism. If
    /// one of these failed, the calculus and the algebra would disagree and
    /// the *specification* would be wrong.
    #[test]
    fn the_algebra_transports_to_terms(t in arb_term(1), u in arb_term(1), v in arb_term(1)) {
        let left = Term::seq(vec![Term::seq(vec![t.clone(), u.clone()]).expect("ne"), v.clone()]).expect("ne");
        let right = Term::seq(vec![t.clone(), Term::seq(vec![u.clone(), v.clone()]).expect("ne")]).expect("ne");
        prop_assert!(evaluate(&left).semantic_eq(&evaluate(&right)), "L1");

        let ab = Term::over(vec![t.clone(), u.clone()]).expect("ne");
        let ba = Term::over(vec![u.clone(), t.clone()]).expect("ne");
        prop_assert!(evaluate(&ab).semantic_eq(&evaluate(&ba)), "L4");

        let l = Term::over(vec![Term::over(vec![t.clone(), u.clone()]).expect("ne"), v.clone()]).expect("ne");
        let r = Term::over(vec![t, Term::over(vec![u, v]).expect("ne")]).expect("ne");
        prop_assert!(evaluate(&l).semantic_eq(&evaluate(&r)), "L5");
    }
}

/// L18 at the term level, with the synchronization the law requires. A
/// generated pair rarely has equal extents, so this is a worked example
/// rather than a property, exactly as the value-level suite does it.
#[test]
fn synchronized_interchange_holds_of_terms() {
    // Four sections of equal duration, which is the synchronization L18
    // requires — an unsynchronized pair is the counterexample, not the law.
    let section = |payload: u8| {
        Term::literal(
            timeline(
                quarters(4),
                vec![Occurrence::new(
                    Span::new(quarters(0), quarters(4)).expect("ordered"),
                    payload,
                )],
            )
            .expect("in bounds"),
        )
    };
    let (m, n, p, q) = (section(1), section(2), section(3), section(4));
    let by_section = Term::seq(vec![
        Term::over(vec![m.clone(), n.clone()]).expect("ne"),
        Term::over(vec![p.clone(), q.clone()]).expect("ne"),
    ])
    .expect("ne");
    let by_voice = Term::over(vec![
        Term::seq(vec![m, p]).expect("ne"),
        Term::seq(vec![n, q]).expect("ne"),
    ])
    .expect("ne");
    assert!(
        evaluate(&by_section).semantic_eq(&evaluate(&by_voice)),
        "L18 transports to terms through T1"
    );
}

/// The two rules `check` exists for, as negative tests: an unbound reference
/// and a shadowed binding are rejected, and the constructors reject what they
/// are supposed to make unrepresentable.
#[test]
fn ill_formed_terms_are_rejected() {
    let literal = || Term::literal(timeline::<u8>(quarters(4), vec![]).expect("empty"));

    assert!(Term::<u8>::var("nothing").check().is_err(), "a free name");
    let shadowed = Term::bind("x", literal(), Term::bind("x", literal(), Term::var("x")));
    assert!(shadowed.check().is_err(), "a shadowed binding");

    assert!(Term::<u8>::seq(vec![]).is_err(), "an empty seq");
    assert!(Term::<u8>::over(vec![]).is_err(), "an empty over");
    assert!(Term::scale(Ratio::new(0, 1), literal()).is_err(), "a zero factor");
    assert!(Term::scale(Ratio::new(-1, 2), literal()).is_err(), "a negative factor");
    assert!(Term::shift(quarters(-1), literal()).is_err(), "a backwards delay");

    let well_formed = Term::bind(
        "x",
        literal(),
        Term::seq(vec![Term::var("x"), Term::var("x")]).expect("ne"),
    );
    assert!(well_formed.check().is_ok(), "a name used twice is not shadowing");
}

/// `shift` is sugar, and the evaluator applies the stated expansion: a
/// delayed term denotes exactly the sequence after an empty timeline. If this
/// ever diverges, `shift` has quietly become a primitive.
#[test]
fn shift_denotes_its_stated_expansion() {
    let body = Term::literal(
        timeline(
            quarters(4),
            vec![Occurrence::new(
                Span::new(quarters(0), quarters(2)).expect("ordered"),
                7,
            )],
        )
        .expect("in bounds"),
    );
    let shifted = Term::shift(quarters(6), body.clone()).expect("non-negative");
    let expansion = Term::seq(vec![Term::literal(timeline(quarters(6), vec![]).expect("empty")), body]).expect("ne");
    assert!(
        evaluate(&shifted).semantic_eq(&evaluate(&expansion)),
        "shift d t = seq (timeline d {{}}) t"
    );
}
