//! The index stratum: what `decide` settles, what it refuses, and what a
//! refinement is erased to.
//!
//! `docs/rules/language/02-core-calculus.md` §1.5 admits index arguments on a
//! type — `Row(12)`, `Bar(p + q)` — drawn from a decidable arithmetic domain,
//! and states four things about them. Each is a section below: two indexed
//! types agree exactly when the *solver* says their indices do; an index outside
//! the grammar is a refusal that names the expression rather than a
//! postponement or a syntactic fallback; the index is erased at quotation, so
//! nothing stored can hold one; and the solver is metered like everything else.
//!
//! **The registry here is not Musa's**, for `base_laws.rs`'s reason and with
//! more force: §1.5's motivating programs are `Pc(n)` and `Row(n)`, and a suite
//! that exercised the mechanism through them would be evidence that the core
//! knows what a pitch class is. `Count` is an index domain and `Row` is a type
//! that carries one, and neither is a musical word.
//!
//! `Count` is a *base* type rather than a counting family because the two
//! readings of a literal index are the interesting pair: a numeral holds its
//! count in the core, and a base literal is opaque under D1 and needs the host's
//! own [`musa_calculus::Measures`]. This suite exercises the second, which is
//! the one that could be wrong.

use std::any::Any;
use std::sync::Arc;

use musa_calculus::{
    Base, Budget, Builtin, CoreError, Cx, Family, Index, Level, Literal, Operator, Origin, Payload, Raw, Refusal,
    Registry, Term, check, convertible_types, infer, normalize_type,
};

use crate::programs::refusal;

/// Where every type in this suite says it was written.
const TYPES: Origin = Origin::node(900);

/// Where every term in this suite says it was written.
const TERMS: Origin = Origin::node(901);

// ---- the host's index domain -----------------------------------------------

/// A whole number, as a host would carry one.
#[derive(Debug)]
struct Count(i128);

impl Payload for Count {
    fn same(&self, other: &dyn Payload) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|it| it.0 == self.0)
    }

    fn shown(&self) -> String {
        self.0.to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `Count : Type 0`, and the host's rule for reading one of its literals as an
/// index.
///
/// The whole of what registering an index domain takes: a base type and a
/// `fn` pointer. Nothing in `src/` learns what a `Count` is.
fn count() -> Base {
    Base::new("Count", Term::universe(TYPES, Level::ZERO)).measuring(|literal| {
        literal
            .payload()
            .as_any()
            .downcast_ref::<Count>()
            .map(|value| (value.0, 1))
    })
}

/// `Row : Type 0` — a type that carries an index and knows nothing about it.
fn row() -> Base {
    Base::new("Row", Term::universe(TYPES, Level::ZERO))
}

/// `Opaque : Type 0` — a base type that registered *no* measure, so a literal
/// of it is not an index.
fn opaque() -> Base {
    Base::new("Opaque", Term::universe(TYPES, Level::ZERO))
}

fn count_lit(value: i128) -> Literal {
    Literal::new(count().term(TYPES), Arc::new(Count(value)))
}

fn count_type() -> Term {
    count().term(TYPES)
}

fn as_count(literal: &Literal) -> Option<i128> {
    literal.payload().as_any().downcast_ref::<Count>().map(|it| it.0)
}

fn arrow(domain: Term, codomain: Term) -> Term {
    Term::pi(TYPES, "_", domain, codomain)
}

/// `count_add : Count → Count → Count`, tagged as §1.5's `+`.
fn count_add() -> Builtin {
    Builtin::new(
        "count_add",
        arrow(count_type(), arrow(count_type(), count_type())),
        Family::Delta,
        |arguments| match arguments {
            [musa_calculus::Datum::Lit(left), musa_calculus::Datum::Lit(right)] => {
                Some(musa_calculus::Datum::Lit(count_lit(as_count(left)?.checked_add(as_count(right)?)?)).into())
            }
            _ => None,
        },
    )
    .indexing(Operator::Add)
}

/// `count_mul : Count → Count → Count`, tagged as §1.5's `k *`.
fn count_mul() -> Builtin {
    Builtin::new(
        "count_mul",
        arrow(count_type(), arrow(count_type(), count_type())),
        Family::Delta,
        |arguments| match arguments {
            [musa_calculus::Datum::Lit(left), musa_calculus::Datum::Lit(right)] => {
                Some(musa_calculus::Datum::Lit(count_lit(as_count(left)?.checked_mul(as_count(right)?)?)).into())
            }
            _ => None,
        },
    )
    .indexing(Operator::Multiply)
}

/// `mystery : Count → Count`, tagged as nothing.
///
/// An arbitrary function over the domain, which is §1.5's "an arbitrary
/// function call" — in the grammar's terms, the thing an index may not be.
fn mystery() -> Builtin {
    Builtin::new(
        "mystery",
        arrow(count_type(), count_type()),
        Family::Delta,
        |arguments| match arguments {
            [musa_calculus::Datum::Lit(value)] => {
                Some(musa_calculus::Datum::Lit(count_lit(as_count(value)?.saturating_add(1))).into())
            }
            _ => None,
        },
    )
}

/// `row_of : (n : Count) → Row(n)` — §2.2's checked constructor, now with a type
/// that says which index it produced.
fn row_of() -> Builtin {
    Builtin::constructor(
        "row_of",
        Term::pi(
            TYPES,
            "n",
            count_type(),
            Term::refine(TYPES, row().term(TYPES), Term::var(TYPES, Index(0))),
        ),
        Family::Machine,
    )
}

/// `echo : {n : Count} → Row(n) → Row(n)`.
///
/// The index is a *parameter*, so a use of `echo` determines `n` from the
/// argument's type by §2.1's first-order matching — which is §1.5's claim that
/// an index variable needs no binder form of its own.
fn echo() -> Builtin {
    Builtin::constructor(
        "echo",
        Term::parameter_pi(
            TYPES,
            "n",
            count_type(),
            Term::pi(
                TYPES,
                "r",
                Term::refine(TYPES, row().term(TYPES), Term::var(TYPES, Index(0))),
                Term::refine(TYPES, row().term(TYPES), Term::var(TYPES, Index(1))),
            ),
        ),
        Family::Machine,
    )
}

/// The worked registry.
///
/// # Panics
///
/// If it refuses its own signatures, which would be a defect in this crate.
fn registry() -> Arc<Registry> {
    Arc::new(
        Registry::new(
            vec![count(), row(), opaque()],
            vec![count_add(), count_mul(), mystery(), row_of(), echo()],
        )
        .expect("the index registry registers"),
    )
}

fn cx() -> Cx {
    Cx::new().with_externs(registry())
}

// ---- writing the types -----------------------------------------------------

fn var(name: &'static str) -> Raw {
    Raw::var(TYPES, name)
}

/// `Row(index)`, written the way a program would.
fn refined(index: Raw) -> Raw {
    Raw::refine(TYPES, var("Row"), index)
}

fn literal_index(value: i128) -> Raw {
    Raw::lit(TYPES, count_lit(value))
}

/// `(n : Count) → body`, so that an *open* index has a binder to be open in.
fn over_one(body: Raw) -> Raw {
    Raw::pi(TYPES, "n", var("Count"), body)
}

/// `(n : Count) → (m : Count) → body`.
fn over_two(body: Raw) -> Raw {
    Raw::pi(TYPES, "n", var("Count"), Raw::pi(TYPES, "m", var("Count"), body))
}

fn applied(head: &'static str, arguments: [Raw; 2]) -> Raw {
    arguments
        .into_iter()
        .fold(var(head), |built, argument| Raw::app(TYPES, built, argument))
}

/// The core type a written type elaborates to.
///
/// # Panics
///
/// If elaboration refuses it, which every caller here treats as a defect in the
/// fixture rather than as an outcome.
fn elaborated(name: &str, raw: &Raw) -> Term {
    infer(&cx(), raw).unwrap_or_else(|error| panic!("{name}: {error:?}")).0
}

/// Whether two written types are the same type.
///
/// # Panics
///
/// On exhaustion, which no law here is about.
fn same_type(name: &str, left: &Raw, right: &Raw) -> bool {
    convertible_types(&cx(), &elaborated(name, left), &elaborated(name, right))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

// ---- the solver decides ----------------------------------------------------

/// §1.5: `Γ ⊢ T(a) ≡ T(b)` exactly when the solver decides `a = b`.
#[test]
fn a_refinement_agrees_with_itself() {
    assert!(same_type(
        "one index",
        &refined(literal_index(12)),
        &refined(literal_index(12))
    ));
}

/// The refusal §1.5's diagnostic sentence is about.
#[test]
fn two_indices_that_differ_are_two_types() {
    assert!(!same_type(
        "twelve against twenty-four",
        &refined(literal_index(12)),
        &refined(literal_index(24))
    ));
}

/// A refinement adds a type rather than narrowing one in place.
///
/// `Row(12)` and `Row` are two types, and the wrapper is what says so. Erasure
/// runs at quotation and not before, which is exactly why this comparison can
/// still tell them apart.
#[test]
fn a_refinement_is_not_the_type_it_refines() {
    assert!(!same_type(
        "refined against bare",
        &refined(literal_index(12)),
        &var("Row")
    ));
    assert!(!same_type(
        "bare against refined",
        &var("Row"),
        &refined(literal_index(12))
    ));
}

/// The linear form is the normal form, so `p + q` and `q + p` are one index.
///
/// This is the whole gain over comparing the two index *terms*: nothing β- or
/// δ-reduces here, because both sides are stuck on variables, and §3's
/// conversion would answer that two different spines disagree.
#[test]
fn addition_is_decided_by_arithmetic_and_not_by_the_spine() {
    let written = over_two(refined(applied("count_add", [var("n"), var("m")])));
    let flipped = over_two(refined(applied("count_add", [var("m"), var("n")])));
    assert!(same_type("p + q against q + p", &written, &flipped));
}

/// Like terms collect, which a syntactic comparison could not do either.
#[test]
fn an_index_is_compared_as_a_quantity_and_not_as_an_expression() {
    // `(n : Count) → Row(n + n)` and `(n : Count) → Row(2 * n)`.
    let doubled = over_one(refined(applied("count_add", [var("n"), var("n")])));
    let scaled = over_one(refined(applied("count_mul", [literal_index(2), var("n")])));
    assert!(same_type("n + n against 2 * n", &doubled, &scaled));

    let tripled = over_one(refined(applied("count_mul", [literal_index(3), var("n")])));
    assert!(!same_type("n + n against 3 * n", &doubled, &tripled));
}

// ---- the refusals ----------------------------------------------------------

/// §1.5: an index outside the grammar is refused, and **not** compared
/// syntactically as a fallback.
///
/// The stronger half of the law, and the one a lenient implementation would
/// get wrong: `Row(mystery n)` is not even the same type as itself. A fallback
/// that compared the two spines would answer `true` here and would thereby have
/// decided an index question by a rule that is not arithmetic.
#[test]
fn an_index_outside_the_grammar_is_refused_rather_than_compared_syntactically() {
    let called = over_one(refined(Raw::app(TYPES, var("mystery"), var("n"))));
    assert!(!same_type("an arbitrary call, against itself", &called, &called));
}

/// §1.5 admits multiplication **by a literal**; two open factors leave the
/// fragment.
#[test]
fn two_variables_multiplied_leave_the_fragment() {
    let quadratic = over_two(refined(applied("count_mul", [var("n"), var("m")])));
    assert!(!same_type("n * m, against itself", &quadratic, &quadratic));
}

/// A base type that registered no measure has no literals the solver can read,
/// which is what keeps `Syntax<Cat>`'s category from silently becoming an index.
#[test]
fn a_literal_of_an_unmeasured_base_type_is_not_an_index() {
    let unmeasured = Raw::refine(
        TYPES,
        var("Row"),
        Raw::lit(TYPES, Literal::new(opaque().term(TYPES), Arc::new(Count(12)))),
    );
    assert!(!same_type(
        "an unmeasured literal, against itself",
        &unmeasured,
        &unmeasured
    ));
}

/// The message names the two types as they were written.
///
/// §1.5: "a `Row(12)` where a `Row(24)` was expected" is the diagnostic, and the
/// solver's internal linear form never appears in one. Both halves are asserted,
/// because the second is the one that rots quietly.
#[test]
fn a_refusal_names_the_index_and_never_the_solver() {
    // `λx. x` checked at `Row(12) → Row(24)`: the body is a `Row(12)` where the
    // codomain asks for a `Row(24)`.
    let signature = Term::pi(
        TYPES,
        "x",
        elaborated("twelve", &refined(literal_index(12))),
        elaborated("twenty-four", &refined(literal_index(24))),
    );
    let error = check(&cx(), &signature, &Raw::lam(TERMS, "x", Raw::var(TERMS, "x")))
        .expect_err("a `Row(12)` is not a `Row(24)`");
    let Refusal::Mismatch(mismatch) = refusal("an index that disagrees", error) else {
        panic!("an index disagreement is a mismatch");
    };
    let shown = mismatch.to_string();
    assert!(shown.contains("Row(24)"), "{shown}");
    assert!(shown.contains("Row(12)"), "{shown}");
    assert!(!shown.contains("linear"), "{shown}");
}

// ---- erasure ---------------------------------------------------------------

/// §1.5: `quote` drops index arguments, so a read-back term carries none.
///
/// The claim byte-identity rests on. It is asserted against the *bare* type
/// rather than against "holds no `Refine`", because the two are the same
/// statement and the first one also says what the erasure left behind.
#[test]
fn quotation_drops_the_index() {
    let refined = elaborated("a refined type", &refined(literal_index(12)));
    let bare = elaborated("the bare type", &var("Row"));
    let read_back = normalize_type(&cx(), &refined).expect("a refined type normalizes");
    assert_eq!(read_back, bare);
}

/// The same, for an index no literal fixes: an open index is dropped too, so
/// there is no term shape in which one survives.
#[test]
fn quotation_drops_an_open_index() {
    let written = over_one(refined(applied("count_add", [var("n"), var("n")])));
    let erased = over_one(var("Row"));
    let read_back = normalize_type(&cx(), &elaborated("an open index", &written)).expect("it normalizes");
    assert_eq!(read_back, elaborated("the erased type", &erased));
}

// ---- a constructor carries its index ---------------------------------------

/// §2.2's checked constructor, with a type that says what it checked.
///
/// Asked rather than read back, and that is not a detour: the index is erased
/// at quotation, so the type [`infer`] hands out is the bare `Row`. What a
/// constructor carries is only ever visible to the *checker*, which is exactly
/// the arrangement §1.5 asks for — a stored type never holds an index, and the
/// index still decides what typechecks.
#[test]
fn a_constructor_carries_its_index_out() {
    let built = Raw::app(TERMS, var("row_of"), literal_index(12));
    check(&cx(), &elaborated("Row(12)", &refined(literal_index(12))), &built).expect("`row_of 12` is a `Row(12)`");
    let (_, read_back) = infer(&cx(), &built).expect("`row_of 12` infers");
    assert_eq!(read_back, elaborated("the bare type", &var("Row")));
}

/// And the index it carries is checked at the use.
#[test]
fn a_constructor_at_one_index_does_not_stand_at_another() {
    let built = Raw::app(TERMS, var("row_of"), literal_index(12));
    let wanted = elaborated("Row(24)", &refined(literal_index(24)));
    let error = check(&cx(), &wanted, &built).expect_err("`row_of 12` is not a `Row(24)`");
    assert!(matches!(
        refusal("a constructor at the wrong index", error),
        Refusal::Mismatch(_)
    ));
}

/// §1.5: an index variable is solved at the call the way a type parameter is.
///
/// `echo`'s `n` is a parameter, and nothing writes it: the argument's type is
/// `Row(12)`, matching against `Row(?n)` determines `?n`, and the result stands
/// at `Row(12)` and at nothing else. That is the flexible half of the conversion
/// hook, and it is §2.1's rule rather than a second one.
#[test]
fn a_parameter_of_index_sort_is_determined_by_the_written_argument() {
    let built = Raw::app(TERMS, var("echo"), Raw::app(TERMS, var("row_of"), literal_index(12)));
    check(&cx(), &elaborated("Row(12)", &refined(literal_index(12))), &built)
        .expect("`echo (row_of 12)` is a `Row(12)`");
    let error = check(&cx(), &elaborated("Row(24)", &refined(literal_index(24))), &built)
        .expect_err("and it is not a `Row(24)`");
    assert!(matches!(
        refusal("a solved index that disagrees", error),
        Refusal::Mismatch(_)
    ));
}

// ---- the meter -------------------------------------------------------------

/// §4: the solver is metered, so a pathological index refuses rather than hangs.
#[test]
fn a_pathological_index_expression_exhausts_rather_than_hanging() {
    // `(n : Count) → Row(n + (n + (n + …)))`, a thousand deep, under a budget a
    // two-hundredth of the language's.
    let mut index = var("n");
    for _ in 0..1_000 {
        index = applied("count_add", [var("n"), index]);
    }
    let deep = over_one(refined(index));
    let narrow = Cx::with_budget(Budget::LANGUAGE.scaled(200)).with_externs(registry());
    let Ok((term, _)) = infer(&narrow, &deep) else {
        // Elaborating the fixture ran out first, which is exhaustion too and so
        // is an honest outcome: nothing was answered, so nothing was answered
        // wrongly.
        return;
    };
    match convertible_types(&narrow, &term, &term) {
        Err(CoreError::Exhausted(_)) => {}
        Err(other) => panic!("the narrow budget failed for another reason: {other}"),
        Ok(answer) => assert!(answer, "if it fits the budget it is the same type as itself"),
    }
}
