//! `01-surface.md` §1.6, stated as tests.
//!
//! # Why the library is a fixture
//!
//! `List` and `Option` are declared *here* rather than shipped as items of this
//! crate, and that is [`musa_calculus`](musa_calculus)'s own boundary rather than a convenience:
//! the core is a calculus, it has no base types at all — no `Bool`, no `Nat`, no
//! `Option` — and everything above it is library code elaborated *into* it. A
//! container declared in `src/` would be the first exception, and it would be an
//! exception that buys nothing: what a container needs is a family, a type
//! parameter, and a name in its own namespace, all of which the core already has.
//!
//! So what the suite states is that the mechanism is enough to write the library,
//! at the size where the argument can fail.
//!
//! # Why there is no `Buildable` and no `Iterable`
//!
//! There was, and prompt 146 deleted both. A container's traversal is a
//! definition in the container's own namespace — `List.fold_from_start`, and
//! `Option.fold_from_start` beside it — reached by the head of the receiver's
//! type and by nothing else. The trait around them dispatched on a set with two
//! members and constrained no signature at all; what is left here is the two
//! folds themselves, which is what every call site was ever reaching.
//!
//! The three derived methods went with it — `map`, `filter`, and `collect` had
//! zero call sites, and `map` and `filter` are ordinary functions over `List`'s
//! constructors in `stdlib/src/list.musa`, where they cost the length rather than
//! its square.
//!
//! # Why there is no `Vec A n`
//!
//! Prompt 141's Design made the length-indexed vector conditional on prompt 132's
//! trial finding a program that needed one, and note 43 §7's table records the
//! answer it found: `Vec A n` — *nothing*. The two fixed-arity things the staff
//! adapter has are up to two numbers and up to two words, and both are enums
//! whose cases are named, which reads better than `Vec Syntax<Expr> 2` would. The
//! index machinery is built and tested regardless (`family_laws.rs`,
//! `coverage_laws.rs`, `termination_laws.rs` all use `Vec`); what is not built is
//! a *collection library* around it, because no program asks for one.

use musa_calculus::{Cx, Raw, RawArm, RawData, RawPattern, Refusal, Term, check, convertible, infer};

use crate::family_laws::{apply, binder, constructor, data, family, nat_context, type0, var};
use crate::namespace_laws::{defining, definition};
use crate::programs::WRITTEN;

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

/// `List τ`.
fn list_of(element: Raw) -> Raw {
    apply(var("List"), [element])
}

/// `Option τ`.
fn option_of(element: Raw) -> Raw {
    apply(var("Option"), [element])
}

/// `data Bool where False : Bool; True : Bool`.
///
/// Two constructors and no fields, which is the whole of what a `match` in a
/// traversal's step needs.
fn booleans() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "Bool",
            vec![constructor("False", Vec::new()), constructor("True", Vec::new())],
        )],
    )
}

/// `data Option (A : Type 0) where None : Option A; Some : (value : A) → Option A`.
///
/// What `List` indexing answers, and the Design's reason for it: indexing a list
/// can be out of range, so it keeps the failing shape rather than acquiring a
/// partial operator. That is prompt 137's rule for failing arithmetic, applied
/// where languages usually make the exception.
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

/// `data List (A : Type 0) where Nil : List A; Cons : (head : A) → (tail : List A) → List A`.
fn lists() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "List",
            vec![
                constructor("Nil", Vec::new()),
                constructor(
                    "Cons",
                    vec![binder("head", var("A")), binder("tail", list_of(var("A")))],
                ),
            ],
        )],
    )
}

/// `{A : Type 0} → {B : Type 0} → List A → B → step → B`.
///
/// The element and the answer are both implicit and both are read off the
/// arguments a call supplies: `xs.fold_from_end(seed, combine)` learns `A` from
/// the receiver and `B` from `seed`. Writing either would make the receiver's own
/// type something a call had to repeat.
fn folding(step: Raw) -> Raw {
    let body = arrow(list_of(var("A")), arrow(var("B"), arrow(step, var("B"))));
    Raw::parameter_pi(WRITTEN, "A", type0(), Raw::parameter_pi(WRITTEN, "B", type0(), body))
}

/// `λ{A}. λ{B}. λsource. λzero. λstep. …`, the shape both traversals share.
fn traversal(body: Raw) -> Raw {
    Raw::parameter_lam(
        WRITTEN,
        "A",
        Raw::parameter_lam(WRITTEN, "B", lam("source", lam("zero", lam("step", body)))),
    )
}

/// `List.fold_from_start`: accumulate forwards.
///
/// Note 41 §4's missing operation. The accumulator travels *with* the traversal,
/// so a reader written this way runs in the direction the source is written in.
fn from_start() -> Raw {
    let walk_ty = arrow(list_of(var("A")), arrow(var("B"), var("B")));
    traversal(Raw::annotated_bind(
        WRITTEN,
        "walk",
        walk_ty.clone(),
        Raw::rec(
            WRITTEN,
            "walk",
            walk_ty,
            lam(
                "xs",
                lam(
                    "built",
                    matching(
                        var("xs"),
                        vec![
                            arm(vec![con("List.Nil", [])], var("built")),
                            arm(
                                vec![con("List.Cons", [bind("head"), bind("tail")])],
                                apply(
                                    var("walk"),
                                    [var("tail"), apply(var("step"), [var("built"), var("head")])],
                                ),
                            ),
                        ],
                    ),
                ),
            ),
        ),
        apply(var("walk"), [var("source"), var("zero")]),
    ))
}

/// `List.fold_from_end`: the catamorphism, which is what prompt 156's generated
/// recursor will be.
fn from_end() -> Raw {
    let fold_ty = arrow(list_of(var("A")), var("B"));
    traversal(Raw::annotated_bind(
        WRITTEN,
        "walk",
        fold_ty.clone(),
        Raw::rec(
            WRITTEN,
            "walk",
            fold_ty,
            lam(
                "xs",
                matching(
                    var("xs"),
                    vec![
                        arm(vec![con("List.Nil", [])], var("zero")),
                        arm(
                            vec![con("List.Cons", [bind("head"), bind("tail")])],
                            apply(var("step"), [var("head"), apply(var("walk"), [var("tail")])]),
                        ),
                    ],
                ),
            ),
        ),
        apply(var("walk"), [var("source")]),
    ))
}

/// `List.push`, which is **snoc**.
///
/// Snoc rather than cons, and the choice is forced by the direction its callers
/// walk in: `fold_from_start` hands the step the elements in source order, so a
/// `push` that prepended would build the reverse — the exact reversal note 41 §4
/// is about. It costs a walk per push, which is the price of building a list by
/// pushing at all, and it is paid on compile-time lists of the size an adapter
/// reads.
fn push() -> Raw {
    let list_a = list_of(var("A"));
    let snoc_ty = arrow(list_a.clone(), arrow(var("A"), list_a));
    Raw::parameter_lam(
        WRITTEN,
        "A",
        Raw::rec(
            WRITTEN,
            "snoc",
            snoc_ty,
            lam(
                "xs",
                lam(
                    "item",
                    matching(
                        var("xs"),
                        vec![
                            arm(
                                vec![con("List.Nil", [])],
                                apply(
                                    var("List.Cons"),
                                    [var("A"), var("item"), apply(var("List.Nil"), [var("A")])],
                                ),
                            ),
                            arm(
                                vec![con("List.Cons", [bind("head"), bind("tail")])],
                                apply(
                                    var("List.Cons"),
                                    [var("A"), var("head"), apply(var("snoc"), [var("tail"), var("item")])],
                                ),
                            ),
                        ],
                    ),
                ),
            ),
        ),
    )
}

/// `List.at`, which answers an `Option` because a list index can be out of range.
///
/// The Design's rule read at the one operation that usually gets an exception:
/// every index answers, including the ones past the end. There is no partial
/// operator to reach for and nothing panics — the out-of-range case is a value
/// the author has to read.
fn at() -> Raw {
    let at_ty = arrow(list_of(var("A")), arrow(var("Nat"), option_of(var("A"))));
    Raw::parameter_lam(
        WRITTEN,
        "A",
        Raw::rec(
            WRITTEN,
            "at",
            at_ty,
            lam(
                "xs",
                lam(
                    "wanted",
                    matching(
                        var("xs"),
                        vec![
                            arm(vec![con("List.Nil", [])], apply(var("Option.None"), [var("A")])),
                            arm(
                                vec![con("List.Cons", [bind("head"), bind("tail")])],
                                matching(
                                    var("wanted"),
                                    vec![
                                        arm(
                                            vec![con("Nat.Zero", [])],
                                            apply(var("Option.Some"), [var("A"), var("head")]),
                                        ),
                                        arm(
                                            vec![con("Nat.Succ", [bind("earlier")])],
                                            apply(var("at"), [var("tail"), var("earlier")]),
                                        ),
                                    ],
                                ),
                            ),
                        ],
                    ),
                ),
            ),
        ),
    )
}

/// `Nat`, `Bool`, `Option`, and `List`, with `List`'s namespace written out.
///
/// # Panics
///
/// If any declaration is refused, which would be a defect in this crate rather
/// than a property of any law.
fn context() -> Cx {
    let (cx, _) = nat_context();
    let mut cx = cx;
    for declaration in [booleans(), options(), lists()] {
        let group = musa_calculus::declare(&cx, &declaration).expect("a collection family is a declaration");
        cx = cx.declaring(&group);
    }
    let list_a = list_of(var("A"));
    defining(
        &cx,
        vec![
            definition(
                "List.fold_from_start",
                folding(arrow(var("B"), arrow(var("A"), var("B")))),
                from_start(),
            ),
            definition(
                "List.fold_from_end",
                folding(arrow(var("A"), arrow(var("B"), var("B")))),
                from_end(),
            ),
            definition(
                "List.push",
                Raw::parameter_pi(WRITTEN, "A", type0(), arrow(list_a.clone(), arrow(var("A"), list_a))),
                push(),
            ),
            definition(
                "List.at",
                Raw::parameter_pi(
                    WRITTEN,
                    "A",
                    type0(),
                    arrow(list_of(var("A")), arrow(var("Nat"), option_of(var("A")))),
                ),
                at(),
            ),
        ],
    )
}

/// A written type as a core term.
///
/// # Panics
///
/// If it is not a type, which is a defect in the test that wrote it.
fn core(cx: &Cx, name: &str, ty: &Raw) -> Term {
    infer(cx, ty).unwrap_or_else(|error| panic!("{name}: {error}")).0
}

/// The successor of `Nat.Zero`, as many times as asked.
fn number(count: u32) -> Raw {
    (0..count).fold(var("Nat.Zero"), |built, _| apply(var("Nat.Succ"), [built]))
}

/// `[a, b, c] : List τ`, written out as the constructors it stands for.
fn listed(element: &Raw, items: impl IntoIterator<Item = Raw>) -> Raw {
    fn built(element: &Raw, mut rest: impl Iterator<Item = Raw>) -> Raw {
        match rest.next() {
            None => apply(var("List.Nil"), [element.clone()]),
            Some(head) => apply(var("List.Cons"), [element.clone(), head, built(element, rest)]),
        }
    }
    built(element, items.into_iter())
}

/// `[m, n, …] : List Nat`.
fn numbers(items: impl IntoIterator<Item = u32>) -> Raw {
    listed(&var("Nat"), items.into_iter().map(number))
}

/// Assert two terms at `ty` are definitionally equal, and that both re-check.
///
/// The re-check is not extra: every one of these programs reaches a definition
/// through a member spelling, and a resolution that produced a well-*believed*
/// term rather than a well-typed one would still pass a convertibility test
/// against another term it produced the same way.
///
/// # Panics
///
/// If either side is not a term at `ty`, or if they are not convertible.
fn same(cx: &Cx, name: &str, ty: &Term, left: &Raw, right: &Raw) {
    let left = check(cx, ty, left).unwrap_or_else(|error| panic!("{name} (left): {error}"));
    let right = check(cx, ty, right).unwrap_or_else(|error| panic!("{name} (right): {error}"));
    assert!(
        convertible(cx, ty, &left, &right).unwrap_or_else(|error| panic!("{name}: {error}")),
        "{name}"
    );
}

/// 127dcfaa's decision, restated where both directions of travel exist.
///
/// The two folds have the same signature, so the direction is in the name and
/// nowhere else — which means only a program that *observes* the order can tell
/// them apart. These two do, and minimally: keeping the newest thing seen
/// answers the last element from the start and the first element from the end.
#[test]
fn the_two_folds_run_in_the_directions_their_names_say() {
    let cx = context();
    let nat = core(&cx, "Nat", &var("Nat"));
    let source = numbers([0, 1, 2]);
    same(
        &cx,
        "folding from the start ends at the last element",
        &nat,
        &apply(
            Raw::method(WRITTEN, source.clone(), "fold_from_start"),
            [number(9), lam("built", lam("item", var("item")))],
        ),
        &number(2),
    );
    same(
        &cx,
        "folding from the end ends at the first element",
        &nat,
        &apply(
            Raw::method(WRITTEN, source, "fold_from_end"),
            [number(9), lam("item", lam("built", var("item")))],
        ),
        &number(0),
    );
}

/// §1.6's indexing, and the rule that it stays total.
#[test]
fn indexing_a_list_answers_at_every_index() {
    let cx = context();
    let option_nat = core(&cx, "Option Nat", &option_of(var("Nat")));
    let source = numbers([7, 8]);
    for (which, expected) in [
        (0, apply(var("Option.Some"), [var("Nat"), number(7)])),
        (1, apply(var("Option.Some"), [var("Nat"), number(8)])),
        (2, apply(var("Option.None"), [var("Nat")])),
        (9, apply(var("Option.None"), [var("Nat")])),
    ] {
        same(
            &cx,
            &format!("index {which}"),
            &option_nat,
            &apply(Raw::method(WRITTEN, source.clone(), "at"), [number(which)]),
            &expected,
        );
    }
}

/// Note 41 §4's `voiced_inside`, written the way the finding said it could not be.
///
/// The trial's words: a chord's pitches must come out as one list, a nested
/// group may hold several, "and the phase language has no operation that builds a
/// list or joins two", so a group written inside brackets contributed no pitch.
/// A `map` alone cannot close it, because a `map` is length-preserving and this
/// traversal is not — three children yield three pitches here only by accident of
/// the fixture, and the second child yields none.
///
/// What closes it is a fold *inside* a fold with `push` at the bottom: the outer
/// traversal carries the accumulator across children, and the inner one adds each
/// child's own contribution to it. Both run from the start, so the pitches come
/// out in written order, which is the property the reversed reading algorithm
/// existed to recover.
#[test]
fn a_nested_forward_traversal_joins_what_a_map_could_not() {
    let cx = context();
    let list_nat = list_of(var("Nat"));
    let written = arrow(list_of(list_nat.clone()), list_nat.clone());
    let voiced_inside = lam(
        "groups",
        apply(
            Raw::method(WRITTEN, var("groups"), "fold_from_start"),
            [
                apply(var("List.Nil"), [var("Nat")]),
                lam(
                    "done",
                    Raw::annotated_lam(
                        WRITTEN,
                        "kid",
                        list_nat.clone(),
                        apply(
                            Raw::method(WRITTEN, var("kid"), "fold_from_start"),
                            [
                                var("done"),
                                lam(
                                    "built",
                                    lam("pitch", apply(var("List.push"), [var("built"), var("pitch")])),
                                ),
                            ],
                        ),
                    ),
                ),
            ],
        ),
    );
    let groups = listed(&list_nat, [numbers([0]), numbers([]), numbers([1, 2])]);
    same(
        &cx,
        "a nested group contributes every pitch it holds, in order",
        &core(&cx, "List Nat", &list_nat),
        // A `let` rather than an annotated λ applied on the spot: the core has
        // no annotation form, so a β-redex an author writes is the one shape
        // the independent re-checker cannot give a type to, and `same` re-checks.
        &Raw::annotated_bind(
            WRITTEN,
            "voiced_inside",
            written,
            voiced_inside,
            apply(var("voiced_inside"), [groups]),
        ),
        &numbers([0, 1, 2]),
    );
}

/// §1.6's refusal: a fold in an inferring position names what it could not
/// determine.
///
/// The rule this states is the one the feature is priced against. A fold's
/// answer type is fixed by what its seed and its step say, and a *use* with
/// neither — `xs.fold_from_start` standing alone — has nothing to read it off.
/// Choosing it from a result type nobody has written down is return-type-directed
/// overloading, which makes elaboration depend on the order constraints are
/// reached. So the refusal is not a gap in inference — it is the design, and the
/// diagnostic is an unsolved metavariable rather than a special case invented for
/// traversals.
#[test]
fn a_fold_with_no_answer_type_is_refused_as_the_hole_it_is() {
    let cx = context();
    let Err(error) = infer(&cx, &Raw::method(WRITTEN, numbers([0, 1]), "fold_from_start")) else {
        panic!("`xs.fold_from_start` with nothing to read `B` off must be refused, not guessed at");
    };
    let refusal = crate::programs::refusal("a fold with no answer type", error);
    assert!(
        matches!(refusal, Refusal::Unsolved { .. }),
        "the report is the unsolved answer type, got `{refusal}`"
    );
}

/// The refusal this suite owns, for `elaboration_laws.rs`'s coverage gate.
///
/// Beside the law rather than inside it because the gate asks a different
/// question — *every* refusal has a program that reaches it — and it needs the
/// container context, which no other corpus has a reason to declare.
pub(crate) fn refused_collections() -> Vec<(&'static str, Cx, Raw, fn(&Refusal) -> bool)> {
    vec![(
        "a fold with no answer type",
        context(),
        Raw::method(WRITTEN, numbers([0, 1]), "fold_from_start"),
        |refusal| matches!(refusal, Refusal::Unsolved { .. }),
    )]
}
