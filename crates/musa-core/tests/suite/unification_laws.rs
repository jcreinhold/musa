//! Pattern unification: what it may solve, what it must refuse, and what it
//! says when it refuses.
//!
//! `docs/rules/language/02-core-calculus.md` §2.1 is deliberately narrow — a
//! metavariable applied to *distinct bound variables* is solved by inverting
//! that spine, and everything else is postponed rather than guessed. The laws
//! here are about that boundary. Solving too much is the failure mode that
//! matters: a unifier that tries the solution nearest to hand accepts programs
//! whose meaning it decided rather than read, and no later stage can tell.
//!
//! The refusal half is the diagnostic policy in `crate::refuse`'s module doc,
//! stated as tests: the *smallest* disagreeing pair, and the path from the two
//! whole types down to it.

use musa_core::{Cx, ElabError, Index, Level, Mismatch, PathStep, Raw, Refusal, Term, check, infer};

use crate::programs::{WRITTEN, annotated_unit, core_unit_type, refusal, unit, unit_type};

fn type0() -> Raw {
    Raw::universe(WRITTEN, Level::ZERO)
}

fn var(name: &'static str) -> Raw {
    Raw::var(WRITTEN, name)
}

/// Elaborate, expecting acceptance.
fn elaborate(name: &str, raw: &Raw) -> Term {
    infer(&Cx::new(), raw)
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .0
}

/// Elaborate, expecting a refusal, and answer it.
fn refuse(name: &'static str, raw: &Raw, ty: Option<&Term>) -> Refusal {
    let cx = Cx::new();
    let outcome = match ty {
        Some(ty) => check(&cx, ty, raw).map(|term| (term, ty.clone())),
        None => infer(&cx, raw),
    };
    let Err(error) = outcome else {
        panic!("{name}: elaboration accepted a program it must refuse");
    };
    refusal(name, error)
}

/// §2.1: an inserted implicit is solved to the one thing that determines it, and
/// read back into the term.
///
/// The expected term is written out rather than derived, because deriving it
/// would mean running the elaborator's own read-back to predict the elaborator's
/// read-back.
#[test]
fn an_inserted_implicit_is_solved_to_the_argument_that_determines_it() {
    // `let id : {X : Type 0} → X → X = λ{X}. λx. x in id ({} : {})`
    let program = Raw::annotated_bind(
        WRITTEN,
        "id",
        Raw::implicit_pi(WRITTEN, "X", type0(), Raw::pi(WRITTEN, "_", var("X"), var("X"))),
        Raw::implicit_lam(WRITTEN, "X", Raw::lam(WRITTEN, "x", var("x"))),
        Raw::app(WRITTEN, var("id"), annotated_unit()),
    );
    let term = elaborate("an inserted implicit", &program);

    let Some((_, _, body)) = binding(&term) else {
        panic!("expected a let, got {term:?}");
    };
    assert_eq!(
        body,
        &Term::app(
            WRITTEN,
            // The inserted implicit, solved to the unit type and read back.
            Term::app(WRITTEN, Term::var(WRITTEN, Index(0)), core_unit_type()),
            Term::record(WRITTEN, []),
        ),
        "the implicit is filled by the type the explicit argument had"
    );
}

/// §2.1: solving is *inverting a spine*, so the same metavariable reached twice
/// with incompatible arguments is a mismatch rather than a re-solve.
///
/// Solutions are write-once. A unifier that overwrote one would make acceptance
/// depend on the order constraints happened to arrive in.
#[test]
fn a_metavariable_determined_twice_must_be_determined_the_same_way() {
    // `let f : {X : Type 0} → X → X → X = λ{X}. λa. λb. a
    //  in f ({} : {}) (Type 0)`
    let program = Raw::annotated_bind(
        WRITTEN,
        "f",
        Raw::implicit_pi(
            WRITTEN,
            "X",
            type0(),
            Raw::pi(WRITTEN, "_", var("X"), Raw::pi(WRITTEN, "_", var("X"), var("X"))),
        ),
        Raw::implicit_lam(WRITTEN, "X", Raw::lam(WRITTEN, "a", Raw::lam(WRITTEN, "b", var("a")))),
        Raw::app(WRITTEN, Raw::app(WRITTEN, var("f"), annotated_unit()), type0()),
    );
    let refusal = refuse("a metavariable determined twice", &program, None);
    assert!(
        matches!(refusal, Refusal::Mismatch(_)),
        "the second argument disagrees with the solution the first fixed, got `{refusal}`"
    );
}

/// §2.1: a metavariable is never defaulted and never generalized, so one nothing
/// determines is a refusal that names its site.
#[test]
fn a_metavariable_nothing_determines_is_refused_rather_than_defaulted() {
    let refusal = refuse("an unannotated binder", &Raw::lam(WRITTEN, "x", var("x")), None);
    assert!(
        matches!(refusal, Refusal::Unsolved { .. }),
        "expected an unsolved metavariable, got `{refusal}`"
    );
}

/// The diagnostic policy: the *smallest* disagreeing pair, with the path to it.
///
/// Reached through §2's `Switch`, which is the only rule that hands two whole
/// types to conversion — a λ checked against a Π is taken apart by the checking
/// rule instead, and never asks conversion about the Π at all. Two function
/// types that agree on their argument type and differ under it must report the
/// codomain pair, not the two Π types: a conversion error that prints both
/// normal forms in full has made the reader do the diffing.
#[test]
fn a_mismatch_reports_the_smallest_pair_that_disagrees() {
    // `let f : {} → {} = λx. x in f`, checked at `{} → Type 0`.
    let mismatch = mismatch(
        "a codomain that disagrees",
        &Raw::annotated_bind(
            WRITTEN,
            "f",
            Raw::pi(WRITTEN, "x", unit_type(), unit_type()),
            Raw::annotated_lam(WRITTEN, "x", unit_type(), var("x")),
            var("f"),
        ),
        &Term::pi(WRITTEN, "x", core_unit_type(), Term::universe(WRITTEN, Level::ZERO)),
    );
    assert_eq!(mismatch.path, vec![PathStep::Codomain]);
    assert_eq!(mismatch.expected, Term::universe(WRITTEN, Level::ZERO));
    assert_eq!(mismatch.found, core_unit_type());
    assert_eq!(mismatch.to_string(), "type mismatch at the result type");
}

/// The same policy one step further in: a field of a record type.
#[test]
fn a_mismatch_inside_a_record_type_names_the_field() {
    // `let r : { a : {} } = { a = {} } in r`, checked at `{ a : Type 0 }`.
    let mismatch = mismatch(
        "a field that disagrees",
        &Raw::annotated_bind(
            WRITTEN,
            "r",
            Raw::record_type(WRITTEN, [("a", unit_type())]),
            Raw::record(WRITTEN, [("a", unit())]),
            var("r"),
        ),
        &Term::record_type(WRITTEN, [("a", Term::universe(WRITTEN, Level::ZERO))]),
    );
    assert_eq!(mismatch.path, vec![PathStep::Field("a".into())]);
    assert_eq!(mismatch.expected, Term::universe(WRITTEN, Level::ZERO));
    assert_eq!(mismatch.found, core_unit_type());
}

/// §2.1: a metavariable's solution may not mention a binder the metavariable was
/// created outside of.
///
/// The escape check, and the reason a contextual metavariable is represented
/// closed: `?α` created at the top level cannot be solved to something naming a
/// λ's argument, because the solution would be read back where that argument does
/// not exist. Refusing is the only honest answer — there is no smaller solution
/// to fall back to.
#[test]
fn a_solution_that_would_escape_its_scope_is_refused_rather_than_captured() {
    // `let f : {X : Type 0} → ((y : {}) → X) → {} = λ{X}. λg. {}
    //  in f (λy. (y : {}))`
    //
    // Determining `X` from the argument's type would need `X := {}` under `y` —
    // which is fine — but the elaborated argument's own type mentions `y`, and
    // `X` was created outside it.
    let program = Raw::annotated_bind(
        WRITTEN,
        "f",
        Raw::implicit_pi(
            WRITTEN,
            "X",
            type0(),
            Raw::pi(WRITTEN, "_", Raw::pi(WRITTEN, "y", unit_type(), var("X")), unit_type()),
        ),
        Raw::implicit_lam(WRITTEN, "X", Raw::lam(WRITTEN, "g", unit())),
        Raw::app(
            WRITTEN,
            var("f"),
            Raw::annotated_lam(WRITTEN, "y", unit_type(), Raw::annot(WRITTEN, var("y"), unit_type())),
        ),
    );
    // Whatever the unifier does here, it may not silently accept a solution that
    // names `y`: either it finds the closed one, or it says it could not.
    match infer(&Cx::new(), &program) {
        Ok((term, _)) => assert!(
            !mentions_free_variable(&term),
            "a solution naming a binder it was created outside of was read back"
        ),
        Err(ElabError::Refused(Refusal::Unsolved { .. } | Refusal::Mismatch(_))) => {}
        Err(error) => panic!("an escaping solution must be refused, not `{error}`"),
    }
}

/// The mismatch a checking question produces, or a panic naming what came out
/// instead.
fn mismatch(name: &'static str, raw: &Raw, ty: &Term) -> Mismatch {
    let refusal = refuse(name, raw, Some(ty));
    let Refusal::Mismatch(mismatch) = refusal else {
        panic!("{name}: expected a conversion mismatch, got `{refusal}`");
    };
    *mismatch
}

/// A `let`'s three parts, when the term is one.
fn binding(term: &Term) -> Option<(&Term, &Term, &Term)> {
    if let musa_core::Shape::Let { ty, value, body, .. } = term.shape() {
        Some((ty, value, body))
    } else {
        None
    }
}

/// Whether any variable in `term` names a binder outside it.
///
/// A closed term has none; this is the property a leaked solution would break.
fn mentions_free_variable(term: &Term) -> bool {
    fn walk(term: &Term, depth: u32) -> bool {
        use musa_core::Shape;

        match term.shape() {
            Shape::Var(index) => index.0 >= depth,
            // Closed by construction, so it escapes nothing.
            Shape::Const(_) => false,
            Shape::Universe(_) | Shape::Meta(_) => false,
            Shape::Pi { domain, codomain, .. } => walk(domain, depth) || walk(codomain, depth.saturating_add(1)),
            Shape::Lam { body, .. } => walk(body, depth.saturating_add(1)),
            Shape::App { function, argument } => walk(function, depth) || walk(argument, depth),
            Shape::RecordType(fields) => fields
                .iter()
                .enumerate()
                .any(|(position, field)| walk(&field.term, depth.saturating_add(u32::try_from(position).unwrap_or(0)))),
            Shape::Record(fields) => fields.iter().any(|field| walk(&field.term, depth)),
            Shape::Project { record, .. } => walk(record, depth),
            Shape::Id { ty, left, right } => walk(ty, depth) || walk(left, depth) || walk(right, depth),
            Shape::Refl(value) => walk(value, depth),
            Shape::J {
                ty,
                from,
                motive,
                base,
                to,
                proof,
            } => [ty, from, motive, base, to, proof].iter().any(|part| walk(part, depth)),
            Shape::Let { ty, value, body, .. } => {
                walk(ty, depth) || walk(value, depth) || walk(body, depth.saturating_add(1))
            }
        }
    }

    walk(term, 0)
}

/// §2.1's third site: a universe the surface wrote without a level is solved by
/// what the term is used as.
///
/// `succ ?ℓ ≡ 1` is the constraint, and `succ` is injective on the naturals, so
/// `?ℓ` is `0` and nothing was guessed to get there.
#[test]
fn a_universe_written_without_a_level_is_solved_by_the_one_it_meets() {
    let one = Term::universe(WRITTEN, Level::ZERO.succ());
    let term = check(&Cx::new(), &one, &Raw::any_universe(WRITTEN))
        .unwrap_or_else(|error| panic!("a bare universe checked at `Type 1`: {error}"));
    assert_eq!(
        term,
        Term::universe(WRITTEN, Level::ZERO),
        "the level solved to the one the checking type determined"
    );
}

/// And one nothing determines is refused, by the same rule that refuses an
/// undetermined term: §2.1 never defaults, in either sort.
///
/// `Type 0` would be the obvious guess and is exactly the guess forbidden — a
/// program whose universe the checker picked is a program whose meaning it
/// decided.
#[test]
fn a_universe_level_nothing_determines_is_refused_rather_than_defaulted() {
    let refusal = refuse("a bare universe with nothing to fix its level", &Raw::any_universe(WRITTEN), None);
    let Refusal::Unsolved { site, created, .. } = &refusal else {
        panic!("expected an unsolved level, got `{refusal}`");
    };
    assert_eq!(site.describe(), "the level of a universe");
    assert_eq!(*created, WRITTEN, "the report points at the `Type` that made it");
    assert_eq!(refusal.to_string(), "could not determine the level of a universe");
}
