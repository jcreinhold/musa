//! Base types, literals, and the builtin registry: §5.8's conservative
//! extension, stated as laws over a worked registry.
//!
//! **The registry here is not Musa's.** `docs/rules/language/02-core-calculus.md`
//! §5.8's corollary is that a later musical domain needs no core amendment —
//! only a base type, arrow-free signatures, and a D1–D4 discharge — and a suite
//! that exercised the mechanism through `Pitch` and `Duration` would be evidence
//! for the opposite claim, because it would have taught this crate what those
//! are. So the base types below are `Int` and `Text`, which Musa does not have
//! under those names, and the δ-builtins over them are arithmetic and
//! concatenation. If the mechanism works for these it works for a pitch, and
//! that is the whole point of the boundary.
//!
//! **Which half of D1–D4 is checked here.** Registration checks the half a
//! signature makes visible: one name one meaning, no arrow in a δ signature, and
//! every base type a δ signature mentions registered inert. D2's totality, D3's
//! purity, and D4's bound are properties of the host's own functions over the
//! host's own domains; prompt 127ca's law suite samples them where the table
//! lives. What this file adds on top of registration is the *reduction*
//! behaviour the core owes: a literal is inert, two literals are convertible iff
//! the host says their payloads agree, δ fires exactly where ι does, a builtin
//! short of its arguments is neutral rather than an error, and every δ step is
//! charged.

use std::any::Any;
use std::sync::Arc;

use musa_core::{
    Base, Budget, Builtin, Cx, ElabError, Family, Level, Literal, Origin, Payload, Raw, RawArm, RawPattern, Refusal,
    Registry, Term, check, convertible, infer, normalize, well_typed,
};

use crate::programs::refusal;

/// Where every type in this suite says it was written.
const TYPES: Origin = Origin::node(700);

/// Where every term in this suite says it was written.
const TERMS: Origin = Origin::node(701);

// ---- the host's payloads ---------------------------------------------------

/// An integer, as a host would carry one.
#[derive(Debug)]
struct Int(i64);

impl Payload for Int {
    fn same(&self, other: &dyn Payload) -> bool {
        // The downcast is the host's, not the core's: §5.8 makes the payload
        // opaque precisely so that this line lives here.
        other.as_any().downcast_ref::<Self>().is_some_and(|it| it.0 == self.0)
    }

    fn shown(&self) -> String {
        self.0.to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A string, as a host would carry one.
#[derive(Debug)]
struct Text(String);

impl Payload for Text {
    fn same(&self, other: &dyn Payload) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|it| it.0 == self.0)
    }

    fn shown(&self) -> String {
        format!("{:?}", self.0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---- the worked registry ---------------------------------------------------

/// `Int : Type 0`.
fn int() -> Base {
    Base::new("Int", Term::universe(TYPES, Level::ZERO))
}

/// `Text : Type 0`.
fn text() -> Base {
    Base::new("Text", Term::universe(TYPES, Level::ZERO))
}

/// `n : Int`.
fn int_lit(value: i64) -> Literal {
    Literal::new(int().term(TYPES), Arc::new(Int(value)))
}

/// `s : Text`.
fn text_lit(value: &str) -> Literal {
    Literal::new(text().term(TYPES), Arc::new(Text(value.to_owned())))
}

/// `A → B`, over closed types, which is all a δ signature may be.
fn arrow(domain: Term, codomain: Term) -> Term {
    Term::pi(TYPES, "_", domain, codomain)
}

/// What a δ-rule reads out of an argument, or `None` when it is not one of ours.
fn as_int(literal: &Literal) -> Option<i64> {
    literal.payload().as_any().downcast_ref::<Int>().map(|it| it.0)
}

fn as_text(literal: &Literal) -> Option<&str> {
    literal
        .payload()
        .as_any()
        .downcast_ref::<Text>()
        .map(|it| it.0.as_str())
}

/// `int_add : Int → Int → Int`.
fn int_add() -> Builtin {
    Builtin::new(
        "int_add",
        arrow(int().term(TYPES), arrow(int().term(TYPES), int().term(TYPES))),
        Family::Delta,
        |arguments| match arguments {
            [left, right] => Some(int_lit(as_int(left)?.checked_add(as_int(right)?)?)),
            _ => None,
        },
    )
}

/// `text_append : Text → Text → Text`.
fn text_append() -> Builtin {
    Builtin::new(
        "text_append",
        arrow(text().term(TYPES), arrow(text().term(TYPES), text().term(TYPES))),
        Family::Delta,
        |arguments| match arguments {
            [left, right] => Some(text_lit(&format!("{}{}", as_text(left)?, as_text(right)?))),
            _ => None,
        },
    )
}

/// `int_show : Int → Text`, so that a δ-rule crossing base types is exercised.
fn int_show() -> Builtin {
    Builtin::new(
        "int_show",
        arrow(int().term(TYPES), text().term(TYPES)),
        Family::Delta,
        |arguments| match arguments {
            [only] => Some(text_lit(&as_int(only)?.to_string())),
            _ => None,
        },
    )
}

/// The registry every accepting law below is stated under.
///
/// # Panics
///
/// If the registry refuses its own worked example, which would be a defect in
/// this crate rather than a property of any test.
fn registry() -> Arc<Registry> {
    Arc::new(
        Registry::new(vec![int(), text()], vec![int_add(), text_append(), int_show()])
            .expect("the worked registry registers"),
    )
}

/// A context carrying it, at the language budget.
fn host() -> Cx {
    Cx::new().with_externs(registry())
}

/// `f a b …`, as a raw term.
fn calls(function: &str, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(Raw::var(TERMS, function), |applied, argument| {
            Raw::app(TERMS, applied, argument)
        })
}

// ---- what the registry names -----------------------------------------------

/// §5.8: a registered base type is a name a program may write, and it stands in
/// type position like any other.
#[test]
fn a_registered_base_type_is_a_type() {
    let cx = host();
    let (term, ty) = infer(&cx, &Raw::var(TERMS, "Int")).expect("`Int` resolves");
    assert_eq!(ty, Term::universe(TYPES, Level::ZERO), "`Int : Type 0`");
    assert_eq!(well_typed(&cx, &ty, &term), Ok(()), "and the core re-checks it");
}

/// A context with no registry names none of it, which is what leaves every other
/// suite in this crate unchanged.
#[test]
fn a_context_without_a_registry_names_no_base_type() {
    let error = infer(&Cx::new(), &Raw::var(TERMS, "Int")).expect_err("`Int` is nobody's name here");
    assert!(
        matches!(refusal("no registry", error), Refusal::UnknownName { .. }),
        "an unregistered base type is an unknown name, not a special case"
    );
}

/// A declaration shadows the registry rather than the other way round.
///
/// The registry is consulted last on purpose: a host that could silently
/// redefine a name in a program it never read would make what a program means
/// depend on which compiler ran it.
#[test]
fn a_binder_shadows_a_registered_name() {
    let cx = host();
    let program = Raw::annotated_lam(TERMS, "Int", Raw::record_type(TERMS, []), Raw::var(TERMS, "Int"));
    let (_, ty) = infer(&cx, &program).expect("the binder resolves");
    assert_eq!(
        ty,
        Term::pi(TYPES, "Int", Term::record_type(TERMS, []), Term::record_type(TERMS, [])),
        "the binder won, so the answer is `{{}} → {{}}` rather than anything about `Int`"
    );
}

/// A literal infers the base type it was built at.
#[test]
fn a_literal_infers_its_base_type() {
    let cx = host();
    let (term, ty) = infer(&cx, &Raw::lit(TERMS, int_lit(3))).expect("`3` infers");
    assert_eq!(ty, int().term(TYPES), "`3 : Int`");
    assert_eq!(well_typed(&cx, &ty, &term), Ok(()), "and the core re-checks it");
}

// ---- conversion ------------------------------------------------------------

/// §5.8: "two closed values of it are convertible iff they are the same
/// constant."
#[test]
fn two_literals_are_convertible_exactly_when_the_host_says_so() {
    let cx = host();
    let questions = [
        ("the same integer", int_lit(3), int_lit(3), true),
        ("two integers", int_lit(3), int_lit(4), false),
        ("the same text", text_lit("c"), text_lit("c"), true),
        ("two texts", text_lit("c"), text_lit("d"), false),
    ];
    for (name, left, right, equal) in questions {
        let ty = left.ty().clone();
        assert_eq!(
            convertible(&cx, &ty, &left.term(TERMS), &right.term(TERMS)),
            Ok(equal),
            "{name}"
        );
    }
}

/// Two literals of *different* base types are not convertible, and the question
/// is settled before either payload is asked.
///
/// It has to be: [`Payload::same`] is documented as being called only on
/// payloads of one base type, so a core that asked first would be relying on
/// every host to defend itself against a comparison the core promised not to
/// make.
#[test]
fn literals_of_different_base_types_are_not_convertible() {
    let cx = host();
    assert_eq!(
        convertible(
            &cx,
            &int().term(TYPES),
            &int_lit(3).term(TERMS),
            &text_lit("3").term(TERMS)
        ),
        Ok(false),
        "an `Int` and a `Text` are not the same value however they print"
    );
}

// ---- inertness -------------------------------------------------------------

/// D1: no reduction rule inspects a closed value of a base type.
///
/// Stated as: a literal is already its own normal form, and so is a base type.
/// There is nothing else it *could* be — the point of inertness is that no rule
/// applies — so the law is that normalization is the identity on both.
#[test]
fn a_literal_and_a_base_type_are_their_own_normal_forms() {
    let cx = host();
    let base = int().term(TYPES);
    assert_eq!(
        normalize_at(&cx, &Term::universe(TYPES, Level::ZERO), &base),
        base,
        "a base type normalizes to itself"
    );
    let literal = int_lit(3).term(TERMS);
    assert_eq!(normalize_at(&cx, &base, &literal), literal, "and so does a literal");
}

/// D1's other half: the only pattern that may stand at a base-typed column is a
/// catch-all, so a pattern that takes one apart is refused by name.
#[test]
fn a_destructuring_pattern_at_a_base_type_is_refused() {
    for RefusedProgram {
        name,
        raw,
        ty,
        expected,
    } in refused_programs()
    {
        let cx = host();
        let (ty, _) = infer(&cx, &ty).unwrap_or_else(|error| panic!("{name}: {error}"));
        let Err(error) = check(&cx, &ty, &raw) else {
            panic!("{name}: elaboration accepted a program it must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
    }
}

/// A catch-all over a base-typed subject is accepted, and needs no coverage
/// rule to be complete.
///
/// The companion to the law above, and the reason coverage says nothing here: a
/// column of catch-alls is never tested, so there is no set of cases to be
/// exhaustive over and no infinite constructor list to enumerate in a
/// diagnostic.
#[test]
fn a_catch_all_over_a_base_typed_subject_is_complete() {
    let cx = host();
    let program = Raw::match_on(
        TERMS,
        [Raw::lit(TERMS, int_lit(3))],
        vec![RawArm {
            patterns: vec![RawPattern::bind(TERMS, "n")],
            body: Raw::var(TERMS, "n"),
        }],
    );
    let term = check(&cx, &int().term(TYPES), &program).expect("a catch-all covers a base type");
    assert_eq!(
        normalize_at(&cx, &int().term(TYPES), &term),
        int_lit(3).term(TERMS),
        "and it answers the subject it bound"
    );
}

// ---- δ ---------------------------------------------------------------------

/// D2 and D3, over a finite sample: a δ-builtin applied to literals reduces to
/// exactly what the host's own function answers.
///
/// The sample is finite and the law is not, which is the honest position: §5.8
/// makes agreement a condition on *registration*, so what a suite can check is
/// that the core runs the registered rule rather than some rule of its own.
#[test]
fn a_builtin_at_literals_answers_what_the_host_function_answers() {
    let cx = host();
    let questions: Vec<(&str, Raw, Term, Term)> = vec![
        (
            "int_add 2 3",
            calls("int_add", [Raw::lit(TERMS, int_lit(2)), Raw::lit(TERMS, int_lit(3))]),
            int().term(TYPES),
            int_lit(5).term(TERMS),
        ),
        (
            "int_add 0 0",
            calls("int_add", [Raw::lit(TERMS, int_lit(0)), Raw::lit(TERMS, int_lit(0))]),
            int().term(TYPES),
            int_lit(0).term(TERMS),
        ),
        (
            "text_append",
            calls(
                "text_append",
                [Raw::lit(TERMS, text_lit("mez")), Raw::lit(TERMS, text_lit("zo"))],
            ),
            text().term(TYPES),
            text_lit("mezzo").term(TERMS),
        ),
        (
            "int_show, which crosses base types",
            calls("int_show", [Raw::lit(TERMS, int_lit(12))]),
            text().term(TYPES),
            text_lit("12").term(TERMS),
        ),
        (
            // Nested, so that a δ-rule's argument is itself a δ-redex: the
            // inner one has to have fired before the outer one can see a
            // literal, which is what "δ fires where ι fires" buys.
            "int_show (int_add 5 7)",
            calls(
                "int_show",
                [calls(
                    "int_add",
                    [Raw::lit(TERMS, int_lit(5)), Raw::lit(TERMS, int_lit(7))],
                )],
            ),
            text().term(TYPES),
            text_lit("12").term(TERMS),
        ),
    ];
    for (name, raw, ty, expected) in questions {
        let term = check(&cx, &ty, &raw).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(normalize_at(&cx, &ty, &term), expected, "{name}");
        assert_eq!(well_typed(&cx, &ty, &term), Ok(()), "{name}: and the core re-checks it");
    }
}

/// A builtin that has not been given literals is *stuck*, not wrong.
///
/// Two ways to be stuck, and both must answer a neutral rather than an error:
/// too few arguments, and an argument that is a variable. A core that refused
/// either would make a perfectly good open term — the body of any function over
/// a base type — unwritable.
#[test]
fn a_builtin_short_of_literals_is_neutral() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let partial = calls("int_add", [Raw::lit(TERMS, int_lit(2))]);
    let term = check(&cx, &arrow(int_ty.clone(), int_ty.clone()), &partial).expect("a partial application checks");
    assert_eq!(
        well_typed(&cx, &arrow(int_ty.clone(), int_ty.clone()), &term),
        Ok(()),
        "one argument short: still a function, and still re-checks"
    );

    // λn. int_add n 1 — saturated, and stuck on the binder.
    let open = Raw::annotated_lam(
        TERMS,
        "n",
        Raw::var(TERMS, "Int"),
        calls("int_add", [Raw::var(TERMS, "n"), Raw::lit(TERMS, int_lit(1))]),
    );
    let term = check(&cx, &arrow(int_ty.clone(), int_ty.clone()), &open).expect("an open body checks");
    let normal = normalize_at(&cx, &arrow(int_ty.clone(), int_ty.clone()), &term);
    assert_eq!(
        normal, term,
        "stuck on a variable, so the normal form is the term itself"
    );
    assert_eq!(well_typed(&cx, &arrow(int_ty.clone(), int_ty), &term), Ok(()));
}

/// D4: a δ step is charged before it is taken, so a budget that cannot afford
/// the reduction ends the judgment rather than performing it.
///
/// Stated against a term whose *only* work is δ: the same term is accepted at
/// the language budget and exhausts at a budget narrow enough, which is the only
/// way to see a charge that would otherwise be invisible.
#[test]
fn a_builtin_reduction_is_charged() {
    let cx = host();
    let int_ty = int().term(TYPES);
    let chain = (0..64).fold(Raw::lit(TERMS, int_lit(0)), |sum, _| {
        calls("int_add", [sum, Raw::lit(TERMS, int_lit(1))])
    });
    let term = check(&cx, &int_ty, &chain).expect("the chain checks at the language budget");
    assert_eq!(
        normalize_at(&cx, &int_ty, &term),
        int_lit(64).term(TERMS),
        "and it reduces to 64"
    );

    let narrow = Cx::with_budget(Budget::LANGUAGE.scaled(200_000)).with_externs(registry());
    let outcome = check(&narrow, &int_ty, &chain);
    assert!(
        matches!(outcome, Err(ElabError::Exhausted(_))),
        "a budget of one step cannot afford 64 δ reductions, and says so rather than answering"
    );
}

// ---- registration ----------------------------------------------------------

/// The three refusals a registration owes, each reached by a registry that earns
/// it.
#[test]
fn a_registration_is_refused_by_the_table_it_is_about() {
    for RefusedRegistry {
        name,
        outcome,
        expected,
    } in refused_registries()
    {
        let Err(refusal) = outcome else {
            panic!("{name}: the registry was admitted");
        };
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
    }
}

// ---- what the coverage gate consumes ---------------------------------------

/// A registry that must be refused, and the refusal it owes.
pub(crate) struct RefusedRegistry {
    pub(crate) name: &'static str,
    pub(crate) outcome: Result<Registry, Refusal>,
    pub(crate) expected: fn(&Refusal) -> bool,
}

/// The registrations §5.8 forbids.
///
/// Carried as outcomes rather than as programs because a registration is not a
/// term: `elaboration_laws.rs`'s gate reaches them the way it reaches a refused
/// trait declaration, and for the same reason.
pub(crate) fn refused_registries() -> Vec<RefusedRegistry> {
    let unregistered = Base::new("Ratio", Term::universe(TYPES, Level::ZERO));
    vec![
        RefusedRegistry {
            name: "one name registered twice",
            outcome: Registry::new(vec![int(), int()], Vec::new()),
            expected: |refusal| matches!(refusal, Refusal::DuplicateExtern { .. }),
        },
        RefusedRegistry {
            name: "a base type and a builtin under one name",
            outcome: Registry::new(
                vec![int()],
                vec![Builtin::new(
                    "Int",
                    arrow(int().term(TYPES), int().term(TYPES)),
                    Family::Delta,
                    |_| None,
                )],
            ),
            expected: |refusal| matches!(refusal, Refusal::DuplicateExtern { .. }),
        },
        RefusedRegistry {
            name: "a δ-builtin taking a function",
            outcome: Registry::new(
                vec![int()],
                vec![Builtin::new(
                    "int_map",
                    arrow(
                        arrow(int().term(TYPES), int().term(TYPES)),
                        arrow(int().term(TYPES), int().term(TYPES)),
                    ),
                    Family::Delta,
                    |_| None,
                )],
            ),
            expected: |refusal| matches!(refusal, Refusal::HigherOrderDelta { .. }),
        },
        RefusedRegistry {
            name: "a δ signature over a base type nobody registered",
            outcome: Registry::new(
                vec![int()],
                vec![Builtin::new(
                    "int_of_ratio",
                    arrow(unregistered.term(TYPES), int().term(TYPES)),
                    Family::Delta,
                    |_| None,
                )],
            ),
            expected: |refusal| matches!(refusal, Refusal::UnknownBase { .. }),
        },
    ]
}

/// A program that must be refused, and the refusal it owes.
pub(crate) struct RefusedProgram {
    pub(crate) name: &'static str,
    pub(crate) raw: Raw,
    /// The type it is checked against, written raw and inferred here.
    pub(crate) ty: Raw,
    pub(crate) expected: fn(&Refusal) -> bool,
}

/// The context those programs are checked in: the worked registry, and nothing
/// else.
pub(crate) fn base_context() -> Cx {
    host()
}

/// The programs D1's second half forbids.
pub(crate) fn refused_programs() -> Vec<RefusedProgram> {
    vec![
        RefusedProgram {
            name: "a constructor pattern at a base type",
            raw: Raw::match_on(
                TERMS,
                [Raw::lit(TERMS, int_lit(3))],
                vec![RawArm {
                    patterns: vec![RawPattern::constructor(TERMS, "Succ", [RawPattern::bind(TERMS, "k")])],
                    body: Raw::lit(TERMS, int_lit(0)),
                }],
            ),
            ty: Raw::var(TERMS, "Int"),
            expected: |refusal| matches!(refusal, Refusal::BaseNotMatchable { .. }),
        },
        RefusedProgram {
            name: "a record pattern at a base type",
            raw: Raw::match_on(
                TERMS,
                [Raw::lit(TERMS, text_lit("c"))],
                vec![RawArm {
                    patterns: vec![RawPattern::record(TERMS, [("head", RawPattern::bind(TERMS, "h"))])],
                    body: Raw::lit(TERMS, int_lit(0)),
                }],
            ),
            ty: Raw::var(TERMS, "Int"),
            expected: |refusal| matches!(refusal, Refusal::BaseNotMatchable { .. }),
        },
    ]
}

/// `term` at `ty`, normalized, or a panic naming what stopped it.
fn normalize_at(cx: &Cx, ty: &Term, term: &Term) -> Term {
    normalize(cx, ty, term).expect("normalization of a closed, well-typed term")
}
