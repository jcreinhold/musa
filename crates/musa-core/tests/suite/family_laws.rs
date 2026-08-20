//! §1.1's inductive families: what a declaration admits, what it refuses, and
//! what the generated recursor computes.
//!
//! The suite is written against the facade rather than against the
//! representation, because the representation is private and the whole point of
//! [`musa_core::declare`] is that a caller writes `Nat`, `Nat.Zero`, and
//! `Nat.elim` as ordinary names afterwards. A law stated over `Group`'s fields
//! would be a law about a data structure; these are laws about a language.

use std::sync::Arc;

use musa_core::{
    Cx, Group, Level, Raw, RawArm, RawBinder, RawConstructor, RawData, RawFamily, RawPattern, Refusal, Term, Visibility,
};

use crate::programs::{WRITTEN, refusal};

pub(crate) fn binder(name: &str, ty: Raw) -> RawBinder {
    RawBinder {
        name: Arc::from(name),
        ty,
    }
}

pub(crate) fn constructor(name: &str, fields: Vec<RawBinder>) -> RawConstructor {
    RawConstructor {
        origin: WRITTEN,
        name: Arc::from(name),
        visibility: Visibility::Public,
        fields,
    }
}

pub(crate) fn family(name: &str, constructors: Vec<RawConstructor>) -> RawFamily {
    RawFamily {
        name: Arc::from(name),
        visibility: Visibility::Public,
        constructors,
    }
}

/// The same constructor, marked `private`.
///
/// A combinator rather than a parameter on [`constructor`], because every test
/// that predates §1.3 wrote a public one and adding an argument to all of them
/// would say "visibility" a hundred times to state the default.
pub(crate) fn hidden_case(mut case: RawConstructor) -> RawConstructor {
    case.visibility = Visibility::Private;
    case
}

/// The same family, marked `private`.
pub(crate) fn hidden_family(mut declared: RawFamily) -> RawFamily {
    declared.visibility = Visibility::Private;
    declared
}

pub(crate) fn data(params: Vec<RawBinder>, families: Vec<RawFamily>) -> RawData {
    RawData {
        origin: WRITTEN,
        params,
        families,
    }
}

pub(crate) fn var(name: &str) -> Raw {
    Raw::var(WRITTEN, name)
}

pub(crate) fn apply(head: Raw, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(head, |function, argument| Raw::app(WRITTEN, function, argument))
}

pub(crate) fn type0() -> Raw {
    Raw::universe(WRITTEN, Level::ZERO)
}

/// `data Nat where Zero : Nat; Succ : (n : Nat) → Nat`.
pub(crate) fn nat() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "Nat",
            vec![
                constructor("Zero", Vec::new()),
                constructor("Succ", vec![binder("n", var("Nat"))]),
            ],
        )],
    )
}

/// `Nat`, declared in the empty context.
///
/// # Panics
///
/// If the declaration is refused, which would be a defect in this crate.
pub(crate) fn nat_context() -> (Cx, Arc<Group>) {
    let cx = Cx::new();
    let group = musa_core::declare(&cx, &nat()).expect("Nat is a declaration");
    let cx = cx.declaring(&group);
    (cx, group)
}

/// `data Vec (A : Type 0) { Nil, Cons(x: A, xs: Vec A) }` — the suite's
/// parameterized family, a list under the shorter name.
pub(crate) fn vec() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "Vec",
            vec![
                constructor("Nil", Vec::new()),
                constructor(
                    "Cons",
                    vec![binder("x", var("A")), binder("xs", apply(var("Vec"), [var("A")]))],
                ),
            ],
        )],
    )
}

/// `data Option (A : Type 0) where None : Option A; Some : (x : A) → Option A`.
pub(crate) fn option() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "Option",
            vec![
                constructor("None", Vec::new()),
                constructor("Some", vec![binder("x", var("A"))]),
            ],
        )],
    )
}

/// `data Result (A : Type 0) (E : Type 0)`, whose two parameters are what make
/// the *order* the rule reads them in observable.
pub(crate) fn result() -> RawData {
    data(
        vec![binder("A", type0()), binder("E", type0())],
        vec![family(
            "Result",
            vec![
                constructor("Ok", vec![binder("x", var("A"))]),
                constructor("Err", vec![binder("e", var("E"))]),
            ],
        )],
    )
}

/// `Nat`, `Option`, and `Result`, declared in that order.
///
/// # Panics
///
/// If any declaration is refused, which would be a defect in this crate.
pub(crate) fn container_context() -> Cx {
    let (mut cx, _) = nat_context();
    for declaration in [option(), result()] {
        let group = musa_core::declare(&cx, &declaration).expect("a container is a declaration");
        cx = cx.declaring(&group);
    }
    cx
}

/// §2: "a constructor checks against its family at known parameters and
/// indices" — so the parameters are read off the expected type and not written.
///
/// Four spellings of one term, and the law is that they are one term. A reader
/// who believes `Some(x)` is sugar for `Option.Some Nat x` should be able to
/// check that belief here rather than in the elaborator, because the whole
/// content of the rule is that the two elaborate to the same core term.
#[test]
fn a_constructor_reads_its_parameters_off_the_expected_type() {
    let cx = container_context();
    let option_nat = checked_type(&cx, &apply(var("Option"), [var("Nat")]));

    let written = |raw: Raw| musa_core::check(&cx, &option_nat, &raw);
    written(var("None")).expect("a bare nullary constructor checks");
    written(var("Option.None")).expect("a qualified nullary constructor checks");

    let bare = written(apply(var("Some"), [var("Nat.Zero")])).expect("a bare applied constructor checks");
    let qualified =
        written(apply(var("Option.Some"), [var("Nat.Zero")])).expect("a qualified applied constructor checks");
    let explicit = written(apply(var("Option.Some"), [var("Nat"), var("Nat.Zero")]))
        .expect("the fully written spelling still checks");
    assert_eq!(bare, explicit, "`Some(0)` and `Option.Some Nat 0` are one term");
    assert_eq!(
        qualified, explicit,
        "`Option.Some(0)` and `Option.Some Nat 0` are one term"
    );
}

/// The rule changes nothing about a family with no parameters: there is nothing
/// to read off the expected type, and the term is the one the qualified
/// spelling already built.
#[test]
fn a_family_with_no_parameters_is_unaffected() {
    let (cx, _) = nat_context();
    let nat = core_nat(&cx);
    let written = |raw: Raw| musa_core::check(&cx, &nat, &raw);
    let expectations: &[(&str, Raw, Raw)] = &[
        ("Zero", var("Zero"), var("Nat.Zero")),
        (
            "Succ",
            apply(var("Succ"), [var("Nat.Zero")]),
            apply(var("Nat.Succ"), [var("Nat.Zero")]),
        ),
    ];
    for (name, bare, qualified) in expectations {
        let bare = written(bare.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
        let qualified = written(qualified.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(bare, qualified, "{name}");
    }
}

/// The parameters an inner constructor reads come from the field type it stands
/// at, not from the outermost expected type — which is the only thing that
/// makes the rule compose.
#[test]
fn a_nested_constructor_reads_the_parameters_of_the_type_it_stands_at() {
    let cx = container_context();
    let inner = apply(var("Result"), [var("Nat"), var("Nat")]);
    let ty = checked_type(&cx, &apply(var("Option"), [inner.clone()]));

    let nested = musa_core::check(&cx, &ty, &apply(var("Some"), [apply(var("Ok"), [var("Nat.Zero")])]))
        .expect("`Some(Ok(0))` checks");
    let explicit = musa_core::check(
        &cx,
        &ty,
        &apply(
            var("Option.Some"),
            [
                inner,
                apply(var("Result.Ok"), [var("Nat"), var("Nat"), var("Nat.Zero")]),
            ],
        ),
    )
    .expect("the fully written spelling checks");
    assert_eq!(nested, explicit, "the nested spellings are one term");
}

/// The rule fires where the elaborator used to fail, so it has nothing new to
/// complain about: a constructor given the wrong number of arguments is refused
/// as it was, and reading the parameters does not rescue it.
#[test]
fn a_constructor_given_the_wrong_number_of_arguments_is_still_refused() {
    let cx = container_context();
    let option_nat = checked_type(&cx, &apply(var("Option"), [var("Nat")]));
    let questions: &[(&str, Raw, fn(&Refusal) -> bool)] = &[
        // Under-applied: the parameter is supplied, and what is left is a
        // function type rather than an `Option`.
        ("Some", var("Some"), |refusal| matches!(*refusal, Refusal::Mismatch(_))),
        // Over-applied: the uniqueness rule reads bare `None` as
        // `Option.None`, and the over-application is refused exactly as the
        // qualified spelling's is — one term, one answer.
        ("None(0)", apply(var("None"), [var("Nat.Zero")]), |refusal| {
            matches!(*refusal, Refusal::NotAFunction { .. })
        }),
        (
            "Option.Some Nat 0 0",
            apply(var("Option.Some"), [var("Nat"), var("Nat.Zero"), var("Nat.Zero")]),
            |refusal| matches!(*refusal, Refusal::NotAFunction { .. }),
        ),
    ];
    for (name, raw, expected) in questions {
        let Err(error) = musa_core::check(&cx, &option_nat, raw) else {
            panic!("{name}: the program was admitted");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
    }
}
/// §1.1: a family, its constructors, and its recursor are in scope under the
/// names the declaration gives them, at the types it gives them.
#[test]
fn a_declaration_brings_its_names_into_scope_at_their_types() {
    let (cx, _) = nat_context();
    let expectations: &[(&str, Term)] = &[
        // `Nat : Type 0` — computed, not written: no constructor stores
        // anything, so the join of the non-recursive field levels is 0.
        ("Nat", Term::universe(WRITTEN, Level::ZERO)),
        ("Nat.Zero", core_nat(&cx)),
        ("Nat.Succ", Term::pi(WRITTEN, "n", core_nat(&cx), core_nat(&cx))),
    ];
    for (name, ty) in expectations {
        let (_, found) = musa_core::infer(&cx, &var(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
        let found = musa_core::normalize_type(&cx, &found).unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected = musa_core::normalize_type(&cx, ty).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(found, expected, "{name}");
    }
}

/// The recursor's type, spelled out.
///
/// Written by hand rather than read back from the group, because the assembled
/// type is the one thing in this crate no author writes and no other test would
/// notice being subtly wrong — a motive quantified over the wrong telescope
/// still type-checks and proves nothing useful.
///
/// Asked as a *checking* question, not an inference: §1.3 admits no universe
/// polymorphism, so a recursor's motive level is chosen per use site and a bare
/// `Nat.elim` has nothing to choose it. That is the level metavariable working,
/// and naming a level is what the question does.
#[test]
fn the_generated_recursor_eliminates_into_the_motive() {
    let (cx, _) = nat_context();
    let at =
        |function: u32, argument: Term| Term::app(WRITTEN, Term::var(WRITTEN, musa_core::Index(function)), argument);
    let succ = |argument: Term| Term::app(WRITTEN, core_constant(&cx, "Nat.Succ"), argument);

    // (P : Nat → Type 0) → P Zero → ((n : Nat) → P n → P (Succ n)) → (t : Nat) → P t
    let expected = Term::pi(
        WRITTEN,
        "P",
        Term::pi(WRITTEN, "_", core_nat(&cx), Term::universe(WRITTEN, Level::ZERO)),
        Term::pi(
            WRITTEN,
            "Zero",
            at(0, core_constant(&cx, "Nat.Zero")),
            Term::pi(
                WRITTEN,
                "Succ",
                Term::pi(
                    WRITTEN,
                    "n",
                    core_nat(&cx),
                    Term::pi(
                        WRITTEN,
                        "_",
                        at(2, Term::var(WRITTEN, musa_core::Index(0))),
                        at(3, succ(Term::var(WRITTEN, musa_core::Index(1)))),
                    ),
                ),
                Term::pi(
                    WRITTEN,
                    "t",
                    core_nat(&cx),
                    at(3, Term::var(WRITTEN, musa_core::Index(0))),
                ),
            ),
        ),
    );
    musa_core::check(&cx, &expected, &var("Nat.elim")).expect("the recursor has the type §1.1 generates");

    // The same telescope with the induction hypothesis dropped: a *recursion*
    // rule rather than an induction one, which is the mistake worth catching.
    let without_hypothesis = Term::pi(
        WRITTEN,
        "P",
        Term::pi(WRITTEN, "_", core_nat(&cx), Term::universe(WRITTEN, Level::ZERO)),
        Term::pi(
            WRITTEN,
            "Zero",
            at(0, core_constant(&cx, "Nat.Zero")),
            Term::pi(
                WRITTEN,
                "Succ",
                Term::pi(
                    WRITTEN,
                    "n",
                    core_nat(&cx),
                    at(2, succ(Term::var(WRITTEN, musa_core::Index(0)))),
                ),
                Term::pi(
                    WRITTEN,
                    "t",
                    core_nat(&cx),
                    at(3, Term::var(WRITTEN, musa_core::Index(0))),
                ),
            ),
        ),
    );
    assert!(
        musa_core::check(&cx, &without_hypothesis, &var("Nat.elim")).is_err(),
        "a recursor with no induction hypothesis was accepted as this one"
    );
}

/// §3's ι: a recursor applied to a constructor is its method, applied to that
/// constructor's fields and then to one induction hypothesis per recursive one.
#[test]
fn iota_fires_when_the_target_becomes_a_constructor() {
    let (cx, _) = nat_context();
    // `Nat.elim (λ_. Nat) Zero (λn. λih. n)` — the predecessor, whose Succ method
    // answers the *field* rather than the hypothesis, so a run that confused the
    // two would answer a different number.
    let predecessor = |target: Raw| {
        apply(
            var("Nat.elim"),
            [
                Raw::lam(WRITTEN, "_", var("Nat")),
                var("Nat.Zero"),
                Raw::lam(WRITTEN, "n", Raw::lam(WRITTEN, "ih", var("n"))),
                target,
            ],
        )
    };
    let zero = var("Nat.Zero");
    let one = apply(var("Nat.Succ"), [zero.clone()]);
    let two = apply(var("Nat.Succ"), [one.clone()]);
    let questions: &[(&str, Raw, Raw)] = &[
        ("pred 0 = 0", predecessor(zero.clone()), zero.clone()),
        ("pred 1 = 0", predecessor(one.clone()), zero),
        ("pred 2 = 1", predecessor(two), one),
    ];
    for (name, left, right) in questions {
        let left = musa_core::check(&cx, &core_nat(&cx), left).unwrap_or_else(|error| panic!("{name}: {error}"));
        let right = musa_core::check(&cx, &core_nat(&cx), right).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            musa_core::convertible(&cx, &core_nat(&cx), &left, &right).expect("both are terms at Nat"),
            "{name}"
        );
    }
}

/// A recursor blocked on a variable is *not* its base case, and stays a normal
/// form rather than answering one.
#[test]
fn a_recursor_blocked_on_a_variable_does_not_fire() {
    let (cx, _) = nat_context();
    let arrow = Term::pi(WRITTEN, "m", core_nat(&cx), core_nat(&cx));
    let predecessor = Raw::lam(
        WRITTEN,
        "m",
        apply(
            var("Nat.elim"),
            [
                Raw::lam(WRITTEN, "_", var("Nat")),
                var("Nat.Zero"),
                Raw::lam(WRITTEN, "n", Raw::lam(WRITTEN, "ih", var("n"))),
                var("m"),
            ],
        ),
    );
    let constantly_zero = Raw::lam(WRITTEN, "m", var("Nat.Zero"));
    let predecessor = musa_core::check(&cx, &arrow, &predecessor).expect("a blocked recursor is still well typed");
    let constantly_zero = musa_core::check(&cx, &arrow, &constantly_zero).expect("a constant function is well typed");
    assert!(
        !musa_core::convertible(&cx, &arrow, &predecessor, &constantly_zero).expect("both are terms at Nat → Nat"),
        "a recursor blocked on a variable answered its base case"
    );
}
/// Large elimination survives where a program actually needs it: a `match`
/// whose goal is a universe computes a type by recursion. The recursor's level
/// is the goal's, read off the goal at the split (§1.3) — never inferred from
/// the arms, which would put a term's universe below the checker rather than
/// in the program.
#[test]
fn a_match_computes_a_type() {
    let (cx, _) = nat_context();
    let universe0 = Term::universe(WRITTEN, Level::ZERO);
    let computing = Raw::match_on(
        WRITTEN,
        [var("Nat.Zero")],
        vec![
            RawArm {
                patterns: vec![RawPattern::constructor(WRITTEN, "Nat.Zero", [])],
                body: var("Nat"),
            },
            RawArm {
                patterns: vec![RawPattern::constructor(
                    WRITTEN,
                    "Nat.Succ",
                    [RawPattern::bind(WRITTEN, "k")],
                )],
                body: var("Nat"),
            },
        ],
    );
    let computed = musa_core::check(&cx, &universe0, &computing).expect("a match at a universe checks");
    let nat = musa_core::check(&cx, &universe0, &var("Nat")).expect("Nat at Type 0");
    assert!(
        musa_core::convertible(&cx, &universe0, &computed, &nat).expect("conversion is decidable"),
        "the computed type is the type the arm wrote"
    );
}

/// §1.1: parameters are fixed across the declaration, and a declaration over
/// an earlier one may use its names.
#[test]
fn a_parameterized_family_declares_over_an_earlier_one() {
    let (cx, _) = nat_context();
    let group = musa_core::declare(&cx, &vec()).expect("Vec is a declaration");
    let cx = cx.declaring(&group);

    // `Vec.Cons A x Nil : Vec A`, with `A := {}` and `x := {}`.
    let unit_type = Raw::record_type(WRITTEN, []);
    let unit = Raw::record(WRITTEN, []);
    let singleton = apply(
        var("Vec.Cons"),
        [unit_type.clone(), unit, apply(var("Vec.Nil"), [unit_type.clone()])],
    );
    let ty = musa_core::check(
        &cx,
        &Term::universe(WRITTEN, Level::ZERO),
        &apply(var("Vec"), [unit_type]),
    )
    .expect("`Vec {}` is a type");
    musa_core::check(&cx, &ty, &singleton).expect("a one-element vector inhabits it");
}

/// §1.1: mutually recursive families are declared together, and the recursor of
/// one takes a motive and methods for *both* — which is what makes a mutual
/// induction expressible at all.
#[test]
fn mutual_families_share_one_declaration_and_one_set_of_motives() {
    let (cx, _) = nat_context();
    let group = musa_core::declare(
        &cx,
        &data(
            Vec::new(),
            vec![
                family(
                    "Even",
                    vec![
                        constructor("Zero", Vec::new()),
                        constructor("FromOdd", vec![binder("o", var("Odd"))]),
                    ],
                ),
                family("Odd", vec![constructor("FromEven", vec![binder("e", var("Even"))])]),
            ],
        ),
    )
    .expect("Even and Odd are one declaration");
    let cx = cx.declaring(&group);

    // `Even.elim` takes two motives and three methods before its target.
    let to_nat = apply(
        var("Even.elim"),
        [
            Raw::lam(WRITTEN, "_", var("Nat")),
            Raw::lam(WRITTEN, "_", var("Nat")),
            var("Nat.Zero"),
            Raw::lam(
                WRITTEN,
                "o",
                Raw::lam(WRITTEN, "ih", apply(var("Nat.Succ"), [var("ih")])),
            ),
            Raw::lam(
                WRITTEN,
                "e",
                Raw::lam(WRITTEN, "ih", apply(var("Nat.Succ"), [var("ih")])),
            ),
            apply(var("Even.FromOdd"), [apply(var("Odd.FromEven"), [var("Even.Zero")])]),
        ],
    );
    let counted = musa_core::check(&cx, &core_nat(&cx), &to_nat).expect("a mutual induction elaborates");
    let two = musa_core::check(
        &cx,
        &core_nat(&cx),
        &apply(var("Nat.Succ"), [apply(var("Nat.Succ"), [var("Nat.Zero")])]),
    )
    .expect("two is a Nat");
    assert!(
        musa_core::convertible(&cx, &core_nat(&cx), &counted, &two).expect("both are Nats"),
        "the mutual recursor did not count two constructors"
    );
}

/// A declaration elaboration must refuse, and the refusal it owes.
///
/// Shared with `elaboration_laws.rs`'s coverage gate rather than kept local:
/// that gate asserts every [`Refusal`] has a program reaching it, and two of
/// them are reached only by a declaration. A corpus in one place is what keeps
/// the gate from being satisfied by a variant nobody ever exercises.
pub(crate) struct RefusedData {
    pub(crate) name: &'static str,
    pub(crate) declaration: RawData,
    pub(crate) expected: fn(&Refusal) -> bool,
}

/// Every declaration §1.1 refuses, and why.
pub(crate) fn refused_declarations() -> Vec<RefusedData> {
    let positive = |name, declaration| RefusedData {
        name,
        declaration,
        expected: |refusal: &Refusal| matches!(*refusal, Refusal::NonPositive { .. }),
    };
    vec![
        positive(
            "an occurrence to the left of an arrow",
            data(
                Vec::new(),
                vec![family(
                    "Bad",
                    vec![constructor(
                        "mk",
                        vec![binder("f", Raw::pi(WRITTEN, "_", var("Bad"), var("Bad")))],
                    )],
                )],
            ),
        ),
        positive(
            // §1.1 admits the infinitary constructor and [`musa_core`] does not:
            // §1.2 makes an arrow unstorable anyway, so the narrowing costs a
            // family nothing here could hold. The mutual form is the workaround.
            "an occurrence to the right of an arrow",
            data(
                Vec::new(),
                vec![family(
                    "Inf",
                    vec![constructor(
                        "sup",
                        vec![binder("f", Raw::pi(WRITTEN, "_", var("Nat"), var("Inf")))],
                    )],
                )],
            ),
        ),
        positive(
            "an occurrence inside a record field",
            data(
                Vec::new(),
                vec![family(
                    "Boxed",
                    vec![constructor(
                        "wrap",
                        vec![binder("r", Raw::record_type(WRITTEN, [("here", var("Boxed"))]))],
                    )],
                )],
            ),
        ),
        positive(
            "an occurrence through a mutual sibling's argument",
            data(
                vec![binder("A", type0())],
                vec![
                    family(
                        "Tree",
                        vec![constructor(
                            "node",
                            vec![binder("kids", apply(var("Forest"), [var("A")]))],
                        )],
                    ),
                    family(
                        "Forest",
                        vec![constructor(
                            "nested",
                            vec![binder(
                                "f",
                                Raw::pi(WRITTEN, "_", apply(var("Tree"), [var("A")]), var("A")),
                            )],
                        )],
                    ),
                ],
            ),
        ),
        positive(
            "a parameter whose type is the declaration",
            data(
                vec![binder("p", var("Loop"))],
                vec![family("Loop", vec![constructor("mk", Vec::new())])],
            ),
        ),
        RefusedData {
            name: "a family declaring one case name twice",
            declaration: data(
                Vec::new(),
                vec![family(
                    "Tying",
                    vec![
                        constructor("Untied", Vec::new()),
                        constructor("Untied", vec![binder("n", var("Nat"))]),
                    ],
                )],
            ),
            expected: |refusal: &Refusal| matches!(*refusal, Refusal::DuplicateCase { .. }),
        },
        RefusedData {
            // §1.3: `private` on a case is what makes a *type* abstract, so a
            // family that hides one case and publishes another has said two
            // incompatible things about the same type. Refused at the
            // declaration rather than at the first client who trips over it.
            name: "an enum hiding some of its cases and not the others",
            declaration: data(
                Vec::new(),
                vec![family(
                    "Half",
                    vec![
                        constructor("Open", Vec::new()),
                        hidden_case(constructor("Shut", Vec::new())),
                    ],
                )],
            ),
            expected: |refusal: &Refusal| matches!(*refusal, Refusal::MixedVisibility { .. }),
        },
    ]
}

/// §1.1: strict positivity and the index arity, refused where they must be.
#[test]
fn a_declaration_is_refused_for_the_reason_it_is_wrong() {
    let (cx, _) = nat_context();
    for RefusedData {
        name,
        declaration,
        expected,
    } in refused_declarations()
    {
        let Err(error) = musa_core::declare(&cx, &declaration) else {
            panic!("{name}: the declaration was admitted");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
    }
}

/// A written type, elaborated so that it can be checked against.
///
/// # Panics
///
/// If it is not a type, which is a defect in the test that asked.
fn checked_type(cx: &Cx, raw: &Raw) -> Term {
    musa_core::check(cx, &Term::universe(WRITTEN, Level::ZERO), raw).expect("a type")
}

/// `Nat` as a core term, for a checking question.
fn core_nat(cx: &Cx) -> Term {
    core_constant(cx, "Nat")
}

/// A declared name as a core term.
///
/// Elaborated rather than constructed, because [`musa_core::Constant`] has no
/// public constructor — which is the boundary holding: a caller writes a name
/// and this crate decides what it means.
///
/// # Panics
///
/// If the name is not in scope, which is a defect in the test that asked.
pub(crate) fn core_constant(cx: &Cx, name: &str) -> Term {
    musa_core::infer(cx, &var(name))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .0
}
