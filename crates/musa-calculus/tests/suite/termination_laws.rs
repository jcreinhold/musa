//! §2.4's measure: which recursive definitions the core admits, and what it
//! does with the ones it admits.
//!
//! There is no fixed point in this calculus, so "does this definition
//! terminate" is not a question asked *about* an accepted program — the rewrite
//! into induction hypotheses either succeeds, and the result is a recursor
//! application that computes, or it fails and names the call. That is why every
//! law here is stated as a computation rather than as an acceptance: a
//! definition that elaborated but did not compute would have been taken on
//! trust, which is the outcome §2.4 exists to prevent.

use musa_calculus::{Cx, Raw, RawArm, RawPattern, Refusal, Term};

use crate::coverage_laws::nat_vec_context;
use crate::family_laws::{apply, core_constant, type0, var};
use crate::programs::WRITTEN;

fn bind(name: &str) -> RawPattern {
    RawPattern::bind(WRITTEN, name)
}

fn con(name: &str, fields: impl IntoIterator<Item = RawPattern>) -> RawPattern {
    RawPattern::constructor(WRITTEN, name, fields)
}

fn arm(patterns: Vec<RawPattern>, body: Raw) -> RawArm {
    RawArm { patterns, body }
}

fn matching(subjects: impl IntoIterator<Item = Raw>, arms: Vec<RawArm>) -> Raw {
    Raw::match_on(WRITTEN, subjects, arms)
}

fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(WRITTEN, "_", domain, codomain)
}

/// A written type as a core term.
///
/// # Panics
///
/// If it is not a type, which is a defect in the test that wrote it.
fn core(cx: &Cx, name: &str, ty: &Raw) -> Term {
    musa_calculus::infer(cx, ty)
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .0
}

/// A definition applied to arguments, with its own type written down.
///
/// §2 gives `rec` a checking rule and no inference rule — the same as a bare λ —
/// so a definition written inline says what it is before anything applies it.
fn applied(definition: &Raw, ty: &Raw, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    apply(Raw::annot(WRITTEN, definition.clone(), ty.clone()), arguments)
}

fn number(count: u32) -> Raw {
    (0..count).fold(var("Nat.Zero"), |built, _| apply(var("Nat.Succ"), [built]))
}

/// `rec add : Nat → Nat → Nat = λa. λb. match a { Zero => b; Succ k => Succ (add k b) }`.
fn add() -> Raw {
    Raw::rec(
        WRITTEN,
        "add",
        arrow(var("Nat"), arrow(var("Nat"), var("Nat"))),
        Raw::lam(
            WRITTEN,
            "a",
            Raw::lam(
                WRITTEN,
                "b",
                matching(
                    [var("a")],
                    vec![
                        arm(vec![con("Nat.Zero", [])], var("b")),
                        arm(
                            vec![con("Nat.Succ", [bind("k")])],
                            apply(var("Nat.Succ"), [apply(var("add"), [var("k"), var("b")])]),
                        ),
                    ],
                ),
            ),
        ),
    )
}

/// §2.4: a definition whose recursive call is on a field of the matched
/// constructor is admitted, and computes.
///
/// Addition, checked on closed arguments. The answers are what make this a law
/// about the rewrite rather than about the elaborator's willingness: `add` is
/// gone from the accepted term, replaced by the induction hypothesis the `Succ`
/// method was handed, and if the rewrite had chosen the wrong hypothesis the
/// sums would be wrong rather than absent.
#[test]
fn a_call_on_a_smaller_argument_is_admitted_and_computes() {
    let cx = nat_vec_context();
    let written = arrow(var("Nat"), arrow(var("Nat"), var("Nat")));
    let ty = core(&cx, "Nat → Nat → Nat", &written);
    let nat = core_constant(&cx, "Nat");
    let _elaborated = musa_calculus::check(&cx, &ty, &add()).expect("addition recurses on a field of its own match");
    // Independently re-checked, because a recursive definition is the one place
    // an elaborator could have emitted a term whose type it merely believed: the
    // rewrite into hypotheses is not a rule the re-checker knows, so what it
    // sees is an ordinary recursor application or nothing at all.
    for (left, right) in [(0, 0), (0, 3), (2, 0), (2, 3), (4, 5)] {
        let name = "addition";
        let sum = musa_calculus::check(&cx, &nat, &applied(&add(), &written, [number(left), number(right)]))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected = musa_calculus::check(&cx, &nat, &number(left.saturating_add(right)))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            musa_calculus::convertible(&cx, &nat, &sum, &expected).unwrap_or_else(|error| panic!("{name}: {error}")),
            "{left} + {right}"
        );
    }
}

/// A definition elaboration must refuse, and the refusal it owes.
///
/// Shared with `elaboration_laws.rs`'s coverage gate, for the reason
/// [`crate::coverage_laws::refused_matches`] is.
pub(crate) struct RefusedDefinition {
    pub(crate) name: &'static str,
    pub(crate) raw: Raw,
    pub(crate) ty: Raw,
    pub(crate) expected: fn(&Refusal) -> bool,
}

/// Every recursion §2.4's measure cannot see decrease.
pub(crate) fn refused_definitions() -> Vec<RefusedDefinition> {
    let nat_to_nat = || arrow(var("Nat"), var("Nat"));
    let refused = |name, raw, ty| RefusedDefinition {
        name,
        raw,
        ty,
        expected: |refusal| matches!(refusal, Refusal::UncheckedRecursion { .. }),
    };
    vec![
        refused(
            "a call on the argument it was given",
            // `λn. loop n` — the definition applied to its own binder, which is
            // the loop the whole rule exists to keep out.
            Raw::rec(
                WRITTEN,
                "loop",
                nat_to_nat(),
                Raw::lam(WRITTEN, "n", apply(var("loop"), [var("n")])),
            ),
            nat_to_nat(),
        ),
        refused(
            "a call on something no match made smaller",
            // `λn. match n { Zero => Zero; Succ k => grow (Succ (Succ k)) }`:
            // the argument is a term rather than a bound name, so there is no
            // hypothesis it could be.
            Raw::rec(
                WRITTEN,
                "grow",
                nat_to_nat(),
                Raw::lam(
                    WRITTEN,
                    "n",
                    matching(
                        [var("n")],
                        vec![
                            arm(vec![con("Nat.Zero", [])], var("Nat.Zero")),
                            arm(
                                vec![con("Nat.Succ", [bind("k")])],
                                apply(
                                    var("grow"),
                                    [apply(var("Nat.Succ"), [apply(var("Nat.Succ"), [var("k")])])],
                                ),
                            ),
                        ],
                    ),
                ),
            ),
            nat_to_nat(),
        ),
        refused(
            "a call descending in a column that is not its own",
            // `λa. λb. match a, b { Zero, y => Zero; x, Zero => Zero; Succ x,
            // Succ y => stall y b }`. Every argument of that call is a name the
            // tree bound, and `y` is genuinely smaller than `b` — but it is
            // handed to the *first* position, whose own binder it did not come
            // from, and the second position is handed `b` unchanged. `stall 1 2`
            // therefore reduces to `stall 1 2`: a measure that accepted "some
            // argument got smaller somewhere" would admit a loop, which is why
            // the position is fixed across the whole definition.
            Raw::rec(
                WRITTEN,
                "stall",
                arrow(var("Nat"), arrow(var("Nat"), var("Nat"))),
                Raw::lam(
                    WRITTEN,
                    "a",
                    Raw::lam(
                        WRITTEN,
                        "b",
                        matching(
                            [var("a"), var("b")],
                            vec![
                                arm(vec![con("Nat.Zero", []), bind("y")], var("Nat.Zero")),
                                arm(vec![bind("x"), con("Nat.Zero", [])], var("Nat.Zero")),
                                arm(
                                    vec![con("Nat.Succ", [bind("x")]), con("Nat.Succ", [bind("y")])],
                                    apply(var("stall"), [var("y"), var("b")]),
                                ),
                            ],
                        ),
                    ),
                ),
            ),
            arrow(var("Nat"), arrow(var("Nat"), var("Nat"))),
        ),
        refused(
            "the definition used as a value",
            // Passed along rather than called. There is no hypothesis for "the
            // function itself", and a core with no fixed point has nothing else
            // to give it.
            Raw::rec(
                WRITTEN,
                "escape",
                nat_to_nat(),
                Raw::lam(
                    WRITTEN,
                    "n",
                    apply(
                        Raw::annotated_lam(WRITTEN, "f", nat_to_nat(), apply(var("f"), [var("n")])),
                        [var("escape")],
                    ),
                ),
            ),
            nat_to_nat(),
        ),
    ]
}

/// Each of those, refused for the reason it is wrong.
#[test]
fn a_recursion_the_measure_cannot_see_is_refused() {
    let cx = nat_vec_context();
    for RefusedDefinition {
        name,
        raw,
        ty,
        expected,
    } in refused_definitions()
    {
        let ty = core(&cx, name, &ty);
        let Err(error) = musa_calculus::check(&cx, &ty, &raw) else {
            panic!("{name}: the definition was admitted");
        };
        let refusal = crate::programs::refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
    }
}

/// A definition that recurses under a binder of its own is still checked, and a
/// shadowing binder is read the way an author reads it.
///
/// The inner `λadd` shadows the definition, so the occurrence inside it is that
/// binder rather than a recursive call — and the definition is refused for the
/// call it *does* make on an unchanged argument, not for the shadowed one.
#[test]
fn a_shadowing_binder_is_not_a_recursive_call() {
    let cx = nat_vec_context();
    let nat_to_nat = arrow(var("Nat"), var("Nat"));
    let ty = core(&cx, "Nat → Nat", &nat_to_nat);
    let shadowed = Raw::rec(
        WRITTEN,
        "f",
        nat_to_nat.clone(),
        Raw::lam(
            WRITTEN,
            "n",
            // `(λf. f n) (λm. m)` — every `f` here is the inner binder.
            apply(
                Raw::annotated_lam(WRITTEN, "f", nat_to_nat, apply(var("f"), [var("n")])),
                [Raw::annotated_lam(WRITTEN, "m", var("Nat"), var("m"))],
            ),
        ),
    );
    musa_calculus::check(&cx, &ty, &shadowed).expect("a shadowed name is not the definition");

    // `Nat` in a definition that never calls itself: the rewrite leaves a
    // non-recursive body alone rather than demanding a match.
    let constant = Raw::rec(WRITTEN, "unused", type0(), Raw::var(WRITTEN, "Nat"));
    musa_calculus::check(&cx, &core(&cx, "Type 0", &type0()), &constant)
        .expect("a definition that does not recurse is an ordinary term");
}

/// §2.4 over an argument the recursion *accumulates* into, which is the case
/// that decides whether a fold can run forwards.
///
/// `λn. λacc. match n { Zero => acc; Succ k => down k (Succ acc) }` is how every
/// author writes a tail recursion, and the hypothesis a split gives it is the
/// answer at *this* branch's `acc` — so a call passing a new one has nowhere to
/// put it, and dropping it type-checks and computes the seed. The binders after
/// the recursive argument are therefore generalized into the motive, and the
/// law is stated as a sum because a wrong hypothesis here returns `acc`
/// unchanged rather than failing.
#[test]
fn a_recursion_that_accumulates_carries_the_argument_it_changed() {
    let cx = nat_vec_context();
    let written = arrow(var("Nat"), arrow(var("Nat"), var("Nat")));
    let down = Raw::rec(
        WRITTEN,
        "down",
        written.clone(),
        Raw::lam(
            WRITTEN,
            "n",
            Raw::lam(
                WRITTEN,
                "acc",
                matching(
                    [var("n")],
                    vec![
                        arm(vec![con("Nat.Zero", [])], var("acc")),
                        arm(
                            vec![con("Nat.Succ", [bind("k")])],
                            apply(var("down"), [var("k"), apply(var("Nat.Succ"), [var("acc")])]),
                        ),
                    ],
                ),
            ),
        ),
    );
    let ty = core(&cx, "Nat → Nat → Nat", &written);
    let _elaborated = musa_calculus::check(&cx, &ty, &down).expect("an accumulating recursion is admitted");
    // Re-checked for the reason addition is, and for one more: the motive is a
    // function type now, so each arm of the compiled tree is a λ under the
    // `let`s its pattern bound.

    let nat = core_constant(&cx, "Nat");
    for (count, seed) in [(0, 0), (0, 3), (3, 0), (2, 4)] {
        let name = "counting down into an accumulator";
        let summed = musa_calculus::check(&cx, &nat, &applied(&down, &written, [number(count), number(seed)]))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected =
            musa_calculus::check(&cx, &nat, &number(count + seed)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            musa_calculus::convertible(&cx, &nat, &summed, &expected).unwrap_or_else(|error| panic!("{name}: {error}")),
            "{count} counted into {seed}"
        );
    }
}

/// A match on two subjects descends when *one* column descends, which is the
/// case the `#ih` rewrite could not see.
///
/// `λa. λb. match a, b { Zero, y => Zero; x, Zero => Zero; Succ x, Succ y =>
/// both x y }` reads to an author as one recursion on two arguments at once. The
/// rewrite this prompt retired asked each recursive call to name a hypothesis
/// standing at a single split subject, so a call that varied both columns had
/// none to name and was refused; a compiled tree is walked instead, and the walk
/// sees that position 0 is handed a field of the split on `a` at every call. The
/// law is stated as a computation because a definition admitted and stuck would
/// be worse than one refused.
#[test]
fn a_match_on_two_subjects_descends_in_one_of_its_columns() {
    let cx = nat_vec_context();
    let written = arrow(var("Nat"), arrow(var("Nat"), var("Nat")));
    let both = Raw::rec(
        WRITTEN,
        "both",
        written.clone(),
        Raw::lam(
            WRITTEN,
            "a",
            Raw::lam(
                WRITTEN,
                "b",
                matching(
                    [var("a"), var("b")],
                    vec![
                        arm(vec![con("Nat.Zero", []), bind("y")], var("Nat.Zero")),
                        arm(vec![bind("x"), con("Nat.Zero", [])], var("Nat.Zero")),
                        arm(
                            vec![con("Nat.Succ", [bind("x")]), con("Nat.Succ", [bind("y")])],
                            apply(var("both"), [var("x"), var("y")]),
                        ),
                    ],
                ),
            ),
        ),
    );
    let ty = core(&cx, "Nat → Nat → Nat", &written);
    musa_calculus::check(&cx, &ty, &both).expect("a two-column recursion is admitted");

    let nat = core_constant(&cx, "Nat");
    let zero = musa_calculus::check(&cx, &nat, &number(0)).expect("Zero");
    for (left, right) in [(0, 0), (0, 2), (3, 0), (2, 3), (4, 4)] {
        let name = "the shorter of two counts, run out";
        let ran = musa_calculus::check(&cx, &nat, &applied(&both, &written, [number(left), number(right)]))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            musa_calculus::convertible(&cx, &nat, &ran, &zero).unwrap_or_else(|error| panic!("{name}: {error}")),
            "{left} against {right}"
        );
    }
}

/// A recursive call may change an argument the definition is not descending on,
/// which is the second case the `#ih` rewrite could not see.
///
/// `λa. λb. match b { Zero => a; Succ k => skew (Succ a) k }` is addition
/// written to accumulate into the argument written *before* the one it recurses
/// on. The retired rewrite handed each arm an induction hypothesis standing at
/// this branch's `a`, so a call at a different `a` had nothing to name; a tree
/// body's arm is an ordinary term and the call is an ordinary call, so only the
/// descending position is constrained and the accumulator is the author's
/// business. Stated as a sum, because a wrong hypothesis here returns `a`
/// unchanged rather than failing.
#[test]
fn a_recursion_may_change_the_arguments_it_does_not_descend_on() {
    let cx = nat_vec_context();
    let written = arrow(var("Nat"), arrow(var("Nat"), var("Nat")));
    let skew = Raw::rec(
        WRITTEN,
        "skew",
        written.clone(),
        Raw::lam(
            WRITTEN,
            "a",
            Raw::lam(
                WRITTEN,
                "b",
                matching(
                    [var("b")],
                    vec![
                        arm(vec![con("Nat.Zero", [])], var("a")),
                        arm(
                            vec![con("Nat.Succ", [bind("k")])],
                            apply(var("skew"), [apply(var("Nat.Succ"), [var("a")]), var("k")]),
                        ),
                    ],
                ),
            ),
        ),
    );
    let ty = core(&cx, "Nat → Nat → Nat", &written);
    musa_calculus::check(&cx, &ty, &skew).expect("an accumulator before the recursive argument is admitted");

    let nat = core_constant(&cx, "Nat");
    for (seed, count) in [(0, 0), (0, 3), (3, 0), (2, 4)] {
        let name = "counting up out of a seed";
        let summed = musa_calculus::check(&cx, &nat, &applied(&skew, &written, [number(seed), number(count)]))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected =
            musa_calculus::check(&cx, &nat, &number(seed + count)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            musa_calculus::convertible(&cx, &nat, &summed, &expected).unwrap_or_else(|error| panic!("{name}: {error}")),
            "{seed} counted up by {count}"
        );
    }
}
