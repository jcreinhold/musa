//! `docs/rules/language/10-traits.md` §2, §3, and §4, stated as tests.
//!
//! Coherence, the orphan rule, and the termination measure are one subject:
//! each of them is what makes instance *lookup* a table read rather than a
//! search. They live apart from [`trait_laws`](crate::trait_laws) — which is
//! about what a trait and an impl are — because they are the properties a whole
//! program has rather than properties of one declaration, and because every one
//! of them is checked at the declaration precisely so that no use site has to
//! ask.

use musa_core::{Cx, Origin, PackageId, RawConstraint, Refusal, declare_impl, declare_trait};

use crate::family_laws::{apply, binder, constructor, data, family, nat, nat_context, type0, var};
use crate::programs::{WRITTEN, refusal};
use crate::trait_laws::{boxed_context, class, constraint, defines, eq, eq_nat, instance, method};

/// A second origin, so that a report naming two declarations can be checked to
/// name the *right* two.
const ELSEWHERE: Origin = Origin::node(401);

#[test]
fn a_second_instance_for_a_key_names_the_first() {
    let (cx, _) = nat_context();
    let equality = declare_trait(&cx, &eq()).expect("Eq is a declaration");
    let cx = cx.declaring_class(&equality);
    let first = eq_nat(&cx);
    let cx = cx.declaring_instance(&first);

    let second = musa_core::RawImpl {
        origin: ELSEWHERE,
        ..instance(
            "Eq",
            Vec::new(),
            vec![var("Nat")],
            Vec::new(),
            vec![defines("equal", var("Nat.Zero"))],
        )
    };
    let Err(error) = declare_impl(&cx, &second) else {
        panic!("a second instance for a key one already answers was accepted");
    };
    let Refusal::DuplicateInstance { at, previous, .. } = refusal("a second instance", error) else {
        panic!("a second instance was refused as something other than a clash");
    };

    // §2 reports at the *second* declaration and names the first, because that
    // is the pair an author has to choose between. A report that named only one
    // of them would send them to a file where nothing looks wrong.
    assert_eq!(at, ELSEWHERE, "the clash is reported away from the second declaration");
    assert_eq!(
        previous, WRITTEN,
        "the clash does not name the instance it collides with"
    );
}

#[test]
fn an_instance_is_at_home_with_its_trait() {
    // Trait and instance in one package, head type in another: §3's first home.
    let types = Cx::new().in_package(PackageId::new(1));
    let group = musa_core::declare(&types, &nat()).expect("Nat is a declaration");
    let library = types.declaring(&group).in_package(PackageId::new(2));
    let equality = declare_trait(&library, &eq()).expect("Eq is a declaration");
    let library = library.declaring_class(&equality);

    assert!(
        declare_impl(
            &library,
            &instance(
                "Eq",
                Vec::new(),
                vec![var("Nat")],
                Vec::new(),
                vec![defines("equal", var("Nat.Zero"))],
            ),
        )
        .is_ok(),
        "an instance is an orphan in the package that declares its trait"
    );
}

#[test]
fn an_instance_is_at_home_with_its_head_type() {
    // Trait in one package, head type and instance in another: §3's second home.
    let library = Cx::new().in_package(PackageId::new(1));
    let equality = declare_trait(&library, &eq()).expect("Eq is a declaration");
    let library = library.declaring_class(&equality).in_package(PackageId::new(2));
    let group = musa_core::declare(&library, &nat()).expect("Nat is a declaration");
    let types = library.declaring(&group);

    assert!(
        declare_impl(
            &types,
            &instance(
                "Eq",
                Vec::new(),
                vec![var("Nat")],
                Vec::new(),
                vec![defines("equal", var("Nat.Zero"))],
            ),
        )
        .is_ok(),
        "an instance is an orphan in the package that declares its head type"
    );
}

/// `trait Held<K, V> where Eq<V> { fn held : V; }`.
///
/// The super-constraint lands on the *second* parameter, which is what lets an
/// instance be filed under a rigid head while the constraint it has to discharge
/// falls on a type variable.
fn held() -> musa_core::RawTrait {
    class(
        "Held",
        vec![binder("K", type0()), binder("V", type0())],
        vec![constraint("Eq", vec![var("V")])],
        vec![method("held", var("V"))],
    )
}

/// `Nat`, `Box`, `Eq`, `impl Eq<Nat>`, and `Held`.
fn holding() -> Cx {
    let (cx, _) = nat_context();
    let cx = boxed_context(&cx);
    let equality = declare_trait(&cx, &eq()).expect("Eq is a declaration");
    let cx = cx.declaring_class(&equality);
    let cx = cx.declaring_instance(&eq_nat(&cx));
    let holder = declare_trait(&cx, &held()).expect("Held is a declaration");
    cx.declaring_class(&holder)
}

/// `impl<A> Held<Box A, A> [where Eq<A>] { held = Eq.equal; }`.
fn holds(cx: &Cx, context: Vec<RawConstraint>) -> Result<(), musa_core::ElabError> {
    declare_impl(
        cx,
        &instance(
            "Held",
            vec![binder("A", type0())],
            vec![apply(var("Box"), [var("A")]), var("A")],
            context,
            vec![defines("held", var("Eq.equal"))],
        ),
    )
    .map(|_| ())
}

#[test]
fn a_local_dictionary_answers_where_no_global_instance_could() {
    let cx = holding();

    // With the `where`, both the super-constraint `Eq<A>` and the method's own
    // use of `Eq.equal` are answered by the same bound dictionary — §4 step 1,
    // before the table is consulted at all.
    assert!(
        holds(&cx, vec![constraint("Eq", vec![var("A")])]).is_ok(),
        "a `where` dictionary did not answer the constraint it discharges"
    );

    // Without it, `impl Eq<Nat>` is still in scope and still cannot help: a type
    // variable is not `Nat`, and no instance for a variable can ever be
    // declared. That is why the two lookups can never disagree under coherence,
    // and why local-beats-global buys determinacy rather than a different
    // answer — instantiating `A` later cannot reroute a call already elaborated.
    let Err(error) = holds(&cx, Vec::new()) else {
        panic!("a constraint on an unconstrained variable was answered from the table");
    };
    let refused = refusal("an unconstrained variable", error);
    assert!(
        matches!(refused, Refusal::UnconstrainedVariable { .. }),
        "an unconstrained variable was refused as `{refused}`"
    );
}

/// `data F (A : Type 0)` and `data G (A : Type 0) (B : Type 0)`, plus two
/// one-parameter traits — the shape §4's measure is stated over.
fn measured() -> Cx {
    let cx = Cx::new();
    let one = musa_core::declare(
        &cx,
        &data(
            vec![binder("A", type0())],
            vec![family(
                "F",
                Vec::new(),
                vec![constructor("Wrapped", vec![binder("a", var("A"))], Vec::new())],
            )],
        ),
    )
    .expect("F is a declaration");
    let cx = cx.declaring(&one);
    let two = musa_core::declare(
        &cx,
        &data(
            vec![binder("A", type0()), binder("B", type0())],
            vec![family(
                "G",
                Vec::new(),
                vec![constructor(
                    "Both",
                    vec![binder("a", var("A")), binder("b", var("B"))],
                    Vec::new(),
                )],
            )],
        ),
    )
    .expect("G is a declaration");
    let cx = cx.declaring(&two);
    let c = declare_trait(&cx, &class("C", vec![binder("A", type0())], Vec::new(), Vec::new()))
        .expect("C is a declaration");
    let cx = cx.declaring_class(&c);
    let d = declare_trait(&cx, &class("D", vec![binder("A", type0())], Vec::new(), Vec::new()))
        .expect("D is a declaration");
    cx.declaring_class(&d)
}

/// `impl<A> C<F A> where <context> { }`.
fn measured_instance(cx: &Cx, context: Vec<RawConstraint>) -> Result<(), musa_core::ElabError> {
    declare_impl(
        cx,
        &instance(
            "C",
            vec![binder("A", type0())],
            vec![apply(var("F"), [var("A")])],
            context,
            Vec::new(),
        ),
    )
    .map(|_| ())
}

#[test]
fn an_instance_context_no_smaller_than_its_head_is_refused() {
    let cx = measured();

    // `impl<A> C<F A> where D<G A A>` is §4's pathological head: resolving
    // `C (F τ)` would ask for `D (G τ τ)`, which is larger than what was asked,
    // so lookup would not terminate. The measure catches it at the declaration,
    // where the author who wrote it can read the message.
    let Err(error) = measured_instance(&cx, vec![constraint("D", vec![apply(var("G"), [var("A"), var("A")])])]) else {
        panic!("an instance whose context grows was accepted");
    };
    let refused = refusal("a growing instance context", error);
    assert!(
        matches!(refused, Refusal::UnboundedInstance { .. }),
        "a growing instance context was refused as `{refused}`"
    );

    // The same instance with a context that *is* smaller is ordinary.
    assert!(
        measured_instance(&cx, vec![constraint("D", vec![var("A")])]).is_ok(),
        "an instance whose context is smaller than its head was refused"
    );
}

#[test]
fn a_type_variable_may_not_occur_more_often_in_the_context_than_in_the_head() {
    let cx = measured();

    // Small enough by size and still not decreasing: `G A A` inside `F` would
    // be, but the variable is duplicated, so each round doubles the work even as
    // the term shrinks. §4 counts occurrences for exactly this.
    let Err(error) = declare_impl(
        &cx,
        &instance(
            "C",
            vec![binder("A", type0())],
            vec![apply(var("F"), [apply(var("F"), [var("A")])])],
            vec![constraint("D", vec![apply(var("G"), [var("A"), var("A")])])],
            Vec::new(),
        ),
    ) else {
        panic!("an instance duplicating a type variable in its context was accepted");
    };
    let refused = refusal("a duplicated type variable", error);
    assert!(
        matches!(refused, Refusal::UnboundedInstance { .. }),
        "a duplicated type variable was refused as `{refused}`"
    );
}
