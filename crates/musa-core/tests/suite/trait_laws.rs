//! `docs/rules/language/10-traits.md` §1–§4, stated as tests.
//!
//! Every check this suite makes is made at a **declaration**, which is the
//! specification's rule and not an implementation choice: a use site is the
//! wrong place to learn that a library cannot answer, because the author reading
//! the message is not the author who can fix it. So the programs here are
//! declarations rather than terms, and they run through
//! [`declare_trait`](musa_core::declare_trait) and
//! [`declare_impl`](musa_core::declare_impl) rather than through `check`.

use std::sync::Arc;

use musa_core::{
    Cx, ElabError, Instance, PackageId, Raw, RawBinder, RawConstraint, RawDefinition, RawImpl, RawMethod, RawTrait,
    Refusal, Visibility, declare_impl, declare_trait,
};

use crate::family_laws::{binder, constructor, data, family, nat_context, type0, var};
use crate::programs::WRITTEN;

/// A declaration elaboration must refuse, and the refusal it owes.
///
/// The outcome is carried rather than the program, because a trait and an impl
/// are refused by different entry points and a caller that had to know which
/// would be testing the dispatch rather than the rule.
pub(crate) struct RefusedDeclaration {
    /// What the declaration exercises.
    pub(crate) name: &'static str,
    /// What elaborating it answered.
    pub(crate) outcome: Result<(), ElabError>,
    /// Whether the refusal is the one this declaration is about.
    pub(crate) expected: fn(&Refusal) -> bool,
}

pub(crate) fn method(name: &'static str, ty: Raw) -> RawMethod {
    RawMethod {
        origin: WRITTEN,
        name: Arc::from(name),
        ty,
        body: None,
    }
}

pub(crate) fn derived(name: &'static str, ty: Raw, body: Raw) -> RawMethod {
    RawMethod {
        origin: WRITTEN,
        name: Arc::from(name),
        ty,
        body: Some(body),
    }
}

pub(crate) fn defines(name: &'static str, value: Raw) -> RawDefinition {
    RawDefinition {
        origin: WRITTEN,
        name: Arc::from(name),
        value,
    }
}

pub(crate) fn constraint(name: &'static str, args: Vec<Raw>) -> RawConstraint {
    RawConstraint {
        origin: WRITTEN,
        name: Arc::from(name),
        args,
    }
}

pub(crate) fn class(
    name: &'static str,
    params: Vec<RawBinder>,
    context: Vec<RawConstraint>,
    methods: Vec<RawMethod>,
) -> RawTrait {
    RawTrait {
        origin: WRITTEN,
        name: Arc::from(name),
        visibility: Visibility::Public,
        params,
        context,
        methods,
    }
}

pub(crate) fn instance(
    name: &'static str,
    params: Vec<RawBinder>,
    args: Vec<Raw>,
    context: Vec<RawConstraint>,
    methods: Vec<RawDefinition>,
) -> RawImpl {
    RawImpl {
        origin: WRITTEN,
        name: Arc::from(name),
        params,
        args,
        context,
        methods,
    }
}

/// `trait Eq<A> { fn equal : A; }`.
///
/// One parameter and one required method whose type is the parameter, which is
/// the smallest thing that is genuinely a trait: a head to key on, and a field
/// whose type an instance has to satisfy.
pub(crate) fn eq() -> RawTrait {
    class(
        "Eq",
        vec![binder("A", type0())],
        Vec::new(),
        vec![method("equal", var("A"))],
    )
}

/// `trait Ord<A> where Eq<A> { fn least : A; }`.
///
/// The super-constraint is the point: an `impl Ord<τ>` has to *resolve*
/// `Eq<τ>` to fill the field, which is what makes declaration and resolution one
/// mechanism rather than two.
pub(crate) fn ord() -> RawTrait {
    class(
        "Ord",
        vec![binder("A", type0())],
        vec![constraint("Eq", vec![var("A")])],
        vec![method("least", var("A"))],
    )
}

/// `trait Keyed<K, V> where Eq<V> { fn key : K; }`.
///
/// Two parameters, with the super-constraint on the *second* one. An instance
/// is filed under the head of its **first** argument, so this is the only shape
/// in which a super-constraint can land on a bare type variable while the
/// instance itself still has a rigid head to be filed under — which is exactly
/// the case §4 refuses as unconstrained rather than as unresolved.
pub(crate) fn keyed() -> RawTrait {
    class(
        "Keyed",
        vec![binder("K", type0()), binder("V", type0())],
        vec![constraint("Eq", vec![var("V")])],
        vec![method("key", var("K"))],
    )
}

/// `data Box (A : Type 0) where Boxed : (x : A) → Box A`.
///
/// A parameterized family, so that an instance can have a head that mentions a
/// type variable — which is what every question about the termination measure
/// and about a variable head needs.
pub(crate) fn boxed_context(cx: &Cx) -> Cx {
    let group = musa_core::declare(
        cx,
        &data(
            vec![binder("A", type0())],
            vec![family(
                "Box",
                Vec::new(),
                vec![constructor("Boxed", vec![binder("x", var("A"))], Vec::new())],
            )],
        ),
    )
    .expect("Box is a declaration");
    cx.declaring(&group)
}

/// `Nat` and `Box` declared, with `Eq` and `Ord` in scope.
pub(crate) fn context() -> Cx {
    let (cx, _) = nat_context();
    let cx = boxed_context(&cx);
    let equality = declare_trait(&cx, &eq()).expect("Eq is a declaration");
    let cx = cx.declaring_class(&equality);
    let ordering = declare_trait(&cx, &ord()).expect("Ord is a declaration");
    let cx = cx.declaring_class(&ordering);
    let keying = declare_trait(&cx, &keyed()).expect("Keyed is a declaration");
    cx.declaring_class(&keying)
}

/// `impl Eq<Nat> { equal = Nat.Zero; }`, which several laws need in scope.
pub(crate) fn eq_nat(cx: &Cx) -> Arc<Instance> {
    declare_impl(
        cx,
        &instance(
            "Eq",
            Vec::new(),
            vec![var("Nat")],
            Vec::new(),
            vec![defines("equal", var("Nat.Zero"))],
        ),
    )
    .expect("Eq for Nat is an instance")
}

/// Every declaration `10-traits.md` refuses, each with the refusal it owes.
///
/// # Panics
///
/// If a context this suite builds is itself refused, which would be a defect in
/// the crate rather than a property of any law.
pub(crate) fn refused_declarations() -> Vec<RefusedDeclaration> {
    let cx = context();
    let with_eq_nat = cx.declaring_instance(&eq_nat(&cx));

    // §3's orphan rule needs three packages' worth of setting: the trait and the
    // head type at home in one, and the instance written in another.
    let orphan = {
        let home = Cx::new().in_package(PackageId::new(1));
        let group = musa_core::declare(&home, &crate::family_laws::nat()).expect("Nat is a declaration");
        let home = home.declaring(&group);
        let equality = declare_trait(&home, &eq()).expect("Eq is a declaration");
        home.declaring_class(&equality).in_package(PackageId::new(2))
    };

    vec![
        RefusedDeclaration {
            name: "a trait named `Storable`",
            outcome: declare_trait(
                &cx,
                &class("Storable", vec![binder("A", type0())], Vec::new(), Vec::new()),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::ReservedClass { .. }),
        },
        RefusedDeclaration {
            name: "a trait with no parameters",
            outcome: declare_trait(&cx, &class("Global", Vec::new(), Vec::new(), Vec::new())).map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::HeadlessClass { .. }),
        },
        RefusedDeclaration {
            name: "a trait declaring one method twice",
            outcome: declare_trait(
                &cx,
                &class(
                    "Twice",
                    vec![binder("A", type0())],
                    Vec::new(),
                    vec![method("equal", var("A")), method("equal", var("A"))],
                ),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::DuplicateMethod { .. }),
        },
        RefusedDeclaration {
            name: "an instance written with too many arguments",
            outcome: declare_impl(
                &cx,
                &instance(
                    "Eq",
                    Vec::new(),
                    vec![var("Nat"), var("Nat")],
                    Vec::new(),
                    vec![defines("equal", var("Nat.Zero"))],
                ),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::ClassArity { .. }),
        },
        RefusedDeclaration {
            name: "a hand-written `Storable` instance",
            outcome: declare_impl(
                &cx,
                &instance("Storable", Vec::new(), vec![var("Nat")], Vec::new(), Vec::new()),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::HandWrittenStorable { .. }),
        },
        RefusedDeclaration {
            name: "an instance for every type at once",
            outcome: declare_impl(
                &cx,
                &instance(
                    "Eq",
                    vec![binder("A", type0())],
                    vec![var("A")],
                    Vec::new(),
                    vec![defines("equal", var("Nat.Zero"))],
                ),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::BlanketInstance { .. }),
        },
        RefusedDeclaration {
            name: "a second instance for a key one already answers",
            outcome: declare_impl(
                &with_eq_nat,
                &instance(
                    "Eq",
                    Vec::new(),
                    vec![var("Nat")],
                    Vec::new(),
                    vec![defines("equal", var("Nat.Zero"))],
                ),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::DuplicateInstance { .. }),
        },
        RefusedDeclaration {
            name: "an instance away from both its trait and its head type",
            outcome: declare_impl(
                &orphan,
                &instance(
                    "Eq",
                    Vec::new(),
                    vec![var("Nat")],
                    Vec::new(),
                    vec![defines("equal", var("Nat.Zero"))],
                ),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::OrphanInstance { .. }),
        },
        RefusedDeclaration {
            name: "an instance whose context is no smaller than its head",
            outcome: declare_impl(
                &cx,
                &instance(
                    "Eq",
                    Vec::new(),
                    vec![var("Nat")],
                    vec![constraint("Eq", vec![var("Nat")])],
                    vec![defines("equal", var("Nat.Zero"))],
                ),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::UnboundedInstance { .. }),
        },
        RefusedDeclaration {
            name: "an instance defining a method its trait derives",
            outcome: {
                let derived_class = declare_trait(
                    &cx,
                    &class(
                        "Same",
                        vec![binder("A", type0())],
                        Vec::new(),
                        vec![method("one", var("A")), derived("other", var("A"), var("one"))],
                    ),
                )
                .expect("a trait with a derived method is a declaration");
                declare_impl(
                    &cx.declaring_class(&derived_class),
                    &instance(
                        "Same",
                        Vec::new(),
                        vec![var("Nat")],
                        Vec::new(),
                        vec![defines("one", var("Nat.Zero")), defines("other", var("Nat.Zero"))],
                    ),
                )
                .map(|_| ())
            },
            expected: |refusal| matches!(refusal, Refusal::DerivedMethod { .. }),
        },
        RefusedDeclaration {
            name: "an instance defining a method its trait does not declare",
            outcome: declare_impl(
                &cx,
                &instance(
                    "Eq",
                    Vec::new(),
                    vec![var("Nat")],
                    Vec::new(),
                    vec![defines("equals", var("Nat.Zero"))],
                ),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::NoSuchMethod { .. }),
        },
        RefusedDeclaration {
            name: "an instance leaving a required method undefined",
            outcome: declare_impl(
                &cx,
                &instance("Eq", Vec::new(), vec![var("Nat")], Vec::new(), Vec::new()),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::MissingMethod { .. }),
        },
        RefusedDeclaration {
            // The super-constraint is what makes this reachable at a
            // declaration: `Ord<Nat>` needs `Eq<Nat>` for its field, the head is
            // known, and the table has no entry.
            name: "a super-constraint nothing implements",
            outcome: declare_impl(
                &cx,
                &instance(
                    "Ord",
                    Vec::new(),
                    vec![var("Nat")],
                    Vec::new(),
                    vec![defines("least", var("Nat.Zero"))],
                ),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::UnresolvedInstance { .. }),
        },
        RefusedDeclaration {
            // The same constraint at a *variable* head, which no global instance
            // can ever answer — so the repair is on the signature and the
            // refusal says so.
            name: "a super-constraint on a type variable nothing constrains",
            outcome: declare_impl(
                &cx,
                &instance(
                    "Keyed",
                    vec![binder("A", type0())],
                    vec![var("Nat"), var("A")],
                    Vec::new(),
                    vec![defines("key", var("Nat.Zero"))],
                ),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::UnconstrainedVariable { .. }),
        },
        RefusedDeclaration {
            // `02-core-calculus.md` §1.2's arrow, reached as an ordinary
            // constraint: the key would have to be filed under a name and a
            // function type has none, so no author could write the repair.
            name: "a super-constraint on a function type",
            outcome: declare_impl(
                &cx,
                &instance(
                    "Keyed",
                    Vec::new(),
                    vec![var("Nat"), Raw::pi(WRITTEN, "n", var("Nat"), var("Nat"))],
                    Vec::new(),
                    vec![defines("key", var("Nat.Zero"))],
                ),
            )
            .map(|_| ()),
            expected: |refusal| matches!(refusal, Refusal::UnkeyedConstraint { .. }),
        },
    ]
}

/// `trait Payload<A> where Storable<A> { }`.
///
/// The only way an author may name `Storable`: as a *requirement*. Every law
/// below about which types are storable is stated through this trait, because a
/// constraint is only observable where something has to discharge it, and this
/// is the smallest declaration that does.
pub(crate) fn payload() -> RawTrait {
    class(
        "Payload",
        vec![binder("A", type0())],
        vec![constraint("Storable", vec![var("A")])],
        Vec::new(),
    )
}

/// The shared context with `Payload` in scope as well.
fn storing() -> Cx {
    let cx = context();
    let carrier = declare_trait(&cx, &payload()).expect("Payload is a declaration");
    cx.declaring_class(&carrier)
}

/// Whether `Payload<τ>` can be declared, which is whether `τ` is storable.
fn storable(cx: &Cx, params: Vec<RawBinder>, argument: Raw, context: Vec<RawConstraint>) -> Result<(), ElabError> {
    declare_impl(cx, &instance("Payload", params, vec![argument], context, Vec::new())).map(|_| ())
}

#[test]
fn a_use_of_a_method_becomes_the_instance_that_answers_it() {
    let cx = context();
    let cx = cx.declaring_instance(&eq_nat(&cx));
    let nat = crate::family_laws::core_constant(&cx, "Nat");
    let term = musa_core::check(&cx, &nat, &var("Eq.equal")).expect("`Eq.equal` at `Nat` resolves");

    // The instance defines `equal = Nat.Zero`, so a resolved use is convertible
    // to it. Convertible rather than syntactically equal: what elaboration
    // produces is a projection out of the instance's dictionary, and it is β and
    // the record rule that make the two one term.
    let zero = crate::family_laws::core_constant(&cx, "Nat.Zero");
    assert!(
        musa_core::convertible(&cx, &nat, &term, &zero).expect("conversion is decidable"),
        "a resolved method use is not the definition the instance gave it"
    );
}

#[test]
fn the_re_checker_accepts_what_dictionary_elaboration_produced() {
    let cx = context();
    let cx = cx.declaring_instance(&eq_nat(&cx));
    let nat = crate::family_laws::core_constant(&cx, "Nat");
    let term = musa_core::check(&cx, &nat, &var("Eq.equal")).expect("`Eq.equal` at `Nat` resolves");

    // The point of the independent re-checker (prompt 134) is that no stage may
    // vouch for its own output. A dictionary is the first term in this crate
    // that nobody wrote, so it is the first one where that matters.
    if let Err(error) = musa_core::well_typed(&cx, &nat, &term) {
        panic!("the re-checker rejects a term dictionary elaboration produced: {error}");
    }
}

#[test]
fn a_constraint_no_argument_determines_is_reported_as_the_hole_it_is() {
    let cx = context();
    let cx = cx.declaring_instance(&eq_nat(&cx));

    // `Eq.equal` says which trait and not at which type, so §4 postpones. With
    // nothing to check it against, nothing ever determines the head, and the
    // report is the one for an unsolved hole rather than a second report saying
    // the same thing about instances.
    let outcome = musa_core::infer(&cx, &var("Eq.equal"));
    let Err(error) = outcome else {
        panic!("a method use at no particular type was accepted");
    };
    let refusal = crate::programs::refusal("an undetermined method use", error);
    assert!(
        matches!(refusal, Refusal::Unsolved { .. }),
        "an undetermined method use was refused as `{refusal}`"
    );
}

#[test]
fn a_family_is_storable_when_every_field_it_stores_is() {
    let cx = storing();
    assert!(
        storable(&cx, Vec::new(), var("Nat"), Vec::new()).is_ok(),
        "`Nat` stores only `Nat`s and is not storable"
    );
}

#[test]
fn a_parameterized_family_is_storable_exactly_when_its_parameter_is() {
    let cx = storing();
    let boxed = || crate::family_laws::apply(var("Box"), [var("A")]);

    if let Err(error) = storable(
        &cx,
        vec![binder("A", type0())],
        boxed(),
        vec![constraint("Storable", vec![var("A")])],
    ) {
        panic!("`Box A` is not storable even where `A` is: {error}");
    }

    // Without the constraint the parameter could be anything, including a
    // function — which is the whole reason §1.2 is a constraint rather than a
    // property of a finished type.
    let unconstrained = storable(&cx, vec![binder("A", type0())], boxed(), Vec::new());
    let Err(error) = unconstrained else {
        panic!("`Box A` was storable for an `A` nothing constrains");
    };
    let refusal = crate::programs::refusal("an unconstrained parameter", error);
    assert!(
        matches!(refusal, Refusal::UnconstrainedVariable { .. }),
        "an unconstrained parameter was refused as `{refusal}`"
    );
}

#[test]
fn a_family_that_stores_a_function_is_not_storable() {
    let cx = storing();
    let group = musa_core::declare(
        &cx,
        &data(
            Vec::new(),
            vec![family(
                "Rule",
                Vec::new(),
                vec![constructor(
                    "Made",
                    vec![binder("f", Raw::pi(WRITTEN, "n", var("Nat"), var("Nat")))],
                    Vec::new(),
                )],
            )],
        ),
    )
    .expect("Rule is a declaration");
    let cx = cx.declaring(&group);

    // No instance is generated, and none can be written, so the report is the
    // ordinary one for a trait nothing implements — which is what §1.2 asks for:
    // "its failures are ordinary instance errors".
    let Err(error) = storable(&cx, Vec::new(), var("Rule"), Vec::new()) else {
        panic!("a family storing a function was storable");
    };
    let refusal = crate::programs::refusal("a family storing a function", error);
    assert!(
        matches!(refusal, Refusal::UnresolvedInstance { .. }),
        "a family storing a function was refused as `{refusal}`"
    );
}

#[test]
fn a_family_holding_a_function_at_depth_is_not_storable() {
    let cx = storing();
    let group = musa_core::declare(
        &cx,
        &data(
            Vec::new(),
            vec![family(
                "Held",
                Vec::new(),
                vec![constructor(
                    "Holding",
                    // `Box (Nat → Nat)`: nothing at the surface of this field is
                    // a function, which is exactly why §1.2 makes the check
                    // structural rather than a rule about what an author wrote.
                    vec![binder(
                        "b",
                        crate::family_laws::apply(var("Box"), [Raw::pi(WRITTEN, "n", var("Nat"), var("Nat"))]),
                    )],
                    Vec::new(),
                )],
            )],
        ),
    )
    .expect("Held is a declaration");
    let cx = cx.declaring(&group);

    let Err(error) = storable(&cx, Vec::new(), var("Held"), Vec::new()) else {
        panic!("a family holding a function inside a container was storable");
    };
    let refusal = crate::programs::refusal("a container holding a function", error);
    assert!(
        matches!(refusal, Refusal::UnresolvedInstance { .. }),
        "a container holding a function was refused as `{refusal}`"
    );
}

#[test]
fn storable_is_in_scope_before_anything_is_declared() {
    // §1.2 gives no declaration that introduces `Storable`, so a context that
    // declared nothing must still be able to require it. The empty context is
    // the strongest form of that question.
    let cx = Cx::new();
    let carrier = declare_trait(&cx, &payload()).expect("`Storable` is nameable in the empty context");

    // And still in scope after that declaration rather than consumed by it: a
    // built-in seeded into the table would be indistinguishable from one written
    // at the first use if only the first use ever asked.
    let cx = cx.declaring_class(&carrier);
    assert!(
        declare_trait(
            &cx,
            &class(
                "Cargo",
                vec![binder("A", type0())],
                vec![constraint("Storable", vec![var("A")])],
                Vec::new(),
            ),
        )
        .is_ok(),
        "`Storable` left scope once a declaration required it"
    );
}
