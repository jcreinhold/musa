//! The kernel auditing what the elaborator built, stated as laws.
//!
//! `crates/musa-calculus/TRUST.md` makes one claim: if elaboration has a bug the
//! kernel rejects the artifact, and only a bug in the kernel can make musa
//! accept an ill-typed program. This file is what turns that sentence into
//! something that can be false.
//!
//! ```text
//! agreement:  every term the elaborator accepts, the kernel re-derives a type
//!             for, and that type is the one the elaborator claimed
//! scope:      every de Bruijn index in a checked term names a binder that
//!             encloses it (TRUST.md's third invariant)
//! control:    a term with an index one too large, and a term at the wrong
//!             universe, are both rejected
//! ```
//!
//! **The control is the load-bearing one.** A re-checker nobody has watched
//! reject anything is a function that returns `Ok`, and every other law here
//! would pass over such a function unchanged.
//!
//! The corpus is not this file's: `fixtures::corpus` and `programs::accepted`
//! are what the conversion and elaboration suites are stated over, and reusing
//! them is deliberate. A re-checker that agreed only with the terms its own file
//! built would be a re-checker about those terms.

use musa_calculus::{Binder, Checked, CoreError, Cx, Index, Malformed, Shape, Sort, Term};

use crate::fixtures;
use crate::programs;

/// Re-check `term` at `ty`, requiring the kernel to agree.
///
/// # Panics
///
/// Naming the fixture, when it does not.
fn agrees(name: &str, cx: &Cx, ty: &Term, term: &Term) {
    let checked =
        Checked::try_from(term.clone()).unwrap_or_else(|fault| panic!("{name}: not a finished term: {fault}"));
    match musa_calculus::recheck(cx, ty, &checked) {
        Ok(()) => {}
        // Not a verdict: the audit runs on its own meter, and a corpus term
        // large enough to exhaust it has told us nothing either way.
        Err(CoreError::Exhausted(_)) => {}
        Err(fault) => panic!("{name}: the kernel rejects a term the elaborator accepted: {fault}"),
    }
}

#[test]
fn the_kernel_agrees_with_every_term_the_conversion_corpus_is_stated_over() {
    for fixtures::Sample {
        name,
        cx,
        ty,
        left,
        right,
        ..
    } in fixtures::corpus()
    {
        for term in [&left, &right] {
            agrees(name, &cx, &ty, term);
        }
    }
}

/// Elaborate a program the corpus says is accepted, and the type it landed at.
///
/// # Panics
///
/// When elaboration refuses it, which `elaboration_laws.rs` would have caught
/// first.
fn elaborated(cx: &Cx, name: &str, raw: &musa_calculus::Raw, ty: Option<&Term>) -> (Term, Term) {
    match ty {
        Some(expected) => (
            musa_calculus::check(cx, expected, raw)
                .unwrap_or_else(|fault| panic!("{name}: elaboration refused it: {fault}")),
            expected.clone(),
        ),
        None => musa_calculus::infer(cx, raw).unwrap_or_else(|fault| panic!("{name}: elaboration refused it: {fault}")),
    }
}

#[test]
fn the_kernel_agrees_with_every_program_elaboration_accepts() {
    let cx = Cx::new();
    for programs::Program { name, raw, ty } in programs::accepted() {
        let (term, found) = elaborated(&cx, name, &raw, ty.as_ref());
        agrees(name, &cx, &found, &term);
    }
}

#[test]
fn every_index_in_a_checked_term_names_a_binder_that_encloses_it() {
    // `TRUST.md`'s third invariant, stated where it can be checked without
    // knowing a context's depth: the accepted programs are closed (`programs`
    // says why they have to be), so *every* index in what they elaborate to
    // must land on a binder the term itself introduced.
    let cx = Cx::new();
    for programs::Program { name, raw, ty } in programs::accepted() {
        let (term, found) = elaborated(&cx, name, &raw, ty.as_ref());
        for (what, checked) in [("the term", &term), ("its type", &found)] {
            let escaping = furthest(checked, 0);
            assert_eq!(
                escaping, 0,
                "{name}: an index in {what} reaches {escaping} binders past the closed term holding it"
            );
        }
    }
}

/// How far past its own binders the deepest index in `term` reaches.
fn furthest(term: &Term, under: u32) -> u32 {
    match term.shape() {
        Shape::Var(Index(index)) => index.saturating_sub(under),
        Shape::Meta(_) | Shape::Named { .. } | Shape::Lit(_) | Shape::Universe(_) => 0,
        Shape::Bind { binder, body, .. } => {
            let inside = furthest(body, under.saturating_add(1));
            let beside = match binder {
                Binder::Lam => 0,
                Binder::Pi { ty, .. } => furthest(ty, under),
                Binder::Let { ty, value } => furthest(ty, under).max(furthest(value, under)),
            };
            inside.max(beside)
        }
        Shape::App { function, argument } => furthest(function, under).max(furthest(argument, under)),
        // A record type's later fields stand under its earlier ones; a literal's
        // do not. Counting the type's the same way as the literal's is the
        // conservative direction — it can only make the reach look longer.
        Shape::RecordType(fields) | Shape::Record(fields) => fields
            .iter()
            .map(|field| furthest(&field.term, under))
            .max()
            .unwrap_or(0),
        Shape::Project { record, .. } => furthest(record, under),
    }
}

#[test]
fn the_kernel_rejects_an_index_that_names_no_binder() {
    let cx = Cx::new();
    let unit = Term::record_type(programs::WRITTEN, []);
    // `λx. y`, where `y` is three binders further out than anything exists.
    let escaped = Term::lam(programs::WRITTEN, "x", Term::var(programs::WRITTEN, Index(3)));
    let ty = Term::pi(programs::WRITTEN, "x", unit.clone(), unit);

    let checked = Checked::try_from(escaped).expect("the control term holds no metavariable");
    let fault = musa_calculus::recheck(&cx, &ty, &checked).expect_err("an escaped index is not a checked term");
    assert!(
        matches!(fault, CoreError::Malformed(Malformed::UnboundVariable(Index(3)))),
        "the control should be rejected for its index, and was rejected as {fault}"
    );
}

#[test]
fn the_kernel_rejects_a_term_at_the_wrong_type() {
    let cx = Cx::new();
    // `Type 0 : Type 0` — off by exactly one universe, which is the smallest
    // wrong answer §1.1 admits.
    let wrong = Term::universe(programs::WRITTEN, Sort::ZERO);
    let ty = Term::universe(programs::WRITTEN, Sort::ZERO);

    let checked = Checked::try_from(wrong).expect("the control term holds no metavariable");
    let fault = musa_calculus::recheck(&cx, &ty, &checked).expect_err("`Type 0` does not inhabit `Type 0`");
    assert!(
        matches!(fault, CoreError::Malformed(Malformed::Mistyped { .. })),
        "the control should be rejected as mistyped, and was rejected as {fault}"
    );
}
