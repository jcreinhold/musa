//! `docs/rules/language/02-core-calculus.md` §3's fifth strategy rule: a
//! recursor is strict in its target and lazy in its methods.
//!
//! The rule is a *cost* rule, so most of these laws are stated over
//! [`musa_calculus::normalize_metered`]'s spend rather than over an answer.
//! That is not a weaker statement than a law about values: the answers are
//! unchanged by construction — totality (§2.4) makes an unevaluated method and
//! an evaluated one the same value — and what the rule actually claims is about
//! what the meter charged. A law that only compared values could not fail if
//! the rule were deleted tomorrow.
//!
//! Two of the five are the other direction: the rule must not have moved a
//! *normal form*. A stuck recursor is read back with its methods forced, and ι
//! at a projection never had a method to delay.

use musa_calculus::{Budget, CoreError, Cx, Raw, RawArm, RawPattern, Spend, Term};

use crate::family_laws::{apply, nat_context, var};
use crate::programs::WRITTEN;

/// The successor of a `Nat` literal, as many times as asked.
fn number(count: u32) -> Raw {
    (0..count).fold(var("Nat.Zero"), |built, _| apply(var("Nat.Succ"), [built]))
}

/// A `match` over one subject.
fn matching(subject: Raw, arms: Vec<RawArm>) -> Raw {
    Raw::match_on(WRITTEN, [subject], arms)
}

/// One arm, over one constructor pattern with no fields bound by name.
fn nullary(constructor: &str, body: Raw) -> RawArm {
    RawArm {
        patterns: vec![RawPattern::constructor(WRITTEN, constructor, [])],
        body,
    }
}

/// One arm binding the successor's field.
fn successor(name: &str, body: Raw) -> RawArm {
    RawArm {
        patterns: vec![RawPattern::constructor(
            WRITTEN,
            "Nat.Succ",
            [RawPattern::bind(WRITTEN, name)],
        )],
        body,
    }
}

/// `Nat`, as a written type.
fn nat_type() -> Raw {
    var("Nat")
}

/// A term that costs real steps to evaluate and answers `Zero`.
///
/// A fold that counts `depth` levels down and answers with the floor. Written
/// rather than picked so that a law can ask for two of different size and
/// compare what they charged.
fn costly(depth: u32) -> Raw {
    applied(
        &Raw::lam(WRITTEN, "n", floor_of(var("n"))),
        &arrow(nat_type(), nat_type()),
        [number(depth)],
    )
}

/// `match n { Zero -> Zero, Succ(k) -> <the same, at k> }`, one level.
///
/// Not a recursive definition — the calculus has none (§2.4) — so this is one
/// elimination whose successor arm answers the field. Counting is what the
/// *argument* does; the cost this contributes is the elimination itself.
fn floor_of(subject: Raw) -> Raw {
    matching(
        subject,
        vec![nullary("Nat.Zero", var("Nat.Zero")), successor("k", var("k"))],
    )
}

fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(WRITTEN, "_", domain, codomain)
}

/// A function applied to arguments, with its own type written down.
fn applied(function: &Raw, ty: &Raw, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    apply(Raw::annot(WRITTEN, function.clone(), ty.clone()), arguments)
}

/// The core term and its type, for a written expression at `Nat`.
///
/// # Panics
///
/// If it does not check, which is a defect in the test that wrote it.
fn checked(cx: &Cx, name: &str, written: &Raw) -> (Term, Term) {
    let ty = musa_calculus::infer(cx, &nat_type())
        .unwrap_or_else(|error| panic!("{name}: Nat is a type: {error}"))
        .0;
    let term = musa_calculus::check(cx, &ty, written).unwrap_or_else(|error| panic!("{name}: {error}"));
    (term, ty)
}

/// What normalizing one written expression at `Nat` answered, and charged.
///
/// # Panics
///
/// If it does not check or does not normalize, either of which is a defect in
/// the test that wrote it.
fn spent(cx: &Cx, name: &str, written: &Raw) -> (Term, Spend) {
    let (term, ty) = checked(cx, name, written);
    musa_calculus::normalize_metered(cx, &ty, &term).unwrap_or_else(|error| panic!("{name}: {error}"))
}

/// §3, fifth rule: an arm that is not chosen is not evaluated.
///
/// Stated at the budget rather than at the clock. The unchosen arm alone
/// exhausts the narrowed budget, so a machine that evaluated it would answer
/// [`CoreError::Exhausted`] here; the rule is what makes the answer an answer.
/// Both halves are asserted, because a budget wide enough for the unchosen arm
/// would make the second half vacuous.
#[test]
fn an_arm_that_is_not_chosen_does_not_spend_what_it_would_have_cost() {
    let (cx, group) = nat_context();
    let (_, expensive) = spent(&cx, "the expensive arm on its own", &costly(220));
    // Steps alone, and narrowed until the expensive arm does not fit. Derived
    // from what that arm charged rather than picked, so a change in what a fold
    // costs cannot quietly make the law vacuous; `scaled` narrows every limit,
    // and the other two go back up where the language leaves them, because a
    // nesting limit of one would refuse the cheap arm for a reason that has
    // nothing to do with the rule.
    let narrowed = Budget::LANGUAGE
        .scaled(Budget::LANGUAGE.steps() / expensive.steps.max(2))
        .nesting(u64::MAX)
        .quoting(u64::MAX);
    // Elaborated in the wide context and normalized in the narrow one: what the
    // rule is about is evaluation, and a checker that ran out first would make
    // the law a law about checking.
    let narrow = Cx::with_budget(narrowed).declaring(&group);

    // The costly arm is the *nullary* one, and the target decides whether it is
    // chosen. A nullary constructor's method is a value, which is the whole of
    // why the rule was needed: an arm that binds a field is a λ and was lazy
    // already, so a law stated over one would hold with the rule deleted.
    let of = |name: &str, target: Raw| {
        let (term, ty) = checked(
            &cx,
            name,
            &matching(
                target,
                vec![nullary("Nat.Zero", costly(220)), successor("k", var("Nat.Zero"))],
            ),
        );
        musa_calculus::normalize_metered(&narrow, &ty, &term)
    };
    assert!(
        matches!(of("the costly arm chosen", number(0)), Err(CoreError::Exhausted(_))),
        "the narrowed budget must be too small for the costly arm, or the law below says nothing"
    );
    assert!(
        of("the costly arm unchosen", number(1)).is_ok(),
        "the arm that was not chosen was evaluated anyway"
    );
}

/// §3, fifth rule: what the chosen arm costs is what it always cost.
///
/// The rule removes the *other* arms and nothing else, so two eliminations that
/// choose the same arm charge the same however the arms they did not choose
/// differ. A machine that had gone lazy in the target, or that memoized an
/// answer rather than a method, would fail this in the direction the first law
/// cannot see.
#[test]
fn what_the_chosen_arm_costs_does_not_depend_on_the_arms_beside_it() {
    let (cx, _) = nat_context();
    let beside = |other: Raw| {
        matching(
            number(3),
            vec![nullary("Nat.Zero", var("Nat.Zero")), successor("k", other)],
        )
    };
    let (cheap_answer, cheap) = spent(&cx, "a cheap arm beside it", &beside(var("k")));
    let (costly_answer, dear) = spent(&cx, "a costly arm beside it", &beside(var("k")));
    assert_eq!(cheap.steps, dear.steps, "the same chosen arm charged two amounts");
    assert_eq!(cheap_answer, costly_answer, "the same chosen arm answered twice");

    let (_, with_a_costly_neighbour) = spent(
        &cx,
        "a costly arm not chosen",
        &matching(
            number(3),
            vec![nullary("Nat.Zero", costly(40)), successor("k", var("k"))],
        ),
    );
    let (_, alone) = spent(
        &cx,
        "the same arm chosen with a cheap neighbour",
        &matching(
            number(3),
            vec![nullary("Nat.Zero", var("Nat.Zero")), successor("k", var("k"))],
        ),
    );
    assert_eq!(
        alone.steps, with_a_costly_neighbour.steps,
        "an arm nobody chose was charged for"
    );
}

/// §3, fifth rule: a method forced once is forced once.
///
/// The memo, stated so that a machine without one fails. `Nat.elim` descends
/// one level per successor and meets its methods at each of them, on a spine
/// the induction hypothesis rebuilds — sharing the same delay. If forcing
/// re-evaluated the term instead of reading the memo, the extra cost of a
/// dearer method would *grow with the depth*. What the law asserts is that it
/// does not: the difference a dearer method makes is the same at two depths,
/// which is what "once" means when the fold is what varies.
#[test]
fn a_method_is_forced_once_however_deep_the_fold() {
    let (cx, _) = nat_context();
    // `λn. λih. ih` — the method that makes this a fold rather than a case
    // analysis: it answers the hypothesis, so ι fires again one level down.
    let descend = || Raw::lam(WRITTEN, "n", Raw::lam(WRITTEN, "ih", var("ih")));
    let method_type = arrow(nat_type(), arrow(nat_type(), nat_type()));
    // The same method, behind a redex that costs real steps to get through.
    // Evaluated once under the rule, once per level without the memo.
    let dear = applied(
        &Raw::lam(WRITTEN, "_", descend()),
        &arrow(nat_type(), method_type),
        [costly(30)],
    );
    let folded = |depth: u32, method: Raw| {
        apply(
            var("Nat.elim"),
            [
                Raw::lam(WRITTEN, "_", nat_type()),
                var("Nat.Zero"),
                method,
                number(depth),
            ],
        )
    };
    let extra = |depth: u32| {
        let (shallow, cheap) = spent(&cx, "a cheap method", &folded(depth, descend()));
        let (deep, borne) = spent(&cx, "a dear method", &folded(depth, dear.clone()));
        assert_eq!(shallow, deep, "the two methods answered differently at {depth}");
        borne.steps.saturating_sub(cheap.steps)
    };
    let (once, again) = (extra(1), extra(12));
    assert_eq!(
        once, again,
        "a dearer method cost {again} extra at depth 12 against {once} at depth 1, so it was forced more than once"
    );
    assert!(
        once > 0,
        "the dear method cost nothing extra, so the law compares two zeroes"
    );
}

/// §3, fifth rule: a stuck recursor reads back exactly as it did.
///
/// The second of the two places the rule names a delay as forced. The target
/// here is a variable, so ι never fires and the methods are never chosen — and
/// the normal form must still be the normal form, with each arm's body reduced.
/// A read-back that handed a delay onward unforced would write the redex.
#[test]
fn a_stuck_recursor_reads_back_with_its_methods_normalized() {
    let (cx, _) = nat_context();
    let over = |zero: Raw| {
        Raw::lam(
            WRITTEN,
            "n",
            matching(var("n"), vec![nullary("Nat.Zero", zero), successor("k", var("k"))]),
        )
    };
    let ty = musa_calculus::infer(&cx, &arrow(nat_type(), nat_type()))
        .expect("Nat → Nat is a type")
        .0;
    let redex = musa_calculus::check(
        &cx,
        &ty,
        &over(applied(
            &Raw::lam(WRITTEN, "z", var("z")),
            &arrow(nat_type(), nat_type()),
            [var("Nat.Zero")],
        )),
    )
    .expect("the stuck recursor with a redex in one arm checks");
    let plain = musa_calculus::check(&cx, &ty, &over(var("Nat.Zero"))).expect("the stuck recursor checks");
    assert_eq!(
        musa_calculus::normalize(&cx, &ty, &redex).expect("it normalizes"),
        musa_calculus::normalize(&cx, &ty, &plain).expect("it normalizes"),
        "a method left delayed reached the normal form unreduced"
    );
}

/// §3, fifth rule: ι at a projection is untouched.
///
/// An accessor takes the parameters and the value and has no method, so there
/// is nothing there to delay and the rule does not mention it. What this checks
/// is that nothing was delayed anyway: a projection out of a literal computes,
/// and computes to the field.
#[test]
fn a_projection_still_computes_to_the_field_it_reads() {
    let (cx, _) = nat_context();
    let pairs = musa_calculus::declare(&cx, &crate::record_laws::pair()).expect("Pair is a declaration");
    let cx = cx.declaring(&pairs);
    let literal = apply(var("Pair.Pair"), [nat_type(), number(2), number(5)]);
    let (read, _) = spent(
        &cx,
        "the second field of a literal",
        &Raw::project(WRITTEN, literal, "snd"),
    );
    let (expected, _) = spent(&cx, "the field itself", &number(5));
    assert_eq!(read, expected, "a projection did not compute to its field");

    // Selecting a large field must not construct that field a second time.
    // The pair and projection add a fixed amount of wiring around either
    // field, so making the selected field twenty times deeper must leave that
    // overhead unchanged.
    let overhead = |field| {
        let literal = apply(var("Pair.Pair"), [nat_type(), number(2), number(field)]);
        let (_, selected) = spent(
            &cx,
            "the second field of a sized literal",
            &Raw::project(WRITTEN, literal, "snd"),
        );
        let (_, alone) = spent(&cx, "the sized field itself", &number(field));
        (
            selected.constructed_nodes - alone.constructed_nodes,
            selected.logical_bytes - alone.logical_bytes,
        )
    };
    assert_eq!(
        overhead(5),
        overhead(100),
        "selecting a field adds wiring but does not charge the selected value again"
    );
}
