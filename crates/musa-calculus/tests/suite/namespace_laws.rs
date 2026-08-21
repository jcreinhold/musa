//! `docs/rules/language/01-surface.md` §1.5, stated as tests.
//!
//! A namespace is a dotted name. `impl Pitch { fn act(…) }` is the definition
//! `Pitch.act`, and the three ways to reach it — the written path, method
//! syntax, and an operator — end at that one definition. So what this suite
//! states is that the spellings reach *one term*, not that a desugaring produced
//! some particular syntax, which is a fact about a printer.
//!
//! The surface spellings themselves are `musa-syntax`'s: this crate has no `+`
//! and no `impl`. What is here is the half that decides meaning, and the
//! negative half is the point of the feature — the three refusals exist to keep
//! a lookup from becoming a search.

use musa_calculus::{Cx, Raw, RawProgram, RawTopLevel, Refusal, Term, Visibility, check, convertible, infer};

use crate::family_laws::{binder, constructor, data, family, nat_context, type0, var};
use crate::programs::WRITTEN;

/// A program §1.5 must refuse, and the refusal it owes.
///
/// Carried out of this file so `elaboration_laws.rs`'s coverage gate can reach
/// them: they need a context with one member spelled in two namespaces, which
/// no other corpus has a reason to declare.
pub(crate) struct RefusedMethod {
    /// What the program exercises.
    pub(crate) name: &'static str,
    /// The context it is written in.
    pub(crate) cx: Cx,
    pub(crate) raw: Raw,
    /// Whether the refusal is the one this program is about.
    pub(crate) expected: fn(&Refusal) -> bool,
}

fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(WRITTEN, "_", domain, codomain)
}

/// `Nat → Nat → Nat`, the signature §1.5's table gives `+`.
fn binary() -> Raw {
    arrow(var("Nat"), arrow(var("Nat"), var("Nat")))
}

/// `λx. λy. x`.
fn first() -> Raw {
    Raw::lam(WRITTEN, "x", Raw::lam(WRITTEN, "y", var("x")))
}

/// `name : ty = value`, public and in no module.
pub(crate) fn definition(name: &str, ty: Raw, value: Raw) -> RawTopLevel {
    RawTopLevel {
        origin: WRITTEN,
        name: name.into(),
        visibility: Visibility::Public,
        module: None,
        ty: Some(ty),
        value,
    }
}

/// `cx` with `definitions` in it.
///
/// # Panics
///
/// If the group is refused, which every caller says it is not.
pub(crate) fn defining(cx: &Cx, definitions: Vec<RawTopLevel>) -> Cx {
    let declared =
        musa_calculus::declare_program(cx, &RawProgram { definitions }).expect("the group is a set of declarations");
    cx.defining(&declared)
}

/// `data Box (A : Type 0) where Boxed : (x : A) → Box A`.
///
/// A parameterized family, so that a receiver can have a head no definition
/// spells — which is what "no member `add` *here*" needs to be a question about
/// the head rather than about the name.
pub(crate) fn boxed_context(cx: &Cx) -> Cx {
    let group = musa_calculus::declare(
        cx,
        &data(
            vec![binder("A", type0())],
            vec![family("Box", vec![constructor("Boxed", vec![binder("x", var("A"))])])],
        ),
    )
    .expect("Box is a declaration");
    cx.declaring(&group)
}

/// `Nat` and `Box` declared, and `Nat.add` defined.
pub(crate) fn one_add() -> Cx {
    let (cx, _) = nat_context();
    let cx = boxed_context(&cx);
    defining(&cx, vec![definition("Nat.add", binary(), first())])
}

/// `Nat` and `Box` each spelling two members, `add` and `unit`.
///
/// Two heads, one member name, twice over, and nothing rules that out: any two
/// libraries naming one operation have this. The pair is here because the two
/// sit at the two shapes §1.5's rules are about — `add` takes its namespace's
/// type as an argument, so a **receiver** fixes the head, and a unit's type *is*
/// its namespace, so an expected **type** fixes it. That is
/// `stdlib/src/algebra.musa`'s own shape, where `P5.unit()` is `P1`.
fn two_namespaces() -> Cx {
    let cx = one_add();
    let boxed = Raw::app(WRITTEN, var("Box"), var("Nat"));
    let held = Raw::app(
        WRITTEN,
        Raw::app(WRITTEN, var("Box.Boxed"), var("Nat")),
        var("Nat.Zero"),
    );
    defining(
        &cx,
        vec![
            definition(
                "Box.add",
                arrow(boxed.clone(), arrow(boxed.clone(), boxed.clone())),
                Raw::lam(WRITTEN, "x", Raw::lam(WRITTEN, "y", var("x"))),
            ),
            definition("Nat.unit", var("Nat"), var("Nat.Zero")),
            definition("Box.unit", boxed, held),
        ],
    )
}

/// `Nat`, as a core term to check against.
fn nat(cx: &Cx) -> Term {
    let (ty, _) = infer(cx, &var("Nat")).expect("Nat is a type");
    ty
}

/// §1.5's shared claim: a method call and the written path are one term.
///
/// The law is convertibility rather than a snapshot, and that is the whole
/// difference between "the path is a spelling" and "the path is a desugaring we
/// happen to perform". A snapshot would pass for an elaborator that built the
/// right syntax and gave it the wrong meaning, and fail for one that built
/// different syntax with the same meaning — both backwards.
#[test]
fn a_method_call_is_the_written_path() {
    let cx = one_add();
    let ty = nat(&cx);
    let zero = var("Nat.Zero");
    let written = Raw::app(WRITTEN, Raw::method(WRITTEN, zero.clone(), "add"), zero.clone());
    let qualified = Raw::app(WRITTEN, Raw::app(WRITTEN, var("Nat.add"), zero.clone()), zero);
    let method = check(&cx, &ty, &written).expect("a method call at a known head resolves");
    let call = check(&cx, &ty, &qualified).expect("the path is the same program written out");
    assert_eq!(
        convertible(&cx, &ty, &method, &call),
        Ok(true),
        "`x.add(y)` and `Nat::add(x, y)` must be one term, not two that agree"
    );
}

/// Prompt 134's invariant, reaching a program that uses a namespace: the term a
/// member call elaborates to is one the independent re-checker accepts.
#[test]
fn a_call_of_a_member_re_checks_in_the_core() {
    let cx = one_add();
    let ty = nat(&cx);
    let zero = var("Nat.Zero");
    let (binary_ty, _) = infer(&cx, &binary()).expect("`Nat → Nat → Nat` is a type");

    for (_spelling, at, raw) in [
        (
            "Nat::add(x, y)",
            &ty,
            Raw::app(WRITTEN, Raw::app(WRITTEN, var("Nat.add"), zero.clone()), zero.clone()),
        ),
        (
            "x.add(y)",
            &ty,
            Raw::app(WRITTEN, Raw::method(WRITTEN, zero.clone(), "add"), zero),
        ),
        // The member as a value, which is the same definition standing in a
        // checking position rather than an inferring one.
        ("Nat.add", &binary_ty, var("Nat.add")),
    ] {
        check(&cx, at, &raw).expect("`add` at `Nat` resolves");
    }
}

/// §1.5: the lookup is keyed on the receiver's head and on nothing else.
///
/// `Nat.add` is defined and `Box.add` is not, so the same three tokens resolve
/// in one place and refuse in the other.
#[test]
fn resolution_is_keyed_on_the_receiver_and_nothing_else() {
    let cx = one_add();
    let ty = nat(&cx);
    let zero = var("Nat.Zero");
    let call = Raw::app(WRITTEN, Raw::method(WRITTEN, zero.clone(), "add"), zero);
    assert!(check(&cx, &ty, &call).is_ok(), "`Nat.add` is defined");

    let at_box = Raw::annotated_lam(
        WRITTEN,
        "b",
        Raw::app(WRITTEN, var("Box"), var("Nat")),
        Raw::method(WRITTEN, var("b"), "add"),
    );
    let Err(error) = infer(&cx, &at_box) else {
        panic!("nothing spells `Box.add`, so `.add` on one must be refused");
    };
    let refusal = crate::programs::refusal("a member no namespace spells for this head", error);
    let Refusal::NoMethodForType { head, method, .. } = &refusal else {
        panic!("expected a missing method, got `{refusal}`");
    };
    assert_eq!(&**head, "Box", "the report names the type that has no such member");
    assert_eq!(&**method, "add", "and the member that was written");
}

/// §1.5's first refusal, and the one the feature exists for: a value of a
/// generic parameter never acquires a member.
///
/// `A` heads nothing, so there is no name to look up — and the repair the
/// message names is the written path, which resolves without asking what the
/// receiver's type is.
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
        panic!("a member on a generic parameter must be refused");
    };
    let refusal = crate::programs::refusal("a member on a generic parameter", error);
    assert!(
        matches!(refusal, Refusal::MethodOnVariable { .. }),
        "expected a method on a variable, got `{refusal}`"
    );
    assert_eq!(
        refusal.to_string(),
        "`.add` needs a receiver whose type is a declared type; write `Head::add(…)` instead"
    );
}

/// §1.5's second rule, and the one place the expected type decides which name a
/// spelling means: a **bare** member two namespaces answer is filtered by the
/// head the site fixes.
///
/// The filter compares two names. `unit` at `Nat` is `Nat.unit` because `Nat`
/// heads the goal, and `unit` at `Box Nat` is `Box.unit` for the same reason —
/// nothing was elaborated in order to be discarded, so no reordering of the
/// candidates could answer differently. That is the whole difference from trial
/// elaboration, which Idris2's `Ambiguity.idr` needs because its surface admits
/// an ambiguity this one does not.
#[test]
fn a_bare_member_is_filtered_by_the_head_the_site_fixes() {
    let cx = two_namespaces();
    for (spelled, written, expected) in [
        ("Nat", var("Nat"), var("Nat.unit")),
        ("Box", Raw::app(WRITTEN, var("Box"), var("Nat")), var("Box.unit")),
    ] {
        let (ty, _) = infer(&cx, &written).expect("the goal is a type");
        let bare = check(&cx, &ty, &var("unit")).expect("one candidate survives the filter");
        let path = check(&cx, &ty, &expected).expect("and the path names it");
        assert_eq!(
            convertible(&cx, &ty, &bare, &path),
            Ok(true),
            "`unit` at `{spelled}` is that namespace's `unit` and not the other's"
        );
    }
}

/// §1.5's second refusal: a bare member spelling that **nothing** narrows is an
/// error naming every candidate.
///
/// Bare and unfixed, which together are the whole scope of the ambiguity: a
/// receiver fixes the head, a written path fixes it, and a checking position
/// whose goal has a rigid head fixes it. What is left is a site that fixed
/// nothing — here an inferring one — and the repair is to write one of the names
/// the report lists.
#[test]
fn a_bare_member_nothing_narrows_is_an_error_naming_both() {
    let cx = two_namespaces();
    let Err(error) = infer(&cx, &var("unit")) else {
        panic!("two namespaces spelling `unit` must be an ambiguity rather than a choice");
    };
    let refusal = crate::programs::refusal("a member two namespaces spell", error);
    let Refusal::AmbiguousMethod { candidates, .. } = &refusal else {
        panic!("expected an ambiguous member, got `{refusal}`");
    };
    let named: Vec<&str> = candidates.iter().map(|candidate| &**candidate).collect();
    assert_eq!(
        named,
        vec!["Box", "Nat"],
        "both namespaces are named, in an order a hash map cannot change"
    );
}

/// A receiver chooses between two namespaces without asking.
///
/// The negative of the law above, and the reason disambiguation is a filter
/// rather than a search: the receiver's head *is* the answer, so a program that
/// writes one never asks the question.
#[test]
fn a_receiver_chooses_between_two_namespaces_without_asking() {
    let cx = two_namespaces();
    let ty = nat(&cx);
    let zero = var("Nat.Zero");
    let call = Raw::app(WRITTEN, Raw::method(WRITTEN, zero.clone(), "add"), zero.clone());
    let term = check(&cx, &ty, &call).expect("`Nat` heads the receiver, so `Nat.add` is the answer");
    let path = Raw::app(WRITTEN, Raw::app(WRITTEN, var("Nat.add"), zero.clone()), zero);
    let written = check(&cx, &ty, &path).expect("the path is the same program");
    assert_eq!(
        convertible(&cx, &ty, &term, &written),
        Ok(true),
        "the receiver picked `Nat.add` and not `Box.add`"
    );
}

/// Every §1.5 refusal, with the program that reaches it.
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
            name: "a member on a generic parameter",
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
            name: "a member no namespace spells for this head",
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
            name: "a bare member nothing narrows",
            cx: two_namespaces(),
            raw: var("unit"),
            expected: |refusal| matches!(refusal, Refusal::AmbiguousMethod { .. }),
        },
    ]
}
