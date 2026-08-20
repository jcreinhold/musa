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
    // Two parameters and no `where` (§1's flat law): what the law below needs
    // is a trait whose key is its *first* argument, so that the second can be
    // the variable a local dictionary constrains.
    class(
        "Held",
        vec![binder("K", type0()), binder("V", type0())],
        Vec::new(),
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

/// `impl<A> Held<Box A, A> { held = Eq.equal; }` — an instance whose method
/// needs `Eq<A>`, so that who answers it (or refuses) is observable.
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
fn a_constraint_on_a_variable_no_local_dictionary_answers_is_refused() {
    let cx = holding();

    // `held = Eq.equal` at the variable `A`: the flat law gives an `impl` no
    // `where` to bind a dictionary with — the author who wants one writes the
    // dictionary-building function — so the constraint is the variable's to
    // answer for, and `impl Eq<Nat>` in scope cannot help: a type variable is
    // not `Nat`, and no instance for a variable can ever be declared.
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
            vec![family("F", vec![constructor("Wrapped", vec![binder("a", var("A"))])])],
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
                vec![constructor("Both", vec![binder("a", var("A")), binder("b", var("B"))])],
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
fn an_instance_context_is_refused_however_it_decreases() {
    let cx = measured();

    // §4's flat law admits no `where` clause on an instance at all — the
    // decreasing measure that used to sort them is what the clause would have
    // been measured *for*, and both went together. `impl<A> C<F A> where
    // D<G A A>` is the pathological shape; `… where D<A>` would have been the
    // admissible one; the law refuses both with the same sentence, because what
    // the author wants is the explicit dictionary-building function.
    for context in [
        vec![constraint("D", vec![apply(var("G"), [var("A"), var("A")])])],
        vec![constraint("D", vec![var("A")])],
    ] {
        let Err(error) = measured_instance(&cx, context) else {
            panic!("an instance carrying a context was accepted");
        };
        let refused = refusal("an instance context", error);
        assert!(
            matches!(refused, Refusal::ConstrainedInstance { .. }),
            "an instance context was refused as `{refused}`"
        );
    }
}

#[test]
fn an_instance_context_duplicating_a_variable_is_refused_like_any_other() {
    let cx = measured();

    // `G A A` in the context used to be the case the occurrence count existed
    // for. Under the flat law it is one more context, refused with the rest.
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
        matches!(refused, Refusal::ConstrainedInstance { .. }),
        "a duplicated type variable was refused as `{refused}`"
    );
}
