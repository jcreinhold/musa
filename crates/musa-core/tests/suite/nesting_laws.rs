//! §1.1's positivity rule at the shape it actually admits: a family may hold a
//! *container* of itself.
//!
//! `data StaffRead { Sung, Body(items : List StaffRead) }` is the program these
//! laws are about. §1.1 states one prohibition — "a recursive occurrence may not
//! appear to the left of an arrow at any depth" — and `List StaffRead` puts the
//! occurrence nowhere near an arrow, so the declaration is admitted. What makes
//! it a law rather than a spelling is that the answer depends on `List`: the
//! same shape written over a family that is *not* positive in its parameter is
//! refused, and the two are told apart by what each container's own declaration
//! computed about itself.
//!
//! # Why the containers are declared here
//!
//! For `collection_laws.rs`'s reason, restated: the core is a calculus with no
//! base types, so `List`, `Option`, and `Result` are library declarations rather
//! than items of this crate. Declaring them in the suite that needs them is also
//! what lets this one add `Cont` — a container that is *not* positive in its
//! parameter — which no library would ship and which is the only way to state
//! that the flag is consulted rather than assumed.

use musa_core::{Cx, Raw, RawArm, RawData, RawPattern, Refusal, Term};

use crate::family_laws::{apply, binder, constructor, data, family, nat_context, type0, var};
use crate::programs::{WRITTEN, refusal};

fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(WRITTEN, "_", domain, codomain)
}

fn lam(name: &'static str, body: Raw) -> Raw {
    Raw::lam(WRITTEN, name, body)
}

fn bind(name: &str) -> RawPattern {
    RawPattern::bind(WRITTEN, name)
}

fn con(name: &str, fields: impl IntoIterator<Item = RawPattern>) -> RawPattern {
    RawPattern::constructor(WRITTEN, name, fields)
}

fn arm(patterns: Vec<RawPattern>, body: Raw) -> RawArm {
    RawArm { patterns, body }
}

fn matching(subject: Raw, arms: Vec<RawArm>) -> Raw {
    Raw::match_on(WRITTEN, [subject], arms)
}

/// `data List (A : Type 0) { Nil, Cons(head : A, tail : List A) }`.
fn lists() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "List",
            vec![
                constructor("Nil", Vec::new()),
                constructor(
                    "Cons",
                    vec![binder("head", var("A")), binder("tail", apply(var("List"), [var("A")]))],
                ),
            ],
        )],
    )
}

/// `data Option (A : Type 0) { None, Some(value : A) }`.
fn options() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "Option",
            vec![
                constructor("None", Vec::new()),
                constructor("Some", vec![binder("value", var("A"))]),
            ],
        )],
    )
}

/// `data Result (E : Type 0) (A : Type 0) { Err(error : E), Ok(value : A) }`.
///
/// Two parameters, so that the nesting law can ask about the *second* one and
/// not merely about "the parameter".
fn results() -> RawData {
    data(
        vec![binder("E", type0()), binder("A", type0())],
        vec![family(
            "Result",
            vec![
                constructor("Err", vec![binder("error", var("E"))]),
                constructor("Ok", vec![binder("value", var("A"))]),
            ],
        )],
    )
}

/// `data Cont (A : Type 0) { mk(k : A → Nat) }` — a container that is *not*
/// positive in its parameter.
///
/// Perfectly legal on its own: `A` is a parameter rather than a family of the
/// group, so nothing about `Cont`'s own declaration is negative. §1.2 makes it
/// unstorable, and this suite never stores one — what it is here for is that a
/// *later* declaration may not put itself where `A` stands.
fn continuations() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "Cont",
            vec![constructor("mk", vec![binder("k", arrow(var("A"), var("Nat")))])],
        )],
    )
}

/// `Nat`, `List`, `Option`, `Result`, and `Cont`, all declared.
///
/// # Panics
///
/// If any of them is refused, which would be a defect in this suite rather than
/// a property of any law.
fn containers() -> Cx {
    let (mut cx, _) = nat_context();
    for (name, declaration) in [
        ("List", lists()),
        ("Option", options()),
        ("Result", results()),
        ("Cont", continuations()),
    ] {
        let group = musa_core::declare(&cx, &declaration).unwrap_or_else(|error| panic!("{name}: {error}"));
        cx = cx.declaring(&group);
    }
    cx
}

/// `data StaffRead { Sung, Body(items : List StaffRead) }`.
fn staff_read() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "StaffRead",
            vec![
                constructor("Sung", Vec::new()),
                constructor("Body", vec![binder("items", apply(var("List"), [var("StaffRead")]))]),
            ],
        )],
    )
}

/// The same context with `StaffRead` declared into it.
///
/// # Panics
///
/// If the declaration is refused, which is the first law below and would make
/// every later one meaningless.
fn staff_context() -> Cx {
    let cx = containers();
    let group = musa_core::declare(&cx, &staff_read()).expect("a family may hold a list of itself");
    cx.declaring(&group)
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

/// `[Sung, Sung]` — a two-element list of the family being declared.
fn two_sung() -> Raw {
    let nil = apply(var("List.Nil"), [var("StaffRead")]);
    let cons = |tail| apply(var("List.Cons"), [var("StaffRead"), var("StaffRead.Sung"), tail]);
    cons(cons(nil))
}

/// The construction law: the declaration is admitted, its nested constructor is
/// applied to an ordinary list literal, and a `match` binds the field at the
/// container type it was written at.
///
/// Stated as three questions about one program rather than three tests, because
/// a declaration that is admitted and cannot then be *built* would be the same
/// failure wearing a different name.
#[test]
fn a_family_may_hold_a_list_of_itself() {
    let cx = staff_context();
    let staff = core(&cx, "StaffRead", &var("StaffRead"));

    let _built = musa_core::check(&cx, &staff, &apply(var("StaffRead.Body"), [two_sung()]))
        .expect("`Body` is applied to a list of `StaffRead`");

    // `match` on it binds `items` at `List StaffRead` — asked by checking the
    // arm's body against that type and nothing narrower.
    let written = arrow(var("StaffRead"), apply(var("List"), [var("StaffRead")]));
    let ty = core(&cx, "StaffRead → List StaffRead", &written);
    let children = lam(
        "t",
        matching(
            var("t"),
            vec![
                arm(
                    vec![con("StaffRead.Sung", [])],
                    apply(var("List.Nil"), [var("StaffRead")]),
                ),
                arm(vec![con("StaffRead.Body", [bind("items")])], var("items")),
            ],
        ),
    );
    let _read = musa_core::check(&cx, &ty, &children).expect("`items` binds at `List StaffRead`");
}

/// A nested field gets **no induction hypothesis**, which is the decision rather
/// than an omission.
///
/// Stated by applying the generated recursor with a `Body` method that takes the
/// field and nothing else: if a hypothesis had been generated the method would
/// need a second binder, and this would be a plicity or conversion failure
/// instead of the answer `1`. `family.rs`'s standing invariant — an induction
/// hypothesis is an application rather than a synthesized closure — is what that
/// buys, and `a_fold_into_a_nested_field_is_refused` below is what it costs.
#[test]
fn the_recursor_hands_a_nested_field_no_hypothesis() {
    let cx = staff_context();
    let nat = core(&cx, "Nat", &var("Nat"));
    let one = musa_core::check(&cx, &nat, &apply(var("Nat.Succ"), [var("Nat.Zero")])).expect("one is a Nat");

    let counted = musa_core::check(
        &cx,
        &nat,
        &apply(
            var("StaffRead.elim"),
            [
                lam("_", var("Nat")),
                var("Nat.Zero"),
                lam("items", apply(var("Nat.Succ"), [var("Nat.Zero")])),
                apply(var("StaffRead.Body"), [two_sung()]),
            ],
        ),
    )
    .expect("the `Body` method takes the field alone");
    assert!(
        musa_core::convertible(&cx, &nat, &counted, &one).expect("both are Nats"),
        "ι did not fire at the nested constructor"
    );
}

/// The nesting-depth law: the rule is about *position*, not about one level.
///
/// `Option (List Deep)` nests two containers, and `Result Nat (List Deep)` puts
/// the occurrence at the second parameter of the outer one. Both are admitted,
/// so the walk descends rather than pattern-matching a shape.
#[test]
fn an_occurrence_may_sit_at_any_depth_of_positive_parameters() {
    let cx = containers();
    let declaration = data(
        Vec::new(),
        vec![family(
            "Deep",
            vec![
                constructor("Flat", Vec::new()),
                constructor(
                    "Perhaps",
                    vec![binder("m", apply(var("Option"), [apply(var("List"), [var("Deep")])]))],
                ),
                constructor(
                    "Either",
                    vec![binder(
                        "e",
                        apply(var("Result"), [var("Nat"), apply(var("List"), [var("Deep")])]),
                    )],
                ),
            ],
        )],
    );
    musa_core::declare(&cx, &declaration).expect("nesting is about position rather than depth");
}

/// A declaration this widening still refuses, and why it is not the arrow's
/// fault alone.
struct Negative {
    name: &'static str,
    declaration: RawData,
}

/// The arrow rule survives every container, and a container that is negative in
/// its parameter is not a place a family may put itself.
///
/// The third program is what makes [`crate::nesting_laws`]'s claim a computation
/// rather than a convention: `Cont Held` and `List Held` are the same *shape*,
/// and they differ only by what `Cont`'s and `List`'s own declarations computed
/// about their parameters.
#[test]
fn an_occurrence_outside_a_positive_position_is_still_refused() {
    let cx = containers();
    for Negative { name, declaration } in [
        Negative {
            name: "an occurrence to the left of an arrow",
            declaration: data(
                Vec::new(),
                vec![family(
                    "Bad",
                    vec![constructor("mk", vec![binder("f", arrow(var("Bad"), var("Nat")))])],
                )],
            ),
        },
        Negative {
            name: "an arrow under a positive parameter is still an arrow",
            declaration: data(
                Vec::new(),
                vec![family(
                    "Bad",
                    vec![constructor(
                        "mk",
                        vec![binder("f", apply(var("List"), [arrow(var("Bad"), var("Nat"))]))],
                    )],
                )],
            ),
        },
        Negative {
            name: "a parameter its own family is not positive in",
            declaration: data(
                Vec::new(),
                vec![family(
                    "Held",
                    vec![constructor("mk", vec![binder("k", apply(var("Cont"), [var("Held")]))])],
                )],
            ),
        },
    ] {
        let Err(error) = musa_core::declare(&cx, &declaration) else {
            panic!("{name}: the declaration was admitted");
        };
        let refusal = refusal(name, error);
        assert!(
            matches!(refusal, Refusal::NonPositive { .. }),
            "{name}: refused, but as `{refusal}`"
        );
    }
}

/// The recorded gap: a fold that recurses *into* a nested field is refused.
///
/// This is the price of the decision in
/// [`the_recursor_hands_a_nested_field_no_hypothesis`], asserted rather than
/// left for a reader to discover. `head` is a field of `List.Cons`, which is a
/// field of `StaffRead.Body` — and §2.4's measure has no reason to believe an
/// element of `items` is smaller than `Body items`, because the only route
/// between them is `List`'s own eliminator and the two families are not one
/// declaration group.
///
/// The day the measure learns to see through a container, this law changes and
/// says so. That is what it is for.
#[test]
fn a_fold_into_a_nested_field_is_refused() {
    let cx = staff_context();
    let written = arrow(var("StaffRead"), var("Nat"));
    let ty = core(&cx, "StaffRead → Nat", &written);
    let size = Raw::rec(
        WRITTEN,
        "size",
        written,
        lam(
            "t",
            matching(
                var("t"),
                vec![
                    arm(vec![con("StaffRead.Sung", [])], var("Nat.Zero")),
                    arm(
                        vec![con("StaffRead.Body", [bind("items")])],
                        matching(
                            var("items"),
                            vec![
                                arm(vec![con("List.Nil", [])], var("Nat.Zero")),
                                arm(
                                    vec![con("List.Cons", [bind("head"), bind("tail")])],
                                    apply(var("Nat.Succ"), [apply(var("size"), [var("head")])]),
                                ),
                            ],
                        ),
                    ),
                ],
            ),
        ),
    );
    let Err(error) = musa_core::check(&cx, &ty, &size) else {
        panic!("a fold through a container was admitted — §2.4's measure has changed, and so must this law");
    };
    let refusal = refusal("a fold into a nested field", error);
    assert!(
        matches!(refusal, Refusal::UncheckedRecursion { .. }),
        "the fold was refused, but as `{refusal}`"
    );
}
