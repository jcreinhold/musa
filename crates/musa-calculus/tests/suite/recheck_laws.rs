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
//! # The audit table (prompt 158)
//!
//! Prompt 149 built this pass over a seven-constructor term language and made
//! extending it an obligation of every prompt that added to the core. Prompt
//! 158 closed it, and the deliverable of that reading is this table: what the
//! kernel re-derives about each construct, and the control that proves it can
//! reject. **A row with no control is a row that has not been checked.**
//!
//! | Owed by | Construct | What the kernel re-derives | Control |
//! | --- | --- | --- | --- |
//! | 149 | scope discipline | every index names an enclosing binder | `the_kernel_rejects_an_index_that_names_no_binder` |
//! | 149 | any term at any type | the type, from the rules in `02-core-calculus.md` §2 | `the_kernel_rejects_a_term_at_the_wrong_type` |
//! | 151 | an applied type constructor | the family's own declared type, off the declaration | the wrong-type control above, which is the only way it fails |
//! | 152 | level parameters | `Def::instance` at the levels *the term names*, so an inconsistent use derives a type its surroundings reject | `sort_laws::level_faults` (`LevelArity`) |
//! | 153 | a metavariable's solution | read back at the unknown's own arity, apart from the check made at solving time | `kernel::unify`'s `a_stored_solution_that_names_a_variable_outside_its_scope_is_caught_again` |
//! | 154 | an inserted implicit | nothing special — it is an ordinary application once inserted | fixture: `programs::accepted`'s "an implicit inserted at a use site" |
//! | 155 | a branch's motive | via the emission: a method's type *is* the motive instantiated at that method's pattern | `elaboration::audit_laws`'s `the_kernel_rejects_a_branch_that_does_not_answer_its_motive` |
//! | 155 | coverage | `CaseTree::uncovered`, asked of the declaration group and asked **by the kernel** | `elaboration::audit_laws`'s `the_kernel_rejects_a_tree_that_leaves_a_constructor_unanalysed` |
//! | 155aa | a lifted definition | an ordinary member, re-checked like any other | `the_kernel_rechecks_every_member_including_the_ones_nobody_wrote` |
//! | 155a | structural descent | `terminate::descends` over the finished tree, asked **by the kernel** | `elaboration::audit_laws`'s `the_kernel_rejects_a_recursion_that_does_not_descend` |
//! | 156 | an `Impossible` branch | the same emission rule, and this is the finding: a refuted branch's motive is a *large elimination over the subject's index*, not a flag copied from the decision that refuted it, so ι gives it an identity type exactly when the branch really is unreachable | the motive control above, which is the mechanism |
//! | 157 | a generated projection | an ordinary constant at `Role::Projection` | fixture: `programs::accepted`'s "a projection" |
//! | — | anything at all | that elaboration cannot produce a term the kernel rejects | `generated_laws`, over programs nobody wrote |
//!
//! **Two rows moved during the audit rather than being built.** Coverage and
//! descent were already re-derived from the declaration — `CaseTree::uncovered`
//! reads the group and `terminate::descends` reads the finished tree — but the
//! only thing *asking* them was the elaborator, which is the pass under audit.
//! Prompt 158 moved the asking to the kernel and left the answering where it
//! was. That is the whole difference between a re-derivation and bookkeeping.
//!
//! **And one row needed nothing.** 156's `Impossible` looked like the hardest
//! obligation here and turned out to be discharged already, for a reason worth
//! writing down: `elaboration::case`'s `filtered` builds a refuted branch's
//! motive by eliminating over the index rather than by recording that it
//! refuted. So the emitted `λx. x` is *checked* against a type computed
//! independently of the decision it is evidence for, and a branch wrongly
//! called impossible is `Malformed::Mistyped`. Re-deriving the unification by
//! hand would have re-implemented `refuted` in the kernel and audited it with a
//! copy of itself.
//!
//! **The last row has since fired, and against the kernel.** `generated_laws`
//! found `(let u : Unit = unit in fn (x : Unit) { x }) unit` — a λ applied
//! through the `let` written in front of it. `recheck`'s `peeled` reads a
//! β-redex as the `let` the evaluator reduces it to, which is the only way a
//! core λ gets a domain at all, and it stopped that walk at the first binder
//! that was not a λ. So the audit inferred a λ and refused a term elaboration
//! was right to build. `programs::accepted` now carries that shape and one with
//! the two kinds of binder alternating, so the row has fixtures under it rather
//! than only a generator; `TRUST.md` records why the defect was the audit's and
//! not the elaborator's.
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
    let cx = programs::cx();
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
    let cx = programs::cx();
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
    }
}

#[test]
fn the_kernel_rejects_an_index_that_names_no_binder() {
    let cx = programs::cx();
    let unit = programs::core_unit_type();
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

/// The closed pass, over a program that holds a member nobody wrote.
///
/// [`musa_calculus::recheck_program`] is what turns the claim into a gate:
/// every definition read at the type its declaration claimed, plus the two
/// questions a tree's emission cannot be wrong about. The fixture is the
/// lifting suite's, because prompt 155aa's obligation is exactly this — the
/// program holds `bump`'s lifted `walk`, which no source line wrote, and a
/// re-checker is only known to walk what it has walked.
#[test]
fn the_kernel_rechecks_every_member_including_the_ones_nobody_wrote() {
    let (cx, program) = crate::lifting_laws::declared();
    let lifted = program.lifted();
    assert!(
        !lifted.is_empty(),
        "the fixture is about a member nobody wrote, and this program holds none"
    );
    musa_calculus::recheck_program(&cx, &program).expect("the kernel agrees with every member");
}

/// The same pass over every corpus term that can stand as a definition.
///
/// One definition each, so this adds no member shape the law above does not
/// have — what it adds is *breadth*, which is the property the corpus was built
/// for and the reason this file reuses it rather than writing its own. The type
/// is left to be inferred: a corpus entry carries a core `Term` and a top level
/// states a `Raw` one, and the elaborator already answers "what type does this
/// have" without this file learning to read a `Term` backwards.
#[test]
fn the_kernel_rechecks_a_program_built_from_every_inferable_corpus_term() {
    let cx = programs::cx();
    let mut declared = 0_u32;
    for programs::Program { name, raw, .. } in programs::accepted() {
        let program = musa_calculus::RawProgram {
            families: Vec::new(),
            definitions: vec![musa_calculus::RawTopLevel {
                origin: programs::WRITTEN,
                name: "member".into(),
                visibility: musa_calculus::Visibility::Public,
                module: None,
                ty: None,
                value: raw,
            }],
        };
        // Not every corpus term is a *definition*: several are checking forms
        // that state no type of their own, and §2 gives a core λ no type to
        // infer. `elaboration_laws.rs` is where that is a law; here it simply
        // is not this test's subject.
        let Ok(elaborated) = musa_calculus::declare_program(&cx, &program) else {
            continue;
        };
        declared = declared.saturating_add(1);
        musa_calculus::recheck_program(&cx, &elaborated)
            .unwrap_or_else(|fault| panic!("{name}: the kernel rejects a program elaboration accepted: {fault}"));
    }
    assert!(
        declared >= 4,
        "only {declared} corpus terms stood as definitions, so this law is close to vacuous"
    );
}
