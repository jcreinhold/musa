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

use musa_core::{Cx, Raw, RawArm, RawPattern, Refusal, Term};

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
    musa_core::infer(cx, ty)
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
    let elaborated = musa_core::check(&cx, &ty, &add()).expect("addition recurses on a field of its own match");
    // Independently re-checked, because a recursive definition is the one place
    // an elaborator could have emitted a term whose type it merely believed: the
    // rewrite into hypotheses is not a rule the re-checker knows, so what it
    // sees is an ordinary recursor application or nothing at all.
    musa_core::well_typed(&cx, &ty, &elaborated).expect("the compiled definition re-checks in the core");
    for (left, right) in [(0, 0), (0, 3), (2, 0), (2, 3), (4, 5)] {
        let name = "addition";
        let sum = musa_core::check(&cx, &nat, &applied(&add(), &written, [number(left), number(right)]))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected = musa_core::check(&cx, &nat, &number(left.saturating_add(right)))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            musa_core::convertible(&cx, &nat, &sum, &expected).unwrap_or_else(|error| panic!("{name}: {error}")),
            "{left} + {right}"
        );
    }
}

/// §2.4 over an *indexed* family, which is the case that decided the rule.
///
/// `count k ys` changes both of its arguments, and it has to: `ys : Vec {} k`,
/// so passing the tail without passing its length is not a program. A measure
/// that asked for one changed argument would therefore admit no recursion over
/// `Vec` at all — the reason the rule reads the recursive position off the
/// definition's own `match` and leaves every other argument to conversion.
#[test]
fn a_recursion_over_an_indexed_family_computes() {
    let cx = nat_vec_context();
    let unit_type = Raw::record_type(WRITTEN, []);
    let unit = Raw::record(WRITTEN, []);
    // `rec count : (n : Nat) → Vec {} n → Nat`, recursing on the vector.
    let ty = Raw::pi(
        WRITTEN,
        "n",
        var("Nat"),
        arrow(apply(var("Vec"), [unit_type.clone(), var("n")]), var("Nat")),
    );
    let count = Raw::rec(
        WRITTEN,
        "count",
        ty.clone(),
        Raw::lam(
            WRITTEN,
            "n",
            Raw::lam(
                WRITTEN,
                "xs",
                matching(
                    [var("xs")],
                    vec![
                        arm(vec![con("Vec.Nil", [])], var("Nat.Zero")),
                        arm(
                            vec![con("Vec.Cons", [bind("k"), bind("x"), bind("ys")])],
                            apply(var("Nat.Succ"), [apply(var("count"), [var("k"), var("ys")])]),
                        ),
                    ],
                ),
            ),
        ),
    );
    let written = ty.clone();
    let ty = core(&cx, "the counting function's type", &ty);
    musa_core::check(&cx, &ty, &count).expect("a recursion over a vector's tail is admitted");

    let nat = core_constant(&cx, "Nat");
    let mut vector = apply(var("Vec.Nil"), [unit_type.clone()]);
    for length in 0..4_u32 {
        let name = "counting a vector";
        let counted = musa_core::check(&cx, &nat, &applied(&count, &written, [number(length), vector.clone()]))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected = musa_core::check(&cx, &nat, &number(length)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            musa_core::convertible(&cx, &nat, &counted, &expected).unwrap_or_else(|error| panic!("{name}: {error}")),
            "a vector of {length} counted as something else"
        );
        vector = apply(
            var("Vec.Cons"),
            [unit_type.clone(), number(length), unit.clone(), vector],
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
            "a call whose arguments both changed",
            // §2.4 admits a lexicographic measure and this checker supplies
            // none, so a call that varies two arguments at once is named rather
            // than guessed at.
            Raw::rec(
                WRITTEN,
                "both",
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
                                    apply(var("both"), [var("x"), var("y")]),
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
        let Err(error) = musa_core::check(&cx, &ty, &raw) else {
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
    musa_core::check(&cx, &ty, &shadowed).expect("a shadowed name is not the definition");

    // `Type 0` in a definition that never calls itself: the rewrite leaves a
    // non-recursive body alone rather than demanding a match.
    let constant = Raw::rec(WRITTEN, "unused", type0(), Raw::record_type(WRITTEN, []));
    musa_core::check(&cx, &core(&cx, "Type 0", &type0()), &constant)
        .expect("a definition that does not recurse is an ordinary term");
}
