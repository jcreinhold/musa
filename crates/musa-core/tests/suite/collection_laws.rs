//! `01-surface.md` §1.6 and `10-traits.md` §8, stated as tests.
//!
//! # Why the library is a fixture
//!
//! `List`, `Option`, `Buildable`, `Iterable`, and `Index` are declared *here*
//! rather than shipped as items of this crate, and that is
//! [`musa_core`](musa_core)'s own boundary rather than a convenience: the core is
//! a calculus, it has no base types at all — no `Bool`, no `Nat`, no `Option` —
//! and everything above it is library code elaborated *into* it. A container
//! declared in `src/` would be the first exception, and it would be an exception
//! that buys nothing: the mechanism a container needs is a trait with a derived
//! method that quantifies and constrains, and that mechanism is what this prompt
//! actually added. Prompt 142 writes these declarations once more, in `.musa`,
//! where authors can reach them.
//!
//! So what the suite states is that the mechanism is enough to write the library
//! — the same argument `operator_laws.rs` makes for `Add` and `trait_laws.rs` for
//! `Eq`, at the size where the argument can fail.
//!
//! # Why there is no `Vec A n`
//!
//! Prompt 141's Design made the length-indexed vector conditional on prompt 132's
//! trial finding a program that needed one, and note 43 §7's table records the
//! answer it found: `Vec A n` — *nothing*. §10 says why. The two fixed-arity
//! things the staff adapter has are up to two numbers and up to two words, and
//! both are enums whose cases are named, which reads better than `Vec Syntax<Expr>
//! 2` would. The index machinery is built and tested regardless
//! (`family_laws.rs`, `coverage_laws.rs`, `termination_laws.rs` all use `Vec`);
//! what is not built is a *collection library* around it, because no program asks
//! for one.

use musa_core::{
    Cx, Raw, RawArm, RawData, RawImpl, RawPattern, RawTrait, Refusal, Term, check, convertible, declare_impl,
    declare_trait, infer,
};

use crate::family_laws::{apply, binder, constructor, data, family, nat_context, type0, var};
use crate::programs::WRITTEN;
use crate::trait_laws::{class, constraint, defines, derived, generic, instance, method};

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
/// `filter`'s predicate answers one, so the trait pair cannot be declared
/// without it. Two constructors and no fields, which is the whole of what a
/// `match` in a derived body needs.
fn booleans() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "Bool",
            vec![
                constructor("False", Vec::new()),
                constructor("True", Vec::new()),
            ])],
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
            ])],
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
                    vec![binder("head", var("A")), binder("tail", list_of(var("A")))]),
            ])],
    )
}

/// `trait Buildable<C, A> { fn empty() -> C; fn push(target: C, item: A) -> C; }`.
fn buildable() -> RawTrait {
    class(
        "Buildable",
        vec![binder("C", type0()), binder("A", type0())],
        Vec::new(),
        vec![
            method("empty", var("C")),
            method("push", arrow(var("C"), arrow(var("A"), var("C")))),
        ],
    )
}

/// §1.6's `Iterable<C, A>`: two required folds and three derived methods.
///
/// The three carry the whole point of the design. Each is written once, in terms
/// of `fold_from_start` and a `Buildable` its *caller* supplies, and every
/// container that writes the two folds gets all three. `map` and `collect`
/// quantify over the container they build into, which is why the target need not
/// be the source's own type; `filter` does not, because a filtered container is
/// the same container with fewer things in it.
fn iterable() -> RawTrait {
    let step_from_start = arrow(var("B"), arrow(var("A"), var("B")));
    let step_from_end = arrow(var("A"), arrow(var("B"), var("B")));
    class(
        "Iterable",
        vec![binder("C", type0()), binder("A", type0())],
        Vec::new(),
        vec![
            generic(
                method(
                    "fold_from_start",
                    arrow(var("C"), arrow(var("B"), arrow(step_from_start, var("B")))),
                ),
                vec![binder("B", type0())],
                Vec::new(),
            ),
            generic(
                method(
                    "fold_from_end",
                    arrow(var("C"), arrow(var("B"), arrow(step_from_end, var("B")))),
                ),
                vec![binder("B", type0())],
                Vec::new(),
            ),
            generic(
                derived(
                    "map",
                    arrow(var("C"), arrow(arrow(var("A"), var("B")), var("D"))),
                    lam("source", lam("f", accumulating(var("D"), apply(var("f"), [var("item")])))),
                ),
                vec![binder("D", type0()), binder("B", type0())],
                vec![constraint("Buildable", vec![var("D"), var("B")])],
            ),
            generic(
                derived(
                    "filter",
                    arrow(var("C"), arrow(arrow(var("A"), var("Bool")), var("C"))),
                    filtering(),
                ),
                Vec::new(),
                vec![constraint("Buildable", vec![var("C"), var("A")])],
            ),
            generic(
                derived(
                    "collect",
                    arrow(var("C"), var("D")),
                    lam("source", accumulating(var("D"), var("item"))),
                ),
                vec![binder("D", type0())],
                vec![constraint("Buildable", vec![var("D"), var("A")])],
            ),
        ],
    )
}

/// The fold `map` and `collect` share: build forwards, pushing `contributed`.
///
/// `fold_from_start` and `push` in that combination is the whole of §1.6's
/// claim, and it is also note 41 §4's missing operation: the accumulator travels
/// *with* the traversal, so a reader written this way runs in the direction the
/// source is written in. The two derived methods differ only in what reaches the
/// accumulator and in how many parameters they bind, which is why the fold is one
/// function here and the λs around it are not.
///
/// `source` and `item` are free, and `contributed` may mention `f`: this is an
/// expression written under whichever binders its method has, not a closed one.
///
/// `fold_from_start` is reached by its bare name and not as `source.fold_…`,
/// because `source : C` and `C` is a trait parameter — §6's first refusal, and
/// the one the trait machinery answers rather than works around. A required
/// method stands in a derived body as an ordinary definition bound to its
/// projection out of the dictionary, which is what §1 means by "a derived method
/// is an ordinary function that takes the dictionary".
fn accumulating(accumulator: Raw, contributed: Raw) -> Raw {
    apply(
        var("fold_from_start"),
        [
            var("source"),
            var("Buildable.empty"),
            // §2.1: the fold's `B` is still the walk's hole here, so the step
            // carries its type — which is what `B` is solved from.
            Raw::annotated_lam(
                WRITTEN,
                "built",
                accumulator,
                Raw::lam(WRITTEN, "item", apply(var("Buildable.push"), [var("built"), contributed])),
            ),
        ],
    )
}

/// `filter`'s body: the same forward fold, pushing only what `keep` admits.
fn filtering() -> Raw {
    lam(
        "source",
        lam(
            "keep",
            apply(
                var("fold_from_start"),
                [
                    var("source"),
                    var("Buildable.empty"),
                    // §2.1: the fold's `B` is the walk's hole here, so the step
                    // carries its type, as in `map`.
                    Raw::annotated_lam(
                        WRITTEN,
                        "built",
                        var("C"),
                        Raw::lam(
                            WRITTEN,
                            "item",
                            matching(
                                apply(var("keep"), [var("item")]),
                                vec![
                                    arm(
                                        vec![con("Bool.True", [])],
                                        apply(var("Buildable.push"), [var("built"), var("item")]),
                                    ),
                                    arm(vec![con("Bool.False", [])], var("built")),
                                ],
                            ),
                        ),
                    ),
                ],
            ),
        ),
    )
}

/// `trait Index<C, I, R> { fn at(source: C, index: I) -> R; }`.
///
/// `10-traits.md` §5's third column is the reason `R` is a parameter rather than
/// `Option<A>`: an index type that cannot be out of range answers the element
/// itself, and a trait that fixed the `Option` would make every total container
/// pay for `List`'s partiality.
fn indexing() -> RawTrait {
    class(
        "Index",
        vec![binder("C", type0()), binder("I", type0()), binder("R", type0())],
        Vec::new(),
        vec![method("at", arrow(var("C"), arrow(var("I"), var("R"))))],
    )
}

/// `impl<A> Buildable<List A, A>`, whose `push` is **snoc**.
///
/// Snoc rather than cons, and the choice is forced: `fold_from_start` hands the
/// accumulator the elements in source order, so a `push` that prepended would
/// make `collect` reverse and `map` reverse with it — the exact reversal note 41
/// §4 is about. It costs a walk per push, which makes `collect` quadratic; that
/// is the price of `Buildable` having only `empty` and `push`, it is paid on
/// compile-time lists of the size an adapter reads, and a cheaper builder is a
/// third method with a program behind it rather than a redesign of this one.
fn buildable_list() -> RawImpl {
    let list_a = list_of(var("A"));
    let snoc = Raw::rec(
        WRITTEN,
        "snoc",
        arrow(list_a.clone(), arrow(var("A"), list_a)),
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
    );
    instance(
        "Buildable",
        vec![binder("A", type0())],
        vec![list_of(var("A")), var("A")],
        Vec::new(),
        vec![
            defines("empty", apply(var("List.Nil"), [var("A")])),
            defines("push", snoc),
        ],
    )
}

/// `impl<A> Iterable<List A, A>`: the two folds, and nothing else.
///
/// Three methods arrive with them, which is §8's "a container earns five
/// operations by writing two" as a thing the type checker enforces rather than a
/// claim in prose — an impl that tried to write `map` is refused as supplying a
/// derived method.
fn iterable_list() -> RawImpl {
    let list_a = list_of(var("A"));

    // `λ{B}. λsource. λzero. λstep. walk source zero`, accumulating forwards.
    let walk_ty = arrow(list_a.clone(), arrow(var("B"), var("B")));
    let from_start = Raw::implicit_lam(
        WRITTEN,
        "B",
        lam(
            "source",
            lam(
                "zero",
                lam(
                    "step",
                    Raw::annotated_bind(
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
                    ),
                ),
            ),
        ),
    );

    // `λ{B}. λsource. λzero. λstep. walk source`, the catamorphism.
    let fold_ty = arrow(list_a, var("B"));
    let from_end = Raw::implicit_lam(
        WRITTEN,
        "B",
        lam(
            "source",
            lam(
                "zero",
                lam(
                    "step",
                    Raw::annotated_bind(
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
                    ),
                ),
            ),
        ),
    );

    instance(
        "Iterable",
        vec![binder("A", type0())],
        vec![list_of(var("A")), var("A")],
        Vec::new(),
        vec![
            defines("fold_from_start", from_start),
            defines("fold_from_end", from_end),
        ],
    )
}

/// `impl<A> Index<List A, Nat, Option A>`.
fn index_list() -> RawImpl {
    let at_ty = arrow(list_of(var("A")), arrow(var("Nat"), option_of(var("A"))));
    let at = Raw::rec(
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
    );
    instance(
        "Index",
        vec![binder("A", type0())],
        vec![list_of(var("A")), var("Nat"), option_of(var("A"))],
        Vec::new(),
        vec![defines("at", at)],
    )
}

/// `Nat`, `Bool`, `Option`, and `List`, with the three traits and their instances.
///
/// # Panics
///
/// If any declaration is refused, which would be a defect in this crate rather
/// than a property of any law.
pub(crate) fn context() -> Cx {
    let (cx, _) = nat_context();
    let mut cx = cx;
    for declaration in [booleans(), options(), lists()] {
        let group = musa_core::declare(&cx, &declaration).expect("a collection family is a declaration");
        cx = cx.declaring(&group);
    }
    // `Buildable` before `Iterable`, because `Iterable`'s derived bodies name it.
    for declaration in [buildable(), iterable(), indexing()] {
        let declared = declare_trait(&cx, &declaration).expect("a collection trait is a declaration");
        cx = cx.declaring_class(&declared);
    }
    for declaration in [buildable_list(), iterable_list(), index_list()] {
        let declared = declare_impl(&cx, &declaration).expect("a collection instance is a declaration");
        cx = cx.declaring_instance(&declared);
    }
    cx
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
pub(crate) fn numbers(items: impl IntoIterator<Item = u32>) -> Raw {
    listed(&var("Nat"), items.into_iter().map(number))
}

/// Assert two terms at `ty` are definitionally equal, and that both re-check.
///
/// The re-check is not extra: every one of these programs is a trait method
/// reached through a dictionary, and a dictionary elaboration that produced a
/// well-*believed* term rather than a well-typed one would still pass a
/// convertibility test against another term it produced the same way.
///
/// # Panics
///
/// If either side is not a term at `ty`, if either is rejected by the
/// or if they are not convertible.
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

/// The round trip: what `collect` builds is what the fold read.
///
/// This is where `push` being snoc is load-bearing rather than a comment. The
/// law is stated at a list whose elements are distinct, so a `push` that
/// prepended would build the reverse and fail here rather than pass on a
/// palindrome.
#[test]
fn collecting_a_list_gives_back_the_list() {
    let cx = context();
    let list_nat = core(&cx, "List Nat", &list_of(var("Nat")));
    same(
        &cx,
        "collect at the source's own type is the identity",
        &list_nat,
        &Raw::method(WRITTEN, numbers([0, 1, 2]), "collect"),
        &numbers([0, 1, 2]),
    );
}

/// §1.6's `map`: the function applied to every element, in order.
#[test]
fn mapping_applies_the_function_to_every_element_in_order() {
    let cx = context();
    let list_nat = core(&cx, "List Nat", &list_of(var("Nat")));
    same(
        &cx,
        "map",
        &list_nat,
        &apply(Raw::method(WRITTEN, numbers([0, 1, 2]), "map"), [var("Nat.Succ")]),
        &numbers([1, 2, 3]),
    );
}

/// `collect` and `map` commute, which is the prompt's law and also the check
/// that the two derived methods share one traversal.
///
/// They are written from one body ([`accumulating`]) so a failure here would be
/// a failure of the *dictionary* rather than of either method: the two are
/// reached at different own-parameters and different `Buildable` constraints, and
/// this is where a use site that filled either wrongly shows up.
#[test]
fn collecting_and_mapping_commute() {
    let cx = context();
    let list_nat = core(&cx, "List Nat", &list_of(var("Nat")));
    let source = numbers([0, 1, 2]);
    let written = list_of(var("Nat"));
    // Both intermediates are annotated for the same reason: `map` and `collect`
    // fix their target by *checking*, and a receiver position infers. §1.6's
    // refusal and §6's exact-receiver rule are one rule seen twice, and this is
    // the annotation both diagnostics ask for.
    same(
        &cx,
        "map then collect is collect then map",
        &list_nat,
        &Raw::method(
            WRITTEN,
            Raw::annot(
                WRITTEN,
                apply(Raw::method(WRITTEN, source.clone(), "map"), [var("Nat.Succ")]),
                written.clone(),
            ),
            "collect",
        ),
        &apply(
            Raw::method(
                WRITTEN,
                Raw::annot(WRITTEN, Raw::method(WRITTEN, source, "collect"), written),
                "map",
            ),
            [var("Nat.Succ")],
        ),
    );
}

/// §1.6's `filter`, and the one derived method with no own parameters: a
/// filtered container is the same container.
#[test]
fn filtering_keeps_the_elements_the_predicate_admits() {
    let cx = context();
    let list_nat = core(&cx, "List Nat", &list_of(var("Nat")));
    // `A` is postponed with the dictionary, so the predicate's binder is checked
    // against a metavariable and the `match` has no constructors to look at.
    // Naming `Nat` is §6's repair, and the one an author writes.
    let is_zero = Raw::annotated_lam(
        WRITTEN,
        "n",
        var("Nat"),
        matching(
            var("n"),
            vec![
                arm(vec![con("Nat.Zero", [])], var("Bool.True")),
                arm(vec![con("Nat.Succ", [bind("earlier")])], var("Bool.False")),
            ],
        ),
    );
    same(
        &cx,
        "filter",
        &list_nat,
        &apply(Raw::method(WRITTEN, numbers([0, 1, 0, 2]), "filter"), [is_zero]),
        &numbers([0, 0]),
    );
}

/// §5's indexing, and the Design's rule that it stays total.
///
/// Every index answers, including the ones past the end. There is no partial
/// operator to reach for and nothing panics, which is the whole of what "total"
/// buys: the out-of-range case is a value the author has to read.
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

/// §1.6's refusal: `collect` in an inferring position names what it could not
/// determine.
///
/// The rule this states is the one the feature is priced against. `collect`'s
/// target is fixed by *checking*, and a checking position is something an author
/// wrote; choosing the instance from a result type nobody has written down is
/// return-type-directed overloading, which makes elaboration depend on the order
/// constraints are reached. So the refusal is not a gap in inference — it is the
/// design, and the diagnostic is an unsolved metavariable rather than a special
/// case invented for `collect`.
#[test]
fn collect_in_an_inferring_position_is_refused() {
    let cx = context();
    let Err(error) = infer(&cx, &Raw::method(WRITTEN, numbers([0, 1]), "collect")) else {
        panic!("`xs.collect()` with nothing to check against must be refused, not guessed at");
    };
    let refusal = crate::programs::refusal("collect with no target", error);
    assert!(
        matches!(refusal, Refusal::Unsolved { .. }),
        "the report is the unsolved target, got `{refusal}`"
    );
}

/// Note 41 §4's `voiced_inside`, written the way the finding said it could not be.
///
/// The trial's words: a chord's pitches must come out as one list, a nested
/// group may hold several, "and the phase language has no operation that builds a
/// list or joins two", so a group written inside brackets contributed no pitch.
/// `map` alone cannot close it, because a `map` is length-preserving and this
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
                var("Buildable.empty"),
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
                                    lam("pitch", apply(var("Buildable.push"), [var("built"), var("pitch")])),
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


