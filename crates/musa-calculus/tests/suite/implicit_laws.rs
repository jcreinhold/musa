//! Inferred binders: where insertion fires, where it stops, and what a use site
//! may write.
//!
//! `01-surface.md` §1.5 says a type parameter is "solved by elaboration, never
//! written", and §1's grammar gives two spellings that say more than the bare
//! one: `{n : Nat}` states an inferred parameter's type, and `{A = Nat}` at a
//! call supplies one by the name of the binder it fills. This suite states what
//! those mean in the core, and — the law the prompt exists for — where the two
//! insertion rules must *not* fire.
//!
//! The corpus is Idris2's `TTImp/Elab/App.idr`, read for its stopping rule
//! rather than transcribed: musa runs insertion-at-check first, so the rule
//! lands on `abstracted` as well as on the spine walk, and both halves are
//! tested here.

use musa_calculus::{Cx, Raw, Refusal, Sort, Term, infer};

use crate::programs::{WRITTEN, annotated_unit, refusal, unit_type};

fn type0() -> Raw {
    Raw::universe(WRITTEN, Sort::ZERO)
}

fn var(name: &'static str) -> Raw {
    Raw::var(WRITTEN, name)
}

/// `{A : Type 0} → {B : Type 0} → A → A`.
///
/// Two inferred binders and only one of them mentioned by anything: `B` is the
/// phantom parameter, and it is what makes this the case rather than `id`'s.
fn phantom_type() -> Raw {
    Raw::parameter_pi(
        WRITTEN,
        "A",
        type0(),
        Raw::parameter_pi(WRITTEN, "B", type0(), Raw::pi(WRITTEN, "_", var("A"), var("A"))),
    )
}

/// `λ{A}. λ{B}. λx. x`.
fn phantom() -> Raw {
    Raw::parameter_lam(
        WRITTEN,
        "A",
        Raw::parameter_lam(WRITTEN, "B", Raw::lam(WRITTEN, "x", var("x"))),
    )
}

/// `let phantom : {A} → {B} → A → A = λ{A}. λ{B}. λx. x in body`.
fn with_phantom(body: Raw) -> Raw {
    Raw::annotated_bind(WRITTEN, "phantom", phantom_type(), phantom(), body)
}

/// `let identity : {A : Type 0} → A → A = λ{A}. λx. x in body`.
fn with_identity(body: Raw) -> Raw {
    Raw::annotated_bind(
        WRITTEN,
        "identity",
        Raw::parameter_pi(WRITTEN, "A", type0(), Raw::pi(WRITTEN, "_", var("A"), var("A"))),
        Raw::parameter_lam(WRITTEN, "A", Raw::lam(WRITTEN, "x", var("x"))),
        body,
    )
}

fn accept(name: &str, raw: &Raw) -> Term {
    infer(&Cx::new(), raw)
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .0
}

fn reject(name: &'static str, raw: &Raw) -> Refusal {
    let Err(error) = infer(&Cx::new(), raw) else {
        panic!("{name}: elaboration accepted a program it must refuse");
    };
    refusal(name, error)
}

/// **The stopping rule.** A value whose own type is an inferred Π, used where
/// that very type is expected, keeps its scheme.
///
/// Without the rule this is *refused*, not looped: `abstracted` runs first and
/// binds `A` and `B` as λs, `Switch` then walks the spine and inserts a fresh
/// unknown for each of the value's own inferred binders, and nothing in the
/// program can determine the one the body never mentions. "Could not determine
/// a type parameter", about a parameter the author never had to determine.
#[test]
fn a_value_whose_type_is_an_inferred_pi_keeps_its_scheme_where_that_type_is_expected() {
    let program = with_phantom(Raw::annot(WRITTEN, var("phantom"), phantom_type()));
    accept("a scheme at its own type", &program);
}

/// The stopping rule stops at the *end* of the walk and nowhere else: an
/// inferred binder with a written argument still to its right is one the walk
/// has to get past, whatever the call is checked against.
#[test]
fn an_inferred_binder_before_a_written_argument_is_still_filled() {
    let program = with_identity(Raw::annot(
        WRITTEN,
        Raw::call(WRITTEN, var("identity"), [annotated_unit()]),
        unit_type(),
    ));
    let term = accept("a written argument behind an inferred binder", &program);
    assert!(
        !format!("{term:?}").contains("Meta"),
        "the inferred binder was filled and solved, not left standing: {term:?}"
    );
}

/// **A name, because inference cannot always reach.** `B` is mentioned by
/// nothing, so no argument and no expected type can determine it. `{B = …}`
/// says it, and the name is the *callee's* binder name rather than a position.
#[test]
fn a_type_parameter_no_argument_determines_can_be_supplied_by_name() {
    let program = with_phantom(Raw::annot(
        WRITTEN,
        Raw::call_supplying(WRITTEN, var("phantom"), [annotated_unit()], [("B", unit_type())]),
        unit_type(),
    ));
    accept("a phantom parameter supplied by name", &program);
}

/// The same call without the name is refused, which is what makes the law
/// above about the name rather than about the program.
#[test]
fn the_same_call_without_the_name_leaves_the_parameter_undetermined() {
    let program = with_phantom(Raw::annot(
        WRITTEN,
        Raw::call(WRITTEN, var("phantom"), [annotated_unit()]),
        unit_type(),
    ));
    let refused = reject("a phantom parameter nothing determines", &program);
    assert!(
        matches!(refused, Refusal::Unsolved { .. }),
        "an undetermined parameter is reported, not defaulted: {refused}"
    );
}

/// A name no inferred binder bears is refused at the argument, naming the
/// function and the names it does bear — the two halves an author needs, since
/// the mistake is a spelling or a signature remembered from elsewhere.
#[test]
fn a_name_no_inferred_binder_bears_is_refused_naming_the_ones_that_are() {
    let program = with_phantom(Raw::call_supplying(
        WRITTEN,
        var("phantom"),
        [annotated_unit()],
        [("C", unit_type())],
    ));
    let refused = reject("a name no binder bears", &program);
    let Refusal::NoSuchParameter { name, borne, .. } = &refused else {
        panic!("a name that names nothing is its own refusal: {refused}");
    };
    assert_eq!(&**name, "C");
    assert_eq!(
        borne.iter().map(|name| &**name).collect::<Vec<_>>(),
        ["A", "B"],
        "the refusal lists every inferred binder the walk met"
    );
    assert!(
        format!("{refused}").contains("phantom"),
        "the refusal names the function: {refused}"
    );
}
