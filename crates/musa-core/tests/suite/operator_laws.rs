//! `docs/rules/language/10-traits.md` §5 and §6, stated as tests.
//!
//! §5's operators and §6's method syntax are one mechanism seen twice. An
//! operator is a spelling of a qualified call and a method is a spelling of the
//! same call with the trait left to be found, so what this suite states is that
//! the two spellings reach *one term* — not that a desugaring produced some
//! particular syntax, which is a fact about a printer.
//!
//! The surface spellings themselves are `musa-language`'s: this crate has no
//! `+`, and prompt 142 is where source text starts reaching either. What is here
//! is the half that decides meaning, and the negative half is the point of the
//! feature — §6's three rules exist to keep a lookup from becoming a search.

use std::sync::Arc;

use musa_core::{Cx, Instance, Raw, Refusal, Term, check, convertible, declare_impl, declare_trait, infer};

use crate::family_laws::{binder, nat_context, type0, var};
use crate::programs::WRITTEN;
use crate::trait_laws::{class, defines, instance, method};

/// A program §6 must refuse, and the refusal it owes.
///
/// Carried out of this file so `elaboration_laws.rs`'s coverage gate can reach
/// them: they need a context with two traits sharing a method name, which no
/// other corpus has a reason to declare.
pub(crate) struct RefusedMethod {
    /// What the program exercises.
    pub(crate) name: &'static str,
    /// The context it is written in.
    pub(crate) cx: Cx,
    pub(crate) raw: Raw,
    /// Whether the refusal is the one this program is about.
    pub(crate) expected: fn(&Refusal) -> bool,
}

/// `A → A → A`, the signature §5's table gives `Add`.
fn binary() -> Raw {
    Raw::pi(WRITTEN, "x", var("A"), Raw::pi(WRITTEN, "y", var("A"), var("A")))
}

/// `λx. λy. x` — an instance body, checked against `Nat → Nat → Nat`.
fn first() -> Raw {
    Raw::lam(WRITTEN, "x", Raw::lam(WRITTEN, "y", var("x")))
}

/// `impl <trait><Nat> { add = λx. λy. x; }`.
fn add_nat(cx: &Cx, class: &'static str) -> Arc<Instance> {
    declare_impl(
        cx,
        &instance(
            class,
            Vec::new(),
            vec![var("Nat")],
            Vec::new(),
            vec![defines("add", first())],
        ),
    )
    .expect("an instance at Nat is a declaration")
}

/// `data Box (A : Type 0) where Boxed : (x : A) → Box A`, on top of `Nat`.
///
/// A second head, so that "no trait has a method `add` *here*" is a question
/// about the head rather than about the method name.
fn box_context(cx: &Cx) -> Cx {
    use crate::family_laws::{constructor, data, family};
    let group = musa_core::declare(
        cx,
        &data(
            vec![binder("A", type0())],
            vec![family("Box", vec![constructor("Boxed", vec![binder("x", var("A"))])])],
        ),
    )
    .expect("Box is a declaration");
    cx.declaring(&group)
}

/// `Nat` and `Box` declared, `trait Add<A>` in scope, and `impl Add<Nat>`.
fn one_add() -> Cx {
    let (cx, _) = nat_context();
    let cx = box_context(&cx);
    let adding = declare_trait(
        &cx,
        &class(
            "Add",
            vec![binder("A", type0())],
            Vec::new(),
            vec![method("add", binary())],
        ),
    )
    .expect("Add is a declaration");
    let cx = cx.declaring_class(&adding);
    let at_nat = add_nat(&cx, "Add");
    cx.declaring_instance(&at_nat)
}

/// The same, plus a second trait whose method is spelled `add` and which also
/// has an instance at `Nat`.
///
/// Coherence does not rule this out and is not meant to: it keeps one instance
/// per (trait, head) pair, and two *different* traits at one head is what a
/// package importing two libraries has.
fn two_adds() -> Cx {
    let cx = one_add();
    let adjoining = declare_trait(
        &cx,
        &class(
            "Adjoin",
            vec![binder("A", type0())],
            Vec::new(),
            vec![method("add", binary())],
        ),
    )
    .expect("Adjoin is a declaration");
    let cx = cx.declaring_class(&adjoining);
    let at_nat = add_nat(&cx, "Adjoin");
    cx.declaring_instance(&at_nat)
}

/// `Nat`, as a core term to check against.
fn nat(cx: &Cx) -> Term {
    let (ty, _) = infer(cx, &var("Nat")).expect("Nat is a type");
    ty
}

/// §5 and §6's shared claim: a method call and the qualified call it resolves to
/// are one term.
///
/// The law is convertibility rather than a snapshot, and that is the whole
/// difference between "operators are a spelling" and "operators are a
/// desugaring we happen to perform". A snapshot would pass for an elaborator
/// that built the right syntax and gave it the wrong meaning, and fail for one
/// that built different syntax with the same meaning — both backwards.
#[test]
fn a_method_call_is_the_qualified_call() {
    let cx = one_add();
    let ty = nat(&cx);
    let zero = var("Nat.Zero");
    let written = Raw::app(WRITTEN, Raw::method(WRITTEN, zero.clone(), "add"), zero.clone());
    let qualified = Raw::app(WRITTEN, Raw::app(WRITTEN, var("Add.add"), zero.clone()), zero);
    let method = check(&cx, &ty, &written).expect("a method call at a known head resolves");
    let call = check(&cx, &ty, &qualified).expect("the qualified call is the same program written out");
    assert_eq!(
        convertible(&cx, &ty, &method, &call),
        Ok(true),
        "`x.add(y)` and `Add.add(x, y)` must be one term, not two that agree"
    );
}

/// Prompt 134's invariant, reaching a program with a trait in it: a call of a
/// method is a term the independent re-checker accepts.
///
/// `trait_laws.rs`'s `the_re_checker_accepts_what_dictionary_elaboration_produced`
/// states the same obligation, and the case it cannot reach is this one. A
/// solved dictionary hole is read back as a **normal form** at the method's
/// type: `Eq.equal : Nat` reads back as `Nat.Zero`, a constant, which infers —
/// but a normal form at a Π is η-long, so `Add.add : Nat → Nat → Nat` reads back
/// as `λx. λy. x`. A *call* of it is then an application whose function is an
/// introduction form, which `02-core-calculus.md` §2 gives no inference rule,
/// and prompt 134's obligation was discharged for exactly the programs with no
/// operator in them.
///
/// So the law is stated at a call rather than at the method: the bare method is
/// checked against a type and never needed one inferred, and it passes whether
/// or not the read-back writes anything down.
#[test]
fn a_call_of_a_method_re_checks_in_the_core() {
    let cx = one_add();
    let ty = nat(&cx);
    let zero = var("Nat.Zero");
    let binary_ty = Raw::pi(WRITTEN, "x", var("Nat"), Raw::pi(WRITTEN, "y", var("Nat"), var("Nat")));
    let (binary_ty, _) = infer(&cx, &binary_ty).expect("`Nat → Nat → Nat` is a type");

    for (_spelling, at, raw) in [
        (
            "Add.add(x, y)",
            &ty,
            Raw::app(WRITTEN, Raw::app(WRITTEN, var("Add.add"), zero.clone()), zero.clone()),
        ),
        (
            "x.add(y)",
            &ty,
            Raw::app(WRITTEN, Raw::method(WRITTEN, zero.clone(), "add"), zero),
        ),
        // The method as a value, which is the same read-back standing in a
        // checking position rather than an inferring one.
        ("Add.add", &binary_ty, var("Add.add")),
    ] {
        check(&cx, at, &raw).expect("`add` at `Nat` resolves");
    }
}

/// §6: exactly one candidate resolves, and the lookup is keyed on the receiver's
/// head rather than on what the call is checked against.
///
/// The second half is what makes this more than a restatement of the test
/// above: `Add` is in scope for `Nat` and not for `Box`, and the same three
/// tokens therefore resolve in one place and refuse in the other.
#[test]
fn resolution_is_keyed_on_the_receiver_and_nothing_else() {
    let cx = one_add();
    let ty = nat(&cx);
    let zero = var("Nat.Zero");
    let call = Raw::app(WRITTEN, Raw::method(WRITTEN, zero.clone(), "add"), zero);
    assert!(check(&cx, &ty, &call).is_ok(), "`Add` has an instance at `Nat`");

    let at_box = Raw::annotated_lam(
        WRITTEN,
        "b",
        Raw::app(WRITTEN, var("Box"), var("Nat")),
        Raw::method(WRITTEN, var("b"), "add"),
    );
    let Err(error) = infer(&cx, &at_box) else {
        panic!("`Add` has no instance at `Box`, so `.add` on one must be refused");
    };
    let refusal = crate::programs::refusal("a method with no trait at this head", error);
    let Refusal::NoMethodForType { head, method, .. } = &refusal else {
        panic!("expected a missing method, got `{refusal}`");
    };
    assert_eq!(&**head, "Box", "the report names the type that has no such method");
    assert_eq!(&**method, "add", "and the method that was written");
}

/// §6's first refusal, and the one the feature exists for: a value of a generic
/// parameter never acquires a method.
///
/// The message has to send the author somewhere real, and "add the constraint"
/// would be wrong here — `where Add<A>` supplies the dictionary and still leaves
/// `.add` unresolvable, because the scan it would need is the thing §9 refuses.
/// The repair §6 names is qualification, which always resolves, so that is what
/// the message says.
#[test]
fn a_method_on_a_type_variable_is_refused_and_names_the_repair() {
    let cx = one_add();
    let program = Raw::annotated_lam(
        WRITTEN,
        "A",
        type0(),
        Raw::annotated_lam(
            WRITTEN,
            "x",
            var("A"),
            Raw::app(WRITTEN, Raw::method(WRITTEN, var("x"), "add"), var("x")),
        ),
    );
    let Err(error) = infer(&cx, &program) else {
        panic!("a method on a generic parameter must be refused");
    };
    let refusal = crate::programs::refusal("a method on a generic parameter", error);
    assert!(
        matches!(refusal, Refusal::MethodOnVariable { .. }),
        "expected a method on a variable, got `{refusal}`"
    );
    assert_eq!(
        refusal.to_string(),
        "`.add` needs a receiver whose type is a declared type; write `Trait.add(…)` instead"
    );
}

/// §6's second refusal: two traits declaring one method name, both in scope for
/// the head, is an error naming both.
#[test]
fn two_candidates_are_an_error_naming_both() {
    let cx = two_adds();
    let ty = nat(&cx);
    let zero = var("Nat.Zero");
    let call = Raw::app(WRITTEN, Raw::method(WRITTEN, zero.clone(), "add"), zero);
    let Err(error) = check(&cx, &ty, &call) else {
        panic!("two traits declaring `add` for `Nat` must be an ambiguity rather than a choice");
    };
    let refusal = crate::programs::refusal("a method two traits declare", error);
    let Refusal::AmbiguousMethod { classes, .. } = &refusal else {
        panic!("expected an ambiguous method, got `{refusal}`");
    };
    let named: Vec<&str> = classes.iter().map(|class| &**class).collect();
    assert_eq!(
        named,
        vec!["Add", "Adjoin"],
        "both traits are named, in an order a hash map cannot change"
    );
}

/// Every §6 refusal, with the program that reaches it.
///
/// # Panics
///
/// If a context this suite builds is itself refused, which would be a defect in
/// the crate rather than a property of any law.
pub(crate) fn refused_methods() -> Vec<RefusedMethod> {
    let one = one_add();
    let call = |receiver: Raw| Raw::app(WRITTEN, Raw::method(WRITTEN, receiver, "add"), var("Nat.Zero"));
    vec![
        RefusedMethod {
            name: "a method on a generic parameter",
            cx: one.clone(),
            raw: Raw::annotated_lam(
                WRITTEN,
                "A",
                type0(),
                Raw::annotated_lam(WRITTEN, "x", var("A"), call(var("x"))),
            ),
            expected: |refusal| matches!(refusal, Refusal::MethodOnVariable { .. }),
        },
        RefusedMethod {
            name: "a method no trait declares for this head",
            cx: one,
            raw: Raw::annotated_lam(
                WRITTEN,
                "b",
                Raw::app(WRITTEN, var("Box"), var("Nat")),
                Raw::method(WRITTEN, var("b"), "add"),
            ),
            expected: |refusal| matches!(refusal, Refusal::NoMethodForType { .. }),
        },
        RefusedMethod {
            name: "a method two traits in scope declare",
            cx: two_adds(),
            raw: call(var("Nat.Zero")),
            expected: |refusal| matches!(refusal, Refusal::AmbiguousMethod { .. }),
        },
        RefusedMethod {
            name: "a method whose target nothing determines",
            // `xs.collect()` in an inferring position: the target would have to
            // be chosen from a result type nobody wrote — return-type-directed
            // selection, which §2.1 refuses as the unsolved hole it is.
            cx: crate::collection_laws::context(),
            raw: Raw::method(WRITTEN, crate::collection_laws::numbers([0, 1]), "collect"),
            expected: |refusal| matches!(refusal, Refusal::Unsolved { .. }),
        },
    ]
}
