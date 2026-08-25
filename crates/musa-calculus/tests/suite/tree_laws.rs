//! §1's second `Definition` arm: a name whose reduction behaviour is a compiled
//! case tree.
//!
//! δ for such a name is not "unfold the stored value". It is: collect as many
//! arguments as the body has binders, force the one the tree splits on, and take
//! the alternative its constructor names. Every law here is stated as a
//! computation for the reason `termination_laws.rs` gives — a definition
//! admitted and stuck would be worse than one refused — and the two that matter
//! most are the ones about *not* reducing, because a tree that took an
//! alternative whenever it had arguments would be a runtime `switch` rather than
//! the eliminator it replaces.
//!
//! Three node kinds, two of them reachable. `Answer` is what an alternative
//! holds and `Split` is what selects one; `Impossible` has no producer until
//! prompt 156 gives index unification something to rule out, so there is no law
//! about reducing one here and there should not be.
//!
//! The re-checker's side of this is `coverage_laws.rs`, not this file. A tree
//! body is stored beside its *emission* and the kernel audits the emission, so
//! the obligations — every alternative answers the motive instantiated at its
//! own pattern, and the alternatives are exactly the family's constructors —
//! are the ones that file already states over the identical construction, with
//! its own controls. What is new here is that they hold at a *definition's* top,
//! which is the last law below. Descent is the third obligation and its control
//! is `termination_laws.rs`'s refused corpus.

use musa_calculus::{Budget, Cx, Raw, RawArm, RawPattern, RawProgram, RawTopLevel, Refusal, Shape, Term, Visibility};

use crate::coverage_laws::nat_vec_context;
use crate::family_laws::{apply, var};
use crate::programs::{WRITTEN, refusal};

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

fn number(count: u32) -> Raw {
    (0..count).fold(var("Nat.Zero"), |built, _| apply(var("Nat.Succ"), [built]))
}

/// `name : ty = value`, public and in no module.
fn definition(name: &str, ty: Raw, value: Raw) -> RawTopLevel {
    RawTopLevel {
        origin: WRITTEN,
        name: name.into(),
        visibility: Visibility::Public,
        module: None,
        ty: Some(ty),
        value,
    }
}

/// A context in which `definitions` are declared and usable.
///
/// # Panics
///
/// Naming the fixture, when the group is refused.
fn defining(name: &str, cx: &Cx, definitions: Vec<RawTopLevel>) -> Cx {
    let program = RawProgram {
        families: Vec::new(),
        definitions,
    };
    let group = musa_calculus::declare_program(cx, &program).unwrap_or_else(|error| panic!("{name}: {error}"));
    cx.defining(&group)
}

/// The refusal a group answers with.
///
/// # Panics
///
/// When the group is declared, or fails for something that is not a refusal.
fn refusing(name: &str, cx: &Cx, definitions: Vec<RawTopLevel>) -> Refusal {
    let program = RawProgram {
        families: Vec::new(),
        definitions,
    };
    let Err(error) = musa_calculus::declare_program(cx, &program) else {
        panic!("{name}: the group was declared, and this law says it cannot be");
    };
    refusal(name, error)
}

/// `Nat` as a core term.
///
/// # Panics
///
/// If `Nat` is not in scope, which is a defect in the fixture.
fn nat(cx: &Cx) -> Term {
    musa_calculus::infer(cx, &var("Nat")).expect("Nat is declared").0
}

/// That `term` at `ty` computes to `expected`.
///
/// # Panics
///
/// Naming the fixture, when it does not.
fn computes(name: &str, cx: &Cx, ty: &Term, written: &Raw, expected: &Raw) {
    let term = musa_calculus::check(cx, ty, written).unwrap_or_else(|error| panic!("{name}: {error}"));
    let expected = musa_calculus::check(cx, ty, expected).unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(
        musa_calculus::convertible(cx, ty, &term, &expected).unwrap_or_else(|error| panic!("{name}: {error}")),
        "{name}"
    );
}

/// Whether the normal form still names `wanted`.
fn names(term: &Term, wanted: &str) -> bool {
    match term.shape() {
        Shape::Named { name, .. } => &**name == wanted,
        Shape::Var(_) | Shape::Lit(_) | Shape::Meta(_) | Shape::MetaAt { .. } | Shape::Universe(_) => false,
        Shape::Bind { binder, body, .. } => names(body, wanted) || binder.outer().any(|term| names(term, wanted)),
        Shape::App { function, argument } => names(function, wanted) || names(argument, wanted),
    }
}

/// `rec countdown : Nat → Nat → Nat`, whose body is a tree with a split under a
/// split.
///
/// `match a, b { Zero, y => y ; Succ j, Zero => Succ j ; Succ j, Succ k =>
/// countdown j k }` — the difference of two counts, and the fixture the
/// reduction laws below are stated over. It splits twice, answers in three
/// alternatives, and descends at position 0.
fn countdown() -> RawTopLevel {
    definition(
        "countdown",
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
                        arm(vec![con("Nat.Zero", []), bind("y")], var("y")),
                        arm(
                            vec![con("Nat.Succ", [bind("j")]), con("Nat.Zero", [])],
                            apply(var("Nat.Succ"), [var("j")]),
                        ),
                        arm(
                            vec![con("Nat.Succ", [bind("j")]), con("Nat.Succ", [bind("k")])],
                            apply(var("countdown"), [var("j"), var("k")]),
                        ),
                    ],
                ),
            ),
        ),
    )
}

/// An `Answer` is reached by the `Split` above it taking the alternative the
/// scrutinee's constructor names.
///
/// Three alternatives and three answers, one of them under a second split, so
/// every node the tree has is on some path this law walks. The counts are picked
/// so that each alternative is the *only* one that gives the stated result.
#[test]
fn a_split_takes_the_alternative_its_scrutinee_names() {
    let cx = nat_vec_context();
    let inside = defining("a tree body", &cx, vec![countdown()]);
    let nat = nat(&inside);
    for (left, right) in [(0, 0), (0, 4), (3, 0), (5, 2), (2, 5), (4, 4)] {
        computes(
            "the difference of two counts",
            &inside,
            &nat,
            &apply(var("countdown"), [number(left), number(right)]),
            &number(left.abs_diff(right)),
        );
    }
}

/// A scrutinee that is not canonical leaves the name neutral.
///
/// This is the law that separates a case tree from a runtime `switch`: under a
/// binder the definition has an argument and still must not choose, because the
/// argument is a variable and no constructor named an alternative. Stated over
/// the *normal* form, so a reducer that unfolded the tree into its emitted
/// eliminator spine would fail it too — a name that reduced to something is a
/// name the reader can no longer see.
#[test]
fn a_split_on_a_variable_leaves_the_name_neutral() {
    let cx = nat_vec_context();
    let inside = defining("a tree body", &cx, vec![countdown()]);
    let nat_to_nat = musa_calculus::infer(&inside, &arrow(var("Nat"), var("Nat")))
        .expect("`Nat → Nat` is a type")
        .0;
    let blocked = Raw::lam(WRITTEN, "n", apply(var("countdown"), [var("n"), number(2)]));
    let term = musa_calculus::check(&inside, &nat_to_nat, &blocked).expect("a blocked call is a term");
    let normal = musa_calculus::normalize(&inside, &nat_to_nat, &term).expect("and normalizing it answers");
    assert!(
        names(&normal, "countdown"),
        "a split on a variable chose an alternative anyway"
    );
}

/// Fewer arguments than binders is not a tree to reduce either.
///
/// A `Split` names a binder, so until every binder has an argument there is
/// nothing to force. Partially applied, the definition is an ordinary neutral
/// spine — which is what makes `countdown 3` a value a program may pass around.
#[test]
fn a_definition_given_fewer_arguments_than_binders_does_not_split() {
    let cx = nat_vec_context();
    let inside = defining("a tree body", &cx, vec![countdown()]);
    let nat_to_nat = musa_calculus::infer(&inside, &arrow(var("Nat"), var("Nat")))
        .expect("`Nat → Nat` is a type")
        .0;
    let partial = Raw::annot(
        WRITTEN,
        apply(var("countdown"), [number(3)]),
        arrow(var("Nat"), var("Nat")),
    );
    let term = musa_calculus::check(&inside, &nat_to_nat, &partial).expect("a partial application is a term");
    let normal = musa_calculus::normalize(&inside, &nat_to_nat, &term).expect("and normalizing it answers");
    assert!(
        names(&normal, "countdown"),
        "a tree split before its last argument arrived"
    );

    // And it is still the same function once the argument does arrive.
    computes(
        "the partial application, completed",
        &inside,
        &nat(&inside),
        &apply(partial, [number(1)]),
        &number(2),
    );
}

/// A split on a counting family costs a bounded number of steps.
///
/// `iota.rs`'s tower-avoiding decrement, restated for a reducer that shares no
/// code with it: a numeral decides which alternative it takes from its *count*,
/// not by being unfolded into a spine. `pare 5000 0` splits twice — once on a
/// five-thousand numeral and once on `Zero` — and a reducer without the rule
/// would build five thousand `Succ` nodes to answer the first.
///
/// Stated as a budget rather than as a measurement, the way `numeral_laws.rs`
/// states its own: the meter's high-water mark is private, and exposing it for a
/// test would be a knob added for a test. A sixteenth of the language budget is
/// three orders of magnitude below the count.
#[test]
fn a_split_on_a_large_numeral_costs_a_bounded_number_of_steps() {
    let cx = Cx::with_budget(Budget::LANGUAGE.scaled(16));
    let group = musa_calculus::declare(&cx, &crate::family_laws::nat()).expect("Nat is a declaration");
    let cx = cx.declaring(&group);
    let pare = definition(
        "pare",
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
                        arm(vec![con("Nat.Succ", [bind("j")]), con("Nat.Zero", [])], var("Nat.Zero")),
                        arm(
                            vec![con("Nat.Succ", [bind("j")]), con("Nat.Succ", [bind("k")])],
                            apply(var("pare"), [var("j"), var("k")]),
                        ),
                    ],
                ),
            ),
        ),
    );
    let inside = defining("a tree body split on a numeral", &cx, vec![pare]);
    let nat = nat(&inside);
    computes(
        "a five-thousand numeral, split twice",
        &inside,
        &nat,
        &apply(var("pare"), [Raw::numeral(WRITTEN, "Nat", 5_000), number(0)]),
        &number(0),
    );
}

/// The alternatives of a tree body are exactly the family's constructors, and a
/// definition's top is where that is now asked.
///
/// The obligation itself is `coverage_laws.rs`'s and holds of the emission the
/// body is stored beside. What this pins is that a *recursive* definition, whose
/// body takes the new path, is not admitted through it: the tree is built and
/// refused before descent is ever asked, so the reader is told about the
/// constructor with no arm rather than about a measure that could not see a call
/// in a program that was never going to be complete.
#[test]
fn a_definition_whose_tree_misses_a_constructor_is_refused_before_its_measure() {
    let cx = nat_vec_context();
    let half = definition(
        "half",
        arrow(var("Nat"), var("Nat")),
        Raw::lam(
            WRITTEN,
            "n",
            matching(
                [var("n")],
                vec![arm(vec![con("Nat.Succ", [bind("k")])], apply(var("half"), [var("k")]))],
            ),
        ),
    );
    let refusal = refusing("a tree body with a constructor missing", &cx, vec![half]);
    let Refusal::IncompleteMatch { ref constructor, .. } = refusal else {
        panic!("reached `{refusal}` rather than an incomplete match");
    };
    assert_eq!(&**constructor, "Nat.Zero", "the refusal named the wrong constructor");
}

/// A tree body's arms stand under binders the tree does not carry, and what an
/// unknown is solved to has to be written back counted from *those*.
///
/// `rec held : {A} → A → Nat → A = λv. λn. match n { Zero => v ; Succ k =>
/// held(v, k) }` is the smallest definition that asks the question: the
/// recursive call leaves the type parameter to be inserted, so an alternative's
/// body holds a solved unknown, and the λs it is counted from are the
/// definition's own three plus the two the `Succ` method bound. A term stored
/// with those written back at the wrong depth type-checks nowhere and computes
/// something else — `stdlib/src/list.musa`'s `repeated` found this by naming its
/// own `count` as its element type.
#[test]
fn an_unknown_inside_a_tree_is_solved_at_the_depth_the_arm_stands_at() {
    let cx = nat_vec_context();
    let written = Raw::parameter_pi(
        WRITTEN,
        "A",
        crate::family_laws::type0(),
        arrow(var("A"), arrow(var("Nat"), var("A"))),
    );
    let held = definition(
        "held",
        written,
        Raw::lam(
            WRITTEN,
            "v",
            Raw::lam(
                WRITTEN,
                "n",
                matching(
                    [var("n")],
                    vec![
                        arm(vec![con("Nat.Zero", [])], var("v")),
                        arm(
                            vec![con("Nat.Succ", [bind("k")])],
                            apply(var("held"), [var("v"), var("k")]),
                        ),
                    ],
                ),
            ),
        ),
    );
    let inside = defining("a polymorphic tree body", &cx, vec![held]);
    computes(
        "a value carried past a count",
        &inside,
        &nat(&inside),
        &apply(var("held"), [number(3), number(5)]),
        &number(3),
    );
}
