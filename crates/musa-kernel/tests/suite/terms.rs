//! The term-calculus theorem suite (docs/rules/kernel/10-term-calculus.md T1–T5),
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
        let (lv, rv) = (evaluate(left.clone()), evaluate(right.clone()));

        let seq_term = Term::seq(vec![left.clone(), right.clone()]).expect("non-empty");
        prop_assert!(evaluate(seq_term).semantic_eq(&sequence(vec![lv.clone(), rv.clone()])));

        let over_term = Term::over(vec![left.clone(), right]).expect("non-empty");
        prop_assert!(evaluate(over_term).semantic_eq(&overlay(vec![lv.clone(), rv])));

        let factor = Ratio::new(n, d);
        let scaled = Term::scale(factor, left.clone()).expect("positive");
        prop_assert!(evaluate(scaled).semantic_eq(&lv.scale(factor).expect("positive")));

        let window = Span::new(quarters(a.min(b)), quarters(a.max(b))).expect("ordered");
        let restricted = Term::restrict(window, left);
        prop_assert!(evaluate(restricted).semantic_eq(&observed_value(&lv, window)));
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
        prop_assert!(evaluate(shared).semantic_eq(&evaluate(expanded)));
    }

    /// T3 — evaluation is normalization: normalizing an evaluated term is the
    /// canonical form, and two terms are semantically equal exactly when
    /// those canonical values agree. Equal canonical values have equal framed
    /// hashes; digest equality alone is not used as a converse.
    #[test]
    fn evaluation_agrees_with_normalization(left in arb_term(2), right in arb_term(2)) {
        let (lv, rv) = (evaluate(left), evaluate(right));
        prop_assert!(lv.normalize().semantic_eq(&lv), "normalize preserves meaning");
        prop_assert_eq!(
            lv.semantic_eq(&rv),
            lv.normalize() == rv.normalize(),
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
        let value = evaluate(term.clone());
        prop_assert!(value.extent() >= Beat::ZERO);
        // Deterministic: the rules are syntax-directed, one per form.
        prop_assert!(evaluate(term).semantic_eq(&value));
    }

    /// T5 — observation commutes with sharing: a restriction may be pushed
    /// through a binding without changing the answer. This is what makes
    /// deferred observation sound.
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
        prop_assert!(evaluate(outside).semantic_eq(&evaluate(inside)));
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
        prop_assert!(evaluate(left).semantic_eq(&evaluate(right)), "L1");

        let ab = Term::over(vec![t.clone(), u.clone()]).expect("ne");
        let ba = Term::over(vec![u.clone(), t.clone()]).expect("ne");
        prop_assert!(evaluate(ab).semantic_eq(&evaluate(ba)), "L4");

        let l = Term::over(vec![Term::over(vec![t.clone(), u.clone()]).expect("ne"), v.clone()]).expect("ne");
        let r = Term::over(vec![t, Term::over(vec![u, v]).expect("ne")]).expect("ne");
        prop_assert!(evaluate(l).semantic_eq(&evaluate(r)), "L5");
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
        evaluate(by_section).semantic_eq(&evaluate(by_voice)),
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
        evaluate(shifted).semantic_eq(&evaluate(expansion)),
        "shift d t = seq (timeline d {{}}) t"
    );
}

/// A payload for the interchange tests: text that exercises the escaping the
/// format has to survive — quotes, backslashes, and the keywords the grammar
/// reserves.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Awkward(String);

impl musa_kernel::Canonical for Awkward {
    const OWNER_TYPE_ID: &'static str = "musa.kernel.tests.Awkward";
    const QUOTIENT_VERSION: u32 = 1;

    fn canonical_key(&self) -> String {
        self.0.clone()
    }
}

impl musa_kernel::PayloadText for Awkward {
    fn to_text(&self) -> String {
        self.0.clone()
    }

    fn from_text(text: &str) -> Option<Self> {
        Some(Self(text.to_owned()))
    }
}

impl musa_kernel::TextPayload for Awkward {
    fn type_name() -> &'static str {
        "Awkward"
    }
}

fn awkward_term() -> Term<Awkward> {
    let payloads = [
        r#"a "quoted" name"#,
        r"a\backslash",
        "from to timeline occurrence let in;",
        "",
    ];
    let occurrences = payloads
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let at = i64::try_from(index).expect("small");
            let span = Span::new(quarters(at), quarters(at + 1)).expect("ordered");
            Occurrence::new(span, Awkward((*text).to_owned()))
        })
        .collect();
    let subject = Term::literal(timeline(quarters(4), occurrences).expect("in bounds"));
    // A canon: the subject stated once and entered twice, which is the shape
    // the format exists to express.
    Term::bind(
        "subject",
        subject,
        Term::over(vec![
            Term::var("subject"),
            Term::shift(quarters(2), Term::var("subject")).expect("non-negative"),
            Term::scale(Ratio::new(3, 2), Term::var("subject")).expect("positive"),
        ])
        .expect("ne"),
    )
}

/// The round-trip law: printing a term and reading it back yields the same
/// canonical form and the same semantic hash. Payload escaping is what this
/// catches — real payloads contain the characters a generator will not think
/// of.
#[test]
fn printing_and_parsing_a_term_preserves_its_meaning() {
    let term = awkward_term();
    let text = musa_kernel::print("awkward", &term, &[]);
    let document = musa_kernel::parse::<Awkward>(&text).expect("its own output parses");
    assert_eq!(document.name(), "awkward");
    let parsed = document.into_term();
    assert!(parsed.check().is_ok(), "its own output is well formed");
    let (before, after) = (evaluate(term.clone()), evaluate(parsed.clone()));
    assert!(before.semantic_eq(&after), "the round trip preserves meaning");
    assert_eq!(before.semantic_hash(), after.semantic_hash());
    assert_eq!(parsed, term, "and the structure, not only the denotation");
}

/// Sharing survives the round trip *as sharing*: the printed text says `let`
/// once rather than repeating the material. This is the property that makes
/// the format worth having.
#[test]
fn printed_kernel_text_shares_rather_than_repeats() {
    let text = musa_kernel::print("awkward", &awkward_term(), &[]);
    assert_eq!(text.matches("let subject =").count(), 1);
    let occurrence_lines = text
        .lines()
        .filter(|line| line.trim_start().starts_with("occurrence "))
        .count();
    assert_eq!(occurrence_lines, 4, "the subject is written once, not three times");
    assert!(
        text.contains(musa_kernel::FORMAT_VERSION),
        "the header names the version"
    );
}

/// Kernel text that is not a term is rejected with a byte offset rather than
/// silently producing an empty piece.
#[test]
fn malformed_kernel_text_is_rejected() {
    for text in [
        "",
        "kernel \"x\" {",
        "kernel \"x\" { composition main : Timeline[Nope] = timeline 0 {}; }",
        "kernel \"x\" { composition main : Timeline[Awkward] = scale by 0 timeline 0 {}; }",
        "kernel \"x\" { composition main : Timeline[Awkward] = timeline 0 {}; } trailing",
    ] {
        assert!(
            musa_kernel::parse::<Awkward>(text).is_err(),
            "`{text}` is not a kernel file"
        );
    }
}

/// T6 — a mark changes payloads and nothing else.
///
/// The strong form: the marked term and the same term with its marks erased
/// denote timelines with identical extents and identical spans in canonical
/// order. Only the payloads differ, and they differ exactly where the mark
/// said they would.
#[test]
fn a_mark_changes_payloads_and_nothing_else() {
    let at = |a: i64, b: i64| Span::new(quarters(a), quarters(b)).expect("ordered");
    let body = Term::literal(
        timeline(
            quarters(2),
            vec![Occurrence::new(at(0, 1), 1_u32), Occurrence::new(at(1, 2), 2_u32)],
        )
        .expect("in bounds"),
    );
    // Three uses of one body, each marked with what distinguishes it.
    let uses = Term::seq(vec![
        Term::var_marked("subject", "10"),
        Term::var_marked("subject", "20"),
        Term::var("subject"),
    ])
    .expect("non-empty");
    let marked = Term::bind("subject", body.clone(), uses);

    let erased = Term::bind(
        "subject",
        body,
        Term::seq(vec![Term::var("subject"), Term::var("subject"), Term::var("subject")]).expect("non-empty"),
    );

    let with = musa_kernel::evaluate_marked(marked, |mark, instance| {
        let bump: u32 = mark.parse().unwrap_or(0);
        for payload in instance.payloads_mut() {
            *payload += bump;
        }
    });
    let without = musa_kernel::evaluate(erased);

    assert_eq!(with.extent(), without.extent(), "a mark moved the extent");
    // Storage order, not canonical order: `u32` is not `Canonical`, and the
    // claim is about *where* occurrences sit, which storage order already
    // pins because both terms have the same shape.
    let spans_of = |value: &musa_kernel::Timeline<u32>| -> Vec<(Beat, Beat)> {
        value
            .occurrences()
            .iter()
            .map(|occurrence| (occurrence.span().start(), occurrence.span().end()))
            .collect()
    };
    assert_eq!(spans_of(&with), spans_of(&without), "a mark moved an occurrence");

    // And the payloads did change, exactly as the marks said — otherwise the
    // assertions above would hold vacuously.
    let mut payloads: Vec<u32> = with.occurrences().iter().map(|o| *o.payload()).collect();
    payloads.sort_unstable();
    assert_eq!(payloads, vec![1, 2, 11, 12, 21, 22]);
}

/// An unmarked reference is E-Var: `evaluate` and `evaluate_marked` with a
/// hook that is never reached agree on every generated term.
#[test]
fn unmarked_terms_evaluate_identically_either_way() {
    proptest!(|(term in arb_term(3))| {
        let plain = musa_kernel::evaluate(term.clone());
        // `arb_term` builds no marked references, so the hook must never fire;
        // a counter says so without a panic the lints would object to.
        let mut reached = 0_u32;
        let hooked = musa_kernel::evaluate_marked(term, |_, _| reached = reached.saturating_add(1));
        prop_assert_eq!(reached, 0);
        prop_assert!(plain.semantic_eq(&hooked));
    });
}

/// A marked reference survives the interchange format: the mark is written,
/// read back, and still names the same payload map.
#[test]
fn a_mark_round_trips_through_kernel_text() {
    let body = Term::literal(
        timeline(
            quarters(1),
            vec![Occurrence::new(
                Span::new(quarters(0), quarters(1)).expect("ordered"),
                Awkward("a \"quoted\" payload".to_owned()),
            )],
        )
        .expect("in bounds"),
    );
    let term = Term::bind(
        "subject",
        body,
        Term::seq(vec![
            Term::var_marked("subject", "repeat:0"),
            Term::var_marked("subject", "invert:c4\\|d4"),
        ])
        .expect("non-empty"),
    );
    let text = musa_kernel::print("marked", &term, &[]);
    let document = musa_kernel::parse::<Awkward>(&text).expect("parses");
    assert_eq!(document.name(), "marked");
    assert_eq!(*document.term(), term, "the marks did not survive the round trip");
}
