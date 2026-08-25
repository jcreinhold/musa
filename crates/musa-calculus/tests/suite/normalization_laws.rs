//! Normalization: that it terminates on the corpus, that it is deterministic
//! and stable, and that what it produces is the η-long normal form
//! `docs/rules/language/02-core-calculus.md` §3 describes rather than merely
//! *a* term with the same meaning.
//!
//! §5's obligations these partially discharge are named per test. What no
//! example-based suite can supply is §5's *strong* normalization — that every
//! well-typed term has a normal form — which prompt 169 owes as a proof
//! obligation and prompt 135's termination checker owes for the recursive
//! definitions this crate does not yet have.

use musa_calculus::{Binder, Cx, Index, Origin, Role, Shape, Sort, Term, normalize, normalize_type};

use crate::fixtures::{Sample, corpus};

/// As in `conversion_laws.rs`: origins are this file's *input*, not its subject.
const HERE: Origin = Origin::node(910);

/// §5, evaluation is deterministic.
///
/// The same term in the same context normalizes to the same term, every time.
/// A normalizer that consulted a hash map's iteration order, a clock, or an
/// address would fail this, and each of those is a real way to lose it.
#[test]
fn normalization_is_deterministic() {
    for Sample { name, cx, ty, left, .. } in corpus() {
        let once = normalize(&cx, &ty, &left).expect("the corpus normalizes");
        let twice = normalize(&cx, &ty, &left).expect("the corpus normalizes");
        assert_eq!(once, twice, "{name}");
    }
}

/// §3, normalization is stable: a normal form is its own normal form.
#[test]
fn normalization_is_idempotent() {
    for Sample { name, cx, ty, left, .. } in corpus() {
        let ty = normalize_type(&cx, &ty).expect("the corpus normalizes");
        let once = normalize(&cx, &ty, &left).expect("the corpus normalizes");
        let again = normalize(&cx, &ty, &once).expect("a normal form normalizes");
        assert_eq!(once, again, "{name}");
    }
}

/// §3, what comes back has no redex left in it.
///
/// Not a restatement of idempotence: a normalizer that stopped early but
/// stopped *consistently* would be idempotent and would still hand prompt 134's
/// elaborator a `let` to look through. This checks the shape directly — no
/// `let`, no lambda in function position, no projection of a literal, no `J` on
/// `refl`.
#[test]
fn a_normal_form_contains_no_redex() {
    for Sample {
        name,
        cx,
        ty,
        left,
        right,
        ..
    } in corpus()
    {
        for term in [&left, &right] {
            let normal = normalize(&cx, &ty, term).expect("the corpus normalizes");
            assert!(is_normal(&cx, &normal), "{name}: {normal:?} still contains a redex");
        }
    }
}

/// §3, η is performed by quotation, so a normal form at a Π *is* a lambda and a
/// normal form at a one-constructor family *is* that constructor applied to the
/// fields — whatever the input looked like.
#[test]
fn normal_forms_are_eta_long() {
    let cx = Cx::new();
    let a = cx.assume(HERE, &Term::universe(HERE, Sort::ZERO)).expect("A : Type 0");

    let arrow = Term::pi(HERE, "z", Term::var(HERE, Index(0)), Term::var(HERE, Index(1)));
    let f = a.assume(HERE, &arrow).expect("f : A → A");
    let arrow_in_f = Term::pi(HERE, "z", Term::var(HERE, Index(1)), Term::var(HERE, Index(2)));
    let function = normalize(&f, &arrow_in_f, &Term::var(HERE, Index(0))).expect("f normalizes");
    assert!(
        matches!(
            *function.shape(),
            Shape::Bind {
                binder: Binder::Lam,
                ..
            }
        ),
        "a variable at a function type reads back as a lambda, not as itself"
    );

    // `data Pair (A : Type 0) { Pair(fst: A, snd: A) }`, declared under `A`'s
    // binder rather than over it: a declaration is closed, and the parameter is
    // what carries the element type in.
    let group = musa_calculus::declare(&cx, &crate::record_laws::pair()).expect("Pair is a declaration");
    let a = cx.declaring(&group);
    let a = a.assume(HERE, &Term::universe(HERE, Sort::ZERO)).expect("A : Type 0");
    let pair_type = Term::app(
        HERE,
        Term::named(HERE, "Pair", Role::TypeConstructor),
        Term::var(HERE, Index(0)),
    );
    let r = a.assume(HERE, &pair_type).expect("r : Pair A");
    let pair_in_r = Term::app(
        HERE,
        Term::named(HERE, "Pair", Role::TypeConstructor),
        Term::var(HERE, Index(1)),
    );
    let record = normalize(&r, &pair_in_r, &Term::var(HERE, Index(0))).expect("r normalizes");
    assert_eq!(
        head_of(&record),
        Some(("Pair.Pair".to_owned(), 3)),
        "a variable at a one-constructor family reads back as that constructor over its projections"
    );
}

/// The name at the head of an application spine, and how many arguments it took.
fn head_of(term: &Term) -> Option<(String, usize)> {
    let mut head = term;
    let mut arguments = 0_usize;
    while let Shape::App { function, .. } = head.shape() {
        arguments = arguments.saturating_add(1);
        head = function;
    }
    match head.shape() {
        Shape::Named { name, .. } => Some((name.to_string(), arguments)),
        Shape::Meta(_)
        | Shape::MetaAt { .. }
        | Shape::Var(_)
        | Shape::Lit(_)
        | Shape::Universe(_)
        | Shape::Bind { .. }
        | Shape::App { .. } => None,
    }
}

/// §3, α-equivalent inputs have *identical* normal forms.
///
/// Stronger than "are convertible", and it is what makes conversion a `==`:
/// terms carry de Bruijn indices, so a binder name is not part of the term, and
/// quotation takes the name it writes from the Π it is quoting at.
#[test]
fn alpha_equivalent_terms_normalize_to_the_same_term() {
    let cx = Cx::new();
    let a = cx.assume(HERE, &Term::universe(HERE, Sort::ZERO)).expect("A : Type 0");
    let arrow = Term::pi(HERE, "z", Term::var(HERE, Index(0)), Term::var(HERE, Index(1)));

    let by_one_name = normalize(&a, &arrow, &Term::lam(HERE, "first", Term::var(HERE, Index(0)))).expect("normalizes");
    let by_another = normalize(&a, &arrow, &Term::lam(HERE, "second", Term::var(HERE, Index(0)))).expect("normalizes");
    assert_eq!(by_one_name, by_another);
    assert_eq!(
        by_one_name,
        Term::lam(HERE, "z", Term::var(HERE, Index(0))),
        "the name that survives is the function type's, which is the one a reader was shown"
    );
}

/// §3, δ: a definition is unfolded, whether it came from a `let` or from the
/// context.
#[test]
fn definitions_are_unfolded() {
    let cx = Cx::new();
    let type0 = Term::universe(HERE, Sort::ZERO);
    let a = cx.assume(HERE, &type0).expect("A : Type 0");
    let x = a.assume(HERE, &Term::var(HERE, Index(0))).expect("x : A");

    let through_let = normalize(
        &x,
        &Term::var(HERE, Index(1)),
        &Term::bind(
            HERE,
            "z",
            Term::var(HERE, Index(1)),
            Term::var(HERE, Index(0)),
            Term::var(HERE, Index(0)),
        ),
    )
    .expect("normalizes");
    assert_eq!(through_let, Term::var(HERE, Index(0)), "let x = v in x is v");

    let defined = x
        .define(&Term::var(HERE, Index(1)), &Term::var(HERE, Index(0)))
        .expect("d := x");
    let through_context =
        normalize(&defined, &Term::var(HERE, Index(2)), &Term::var(HERE, Index(0))).expect("normalizes");
    assert_eq!(
        through_context,
        Term::var(HERE, Index(1)),
        "a defined variable reads back as what it was defined to be"
    );
}

/// Whether `term` is a δ-redex: a builtin applied to as many literals as its
/// arity.
///
/// §5.8's D3 makes a δ-rule a function of its arguments alone, so this is the
/// whole of the condition — a builtin one argument short, or one whose argument
/// is still a variable, is stuck rather than reducible.
///
/// §5.8's *structural* eliminators are the other half of the family and are not
/// read here, because their condition is about one declared argument and which
/// one that is lives in the registration rather than in the term. The law over
/// them is stated where a registry exists to state it against: `base_laws.rs`'s
/// worked traversal, which normalizes to itself exactly when its target has not
/// become a literal.
///
/// The arity comes from the context rather than from the term: a term spells the
/// builtin's name and says it is one, and [`Cx::extern_named`] answers which
/// registration that name reached.
fn is_delta_redex(cx: &Cx, term: &Term) -> bool {
    let mut arguments = 0_usize;
    let mut every_argument_is_a_literal = true;
    let mut head = term;
    while let Shape::App { function, argument } = head.shape() {
        arguments = arguments.saturating_add(1);
        every_argument_is_a_literal &= matches!(*argument.shape(), Shape::Lit(_));
        head = function;
    }
    let Shape::Named {
        name,
        role: musa_calculus::Role::Builtin,
        ..
    } = head.shape()
    else {
        return false;
    };
    let Some(musa_calculus::Extern::Builtin(builtin)) = cx.extern_named(name) else {
        return false;
    };
    every_argument_is_a_literal && arguments >= builtin.arity()
}

/// Whether a term has no redex anywhere inside it.
///
/// Written out rather than matched with a wildcard: a variant added later must
/// make this fail to compile, because a new form of redex that nobody taught
/// this function about would silently pass every test above.
fn is_normal(cx: &Cx, term: &Term) -> bool {
    match term.shape() {
        // A base type, a literal, and a builtin are leaves. §5.8's D1 gives a
        // base type no eliminator, so nothing built from one is a redex; a
        // builtin applied to enough literals is, and that is an `App` whose
        // function is this leaf, which the `App` arm below already reads.
        Shape::Var(_) | Shape::Universe(_) | Shape::Named { .. } | Shape::Lit(_) => true,
        // A refinement has no elimination form, so it is never a redex; both
        // halves still have to be normal.
        // A `let` is a redex on sight; a Π and a λ are normal when what they
        // bind and what they hold are.
        Shape::Bind {
            binder: Binder::Let { .. },
            ..
        } => false,
        Shape::Bind { binder, body, .. } => binder.outer().all(|term| is_normal(cx, term)) && is_normal(cx, body),
        Shape::App { function, argument } => {
            !matches!(
                *function.shape(),
                Shape::Bind {
                    binder: Binder::Lam,
                    ..
                }
            ) && !is_delta_redex(cx, term)
                && is_normal(cx, function)
                && is_normal(cx, argument)
        }
        Shape::Meta(_) | Shape::MetaAt { .. } => false,
        // A normal form has none: elaboration either solved it or refused the
        // declaration that left it unsolved (§2.1). Reaching one here means a
        // term went to `normalize` before that happened.
    }
}

// ---- §3's conversion strategy ---------------------------------------------
//
// Four rules, all four implemented before prompt 142db and none of them written
// down, which is what that prompt changed: conversion is decidable either way,
// and these are what make it *cheap*. A checker that lost one would still be
// correct and would grind, so each is metered here rather than described.
//
// The **numeral** rule is pinned twice over: `numeral_laws.rs` holds that a
// numeral means the tower without costing like it, and the test below holds the
// half that belongs to conversion — that comparing two of them does not scale
// with the count.
//
// The fourth rule — **the meter is the backstop** — is metered in
// `budget_laws.rs`, which owns §4 and already states the part that matters here:
// under a narrowed budget conversion exhausts or agrees, and never reports
// exhaustion as two types disagreeing. Restating it against a second budget here
// would be a copy, not a second law.

/// The two contexts these tests measure against: `x : A` assumed, and `d := x`
/// defined beside it.
///
/// Returned together because every assertion below is a *comparison* of two
/// spends, and two spends are only comparable when the contexts they were
/// charged in are the same shape.
fn assumed_and_defined() -> (Cx, Term, Term, Term) {
    let cx = Cx::new();
    let type0 = Term::universe(HERE, Sort::ZERO);
    let a = cx.assume(HERE, &type0).expect("A : Type 0");
    let x = a.assume(HERE, &Term::var(HERE, Index(0))).expect("x : A");
    let defined = x
        .define(&Term::var(HERE, Index(1)), &Term::var(HERE, Index(0)))
        .expect("d := x");
    (
        defined,
        Term::var(HERE, Index(2)),
        Term::var(HERE, Index(1)),
        Term::var(HERE, Index(0)),
    )
}

/// §3: **rigid heads first** — a definition met by itself is not unfolded.
///
/// Measured against an *assumed* variable, which has no δ-rule to fire, so the
/// two spends can only agree if δ never fired for the definition either. A
/// checker that unfolded before comparing heads would pay more here and would
/// pay it once per definition in every type it ever compares.
#[test]
fn a_definition_met_by_itself_is_not_unfolded() {
    let (cx, ty, x, d) = assumed_and_defined();
    let (agreed, defined) = musa_calculus::convertible_metered(&cx, &ty, &d, &d).expect("d ≡ d");
    let (also, assumed) = musa_calculus::convertible_metered(&cx, &ty, &x, &x).expect("x ≡ x");
    assert!(agreed && also);
    assert_eq!(
        defined, assumed,
        "a definition compared with itself cost more than an assumed variable did, so δ fired"
    );
}

/// §3: **one side at a time** — a definition met by its own expansion opens one
/// side, and opens it once.
///
/// Exactly one step more than the no-unfold case above. Opening both sides would
/// be two, and that is the difference this measures: the rule is not "unfold as
/// little as possible" but "unfold one side, then ask again".
#[test]
fn one_side_is_opened_at_a_time() {
    let (cx, ty, x, d) = assumed_and_defined();
    let (_, neither) = musa_calculus::convertible_metered(&cx, &ty, &d, &d).expect("d ≡ d");
    let (agreed, one) = musa_calculus::convertible_metered(&cx, &ty, &d, &x).expect("d ≡ x");
    assert!(agreed);
    assert_eq!(
        one.steps,
        neither.steps.saturating_add(1),
        "opening the definition against its expansion cost {} steps against {}, which is not one unfold",
        one.steps,
        neither.steps
    );
}

/// §3: **a numeral is compared as a number**, so conversion does not scale with
/// the count.
///
/// Stated as equality across three counts three orders of magnitude apart rather
/// than as a bound, because the claim is not "cheap" but "flat": a checker that
/// expanded the tower would cost `count` steps, and one that expanded it lazily
/// would still cost more at 4,000 than at 4.
#[test]
fn a_numeral_is_compared_as_a_number_and_not_as_a_tower() {
    let (cx, _group) = crate::family_laws::nat_context();
    let nat = musa_calculus::infer(&cx, &crate::family_laws::var("Nat"))
        .expect("`Nat` is a type")
        .0;
    let spend_at = |count: u64| {
        let raw = musa_calculus::Raw::numeral(crate::programs::WRITTEN, "Nat", count);
        let term = musa_calculus::check(&cx, &nat, &raw).unwrap_or_else(|error| panic!("{count}: {error}"));
        musa_calculus::convertible_metered(&cx, &nat, &term, &term)
            .unwrap_or_else(|error| panic!("{count}: {error}"))
            .1
    };
    let small = spend_at(4);
    assert_eq!(spend_at(40), small, "comparing 40 with itself cost more than 4 did");
    assert_eq!(
        spend_at(4_000),
        small,
        "comparing 4,000 with itself cost more than 4 did"
    );
}
