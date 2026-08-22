//! The unifier's three outcomes, and the constraint queue that makes the third
//! one real.
//!
//! `unification_laws.rs` states what §2.1's *fragment* may and may not solve.
//! This file states what happens to everything else. Before prompt 153 the
//! answer was "decide now or report a mismatch", and the elaborator carried a
//! two-pass walk over each application spine to keep the hardest case away from
//! that choice. §2.1 replaces both with a queue: a comparison nothing can
//! decide yet is *postponed*, retried when a solution arrives, and turned into
//! an error naming what could not be determined only at the declaration's end.
//!
//! The corpus is Idris2's `Core/Unify.idr` and its tests, read for what they
//! exercise rather than transcribed: a parameter determined by a *later*
//! argument, a λ read against a type nothing has determined, two unknowns
//! standing for one another until something settles both, and an unknown
//! nothing ever determines. Musa has no `Delay` and no quantities, so
//! `retryGuess`'s laziness cases have no counterpart here and none is invented.

use musa_calculus::{Budget, Cx, ElabError, Raw, Refusal, Shape, Sort, Term, check, infer};

use crate::programs::{WRITTEN, annotated_unit, core_unit_type, refusal, unit_type};

fn type0() -> Raw {
    Raw::universe(WRITTEN, Sort::ZERO)
}

fn var(name: &'static str) -> Raw {
    Raw::var(WRITTEN, name)
}

/// `f a₀ a₁ …`, written the way a program would.
fn applied(head: Raw, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(head, |built, argument| Raw::app(WRITTEN, built, argument))
}

/// `let twice : {X : Type 0} → (X → X) → X → X = λ{X}. λg. λx. g x in body`.
///
/// The shape the deleted two-pass walk existed for: the parameter `X` is
/// mentioned by the *first* argument, which is a λ and so describes nothing,
/// and it is determined by the *second*.
fn with_twice(body: Raw) -> Raw {
    let endo = Raw::pi(WRITTEN, "_", var("X"), var("X"));
    Raw::annotated_bind(
        WRITTEN,
        "twice",
        Raw::parameter_pi(
            WRITTEN,
            "X",
            type0(),
            Raw::pi(WRITTEN, "g", endo, Raw::pi(WRITTEN, "_", var("X"), var("X"))),
        ),
        Raw::parameter_lam(
            WRITTEN,
            "X",
            Raw::lam(
                WRITTEN,
                "g",
                Raw::lam(WRITTEN, "x", Raw::app(WRITTEN, var("g"), var("x"))),
            ),
        ),
        body,
    )
}

/// `let use : {F : Type 0} → F → F = λ{F}. λf. f in body`.
///
/// The parameter *is* the domain, so an argument written as a bare λ meets a
/// type nothing has determined at all — the one case no later argument can
/// rescue, because the λ is the only argument there is.
fn with_use(body: Raw) -> Raw {
    Raw::annotated_bind(
        WRITTEN,
        "use",
        Raw::parameter_pi(WRITTEN, "F", type0(), Raw::pi(WRITTEN, "_", var("F"), var("F"))),
        Raw::parameter_lam(WRITTEN, "F", Raw::lam(WRITTEN, "f", var("f"))),
        body,
    )
}

/// `use (λx. x)`, the η-expansion case, with `body` wrapped around it.
fn used_identity() -> Raw {
    applied(var("use"), [Raw::lam(WRITTEN, "x", var("x"))])
}

/// Whether a term holds no metavariable at all.
///
/// Stronger than "every metavariable is solved", and deliberately: `zonk` runs
/// before a term is stored, so a stored term that still *holds* one is a
/// solution nobody wrote back — which reads correctly today and stops meaning
/// anything the moment the term is evaluated somewhere else.
///
/// Written by walking the shape rather than over a `Debug` string, so that a
/// new [`Shape`] variant holding a term is a compile error here.
fn holds_no_unknown(term: &Term) -> bool {
    match term.shape() {
        Shape::Meta(_) => false,
        Shape::Var(_) | Shape::Universe(_) | Shape::Named { .. } | Shape::Lit(_) => true,
        Shape::Bind { binder, body, .. } => binder.outer().all(holds_no_unknown) && holds_no_unknown(body),
        Shape::App { function, argument } => holds_no_unknown(function) && holds_no_unknown(argument),
        Shape::RecordType(fields) | Shape::Record(fields) => fields.iter().all(|field| holds_no_unknown(&field.term)),
        Shape::Project { record, .. } => holds_no_unknown(record),
    }
}

/// Elaborate, expecting acceptance, and answer the term.
fn accept(name: &str, raw: &Raw) -> Term {
    let term = infer(&Cx::new(), raw)
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .0;
    assert!(
        holds_no_unknown(&term),
        "{name}: the stored term still holds an unknown"
    );
    term
}

/// Elaborate, expecting a refusal, and answer it.
fn reject(name: &'static str, raw: &Raw) -> Refusal {
    let Err(error) = infer(&Cx::new(), raw) else {
        panic!("{name}: elaboration accepted a program it must refuse");
    };
    refusal(name, error)
}

/// **Solved.** The first outcome, and the one the deleted deferral was built to
/// reach the long way round: the argument that determines a parameter is
/// written *after* the argument that could not be read without it.
#[test]
fn a_parameter_a_later_argument_determines_is_solved_by_it() {
    let program = with_twice(applied(
        var("twice"),
        [Raw::lam(WRITTEN, "y", var("y")), annotated_unit()],
    ));
    accept("a parameter determined by a later argument", &program);
}

/// **Solved, by η-expanding the unknown itself.** A λ read against a type
/// nothing has determined does not wait: whatever else is unknown about a type
/// that has to accept a λ, it is a function type. Committing to that much — and
/// to nothing else, the domain and the codomain staying unknowns — is what lets
/// the argument be read in the order it was written.
#[test]
fn a_lambda_read_against_an_unknown_makes_it_a_function_type() {
    let program = with_use(Raw::annot(
        WRITTEN,
        used_identity(),
        Raw::pi(WRITTEN, "_", unit_type(), unit_type()),
    ));
    accept("a lambda at a wholly undetermined domain", &program);
}

/// **Not unifiable.** The second outcome: two rigid heads that disagree
/// disagree now and at every later moment, so the queue is the wrong place for
/// them and the author hears about it where it was written.
#[test]
fn two_rigid_heads_that_disagree_are_refused_rather_than_postponed() {
    let program = with_twice(Raw::annot(
        WRITTEN,
        applied(var("twice"), [Raw::lam(WRITTEN, "y", var("y")), annotated_unit()]),
        type0(),
    ));
    let refused = reject("a call whose result is not the type written around it", &program);
    assert!(
        matches!(refused, Refusal::Mismatch(_)),
        "a rigid disagreement is a mismatch, not a wait: {refused}"
    );
}

/// **Blocked, and never settled.** The third outcome is a real answer, and the
/// declaration's end is where it stops being one: §2.1 defaults nothing and
/// leaves nothing for a later declaration, so an unknown the program never
/// determined is reported as exactly that.
#[test]
fn an_unknown_nothing_determines_is_reported_at_the_declarations_end() {
    let program = with_use(used_identity());
    let refused = reject("a lambda whose type nothing determines", &program);
    assert!(
        matches!(refused, Refusal::Unsolved { .. }),
        "an undetermined unknown is reported, not defaulted: {refused}"
    );
}

/// **Blocked, and then settled.** A comparison postponed while both sides were
/// unknown is retried when a solution arrives, which is why the queue is
/// drained to a fixpoint rather than read once.
#[test]
fn a_comparison_blocked_on_an_unknown_is_retried_when_one_arrives() {
    // `twice (use (λy. y)) ({} : {})`. The inner call's parameter and the outer
    // call's parameter stand for one another when they meet, and only the last
    // argument determines either.
    let program = with_twice(with_use(applied(var("twice"), [used_identity(), annotated_unit()])));
    accept("two unknowns settled by one later argument", &program);
}

/// **The budget law §4 already answered.** A retry is the same comparison run
/// again and charges the same meter, so *n* retries cost *n* times and a queue
/// whose retries are expensive exhausts rather than hanging. Nothing was added
/// to `budget.rs` for this prompt, and this is why.
#[test]
fn a_queue_that_keeps_retrying_exhausts_the_budget_rather_than_hanging() {
    let program = with_twice(with_use(applied(var("twice"), [used_identity(), annotated_unit()])));
    let narrow = Cx::with_budget(Budget::LANGUAGE.scaled(1_000_000));
    let outcome = infer(&narrow, &program);
    assert!(
        matches!(outcome, Err(ElabError::Exhausted(_))),
        "a budget this narrow must end the judgment rather than answer it"
    );
    // The same question at the language budget is answered, so the law above is
    // about the budget and not about the program.
    assert!(
        infer(&Cx::new(), &program).is_ok(),
        "the same program is accepted at the language budget"
    );
}

/// The drain belongs to the declaration and not to one judgment: a checking
/// question reaches the same queue, and the expected type is one more thing
/// that can settle it.
#[test]
fn the_queue_is_drained_for_a_checking_question_too() {
    let program = with_use(used_identity());
    let outcome = check(
        &Cx::new(),
        &Term::pi(WRITTEN, "_", core_unit_type(), core_unit_type()),
        &program,
    );
    let term = outcome.unwrap_or_else(|error| panic!("the expected type determines what the program did not: {error}"));
    assert!(holds_no_unknown(&term), "the stored term still holds an unknown");
}
