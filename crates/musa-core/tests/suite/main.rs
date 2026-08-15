//! Every `musa-core` integration test, linked as one binary.
//!
//! One test target per file means one more link of the whole workspace on every
//! build, and one more set of object files that cargo never reclaims from
//! `target/debug/deps`. See `docs/notes/toolchain/slow-test-suite.md`.
//!
//! The three suites state `docs/rules/language/02-core-calculus.md`'s §3 and §4
//! obligations as tests. They share one corpus, deliberately: a law that held
//! only for the terms its own file happened to build would be a law about those
//! terms. [`fixtures::corpus`] is that corpus, and every sample in it names the
//! rule it exists to exercise.

// A fixture that cannot be built, or a law that fails, is a defect in this
// crate rather than a program error: panicking is the correct behaviour there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

mod budget_laws;
mod conversion_laws;
mod normalization_laws;

/// The terms every law suite is stated over.
pub(crate) mod fixtures {
    use musa_core::{Budget, CoreError, Cx, Index, Level, Term};

    /// One question the laws are asked at: a context, the type the question is
    /// asked at, two terms, and whether §3 calls them equal.
    ///
    /// The type is not decoration. §3 performs η at Π and at records during
    /// quotation, so "are these two terms equal" is not a question that can be
    /// asked without saying equal *at what*.
    pub(crate) struct Sample {
        /// The rule this sample exists to exercise.
        pub(crate) name: &'static str,
        /// The binders both terms are read under.
        pub(crate) cx: Cx,
        /// The type both terms inhabit.
        pub(crate) ty: Term,
        pub(crate) left: Term,
        pub(crate) right: Term,
        /// What §3 says the answer is.
        pub(crate) equal: bool,
    }

    fn var(index: u32) -> Term {
        Term::Var(Index(index))
    }

    fn type0() -> Term {
        Term::Universe(Level::ZERO)
    }

    /// The corpus at the language budget.
    ///
    /// # Panics
    ///
    /// If the language budget cannot build it, which would be a defect in this
    /// crate rather than a property of any test.
    pub(crate) fn corpus() -> Vec<Sample> {
        corpus_at(Budget::LANGUAGE).expect("the language budget builds the corpus")
    }

    /// The corpus at a stated budget.
    ///
    /// Parameterized because §4's independence law cannot be stated otherwise:
    /// a context evaluates the types it is extended by, so asking the same
    /// questions under a narrower budget means building the same contexts under
    /// it. A narrow enough budget exhausts here rather than at the conversion,
    /// and that is an honest outcome — see `budget_laws.rs`.
    ///
    /// # Errors
    ///
    /// [`CoreError::Exhausted`] when `budget` cannot evaluate one of the types a
    /// context is extended by.
    pub(crate) fn corpus_at(budget: Budget) -> Result<Vec<Sample>, CoreError> {
        let empty = Cx::with_budget(budget);
        // A : Type 0
        let a = empty.assume(&type0())?;
        // A : Type 0, x : A
        let a_x = a.assume(&var(0))?;
        // A : Type 0, x : A, y : A
        let a_xy = a_x.assume(&var(1))?;
        // A : Type 0, x : A, d := x : A
        let a_x_d = a_x.define(&var(1), &var(0))?;

        // A : Type 0, g : A → A
        let arrow = Term::pi("z", var(0), var(1));
        let g = a.assume(&arrow)?;
        // A : Type 0, g : A → A, f : (A → A) → A
        let f_of_arrow = g.assume(&Term::pi("h", Term::pi("z", var(1), var(2)), var(2)))?;
        // A : Type 0, g : A → A, f : A → A
        let two_functions = g.assume(&Term::pi("z", var(1), var(2)))?;

        // A : Type 0, r : { fst : A, snd : A }
        let pair = a.assume(&Term::record_type([("fst", var(0)), ("snd", var(1))]))?;
        // r : { ty : Type 0, val : ty }
        let dependent_pair_type = Term::record_type([("ty", type0()), ("val", var(0))]);
        let dependent_pair = empty.assume(&dependent_pair_type)?;

        // A : Type 0, x : A, y : A, p : Id A x y
        let identified = a_xy.assume(&Term::identity(var(2), var(1), var(0)))?;
        // λy. λe. A, a constant motive: `J`'s result type is then `A` at every
        // endpoint, which is what lets ι be stated without a second family.
        let constant_motive = |depth: u32| Term::lam("y", Term::lam("e", var(depth)));

        Ok(vec![
            Sample {
                name: "β",
                cx: a_x.clone(),
                ty: var(1),
                left: Term::app(Term::lam("z", var(0)), var(0)),
                right: var(0),
                equal: true,
            },
            Sample {
                name: "δ at a let",
                cx: a_x.clone(),
                ty: var(1),
                left: Term::bind("z", var(1), var(0), var(0)),
                right: var(0),
                equal: true,
            },
            Sample {
                name: "δ at a context definition",
                cx: a_x_d,
                ty: var(2),
                left: var(0),
                right: var(1),
                equal: true,
            },
            Sample {
                name: "η at Π",
                cx: g,
                ty: Term::pi("z", var(1), var(2)),
                left: var(0),
                right: Term::lam("z", Term::app(var(1), var(0))),
                equal: true,
            },
            Sample {
                name: "η under a blocked application",
                cx: f_of_arrow,
                ty: var(2),
                left: Term::app(var(0), var(1)),
                right: Term::app(var(0), Term::lam("z", Term::app(var(2), var(0)))),
                equal: true,
            },
            Sample {
                name: "η at a record",
                cx: pair.clone(),
                ty: Term::record_type([("fst", var(1)), ("snd", var(2))]),
                left: var(0),
                right: Term::record([
                    ("fst", Term::project(var(0), "fst")),
                    ("snd", Term::project(var(0), "snd")),
                ]),
                equal: true,
            },
            Sample {
                name: "η at a record whose later field depends on an earlier one",
                cx: dependent_pair,
                ty: dependent_pair_type,
                left: var(0),
                right: Term::record([
                    ("ty", Term::project(var(0), "ty")),
                    ("val", Term::project(var(0), "val")),
                ]),
                equal: true,
            },
            Sample {
                name: "projection",
                cx: a_x.clone(),
                ty: var(1),
                left: Term::project(Term::record([("fst", var(0)), ("snd", var(0))]), "fst"),
                right: var(0),
                equal: true,
            },
            Sample {
                name: "ι at refl",
                cx: a_x,
                ty: var(1),
                left: Term::jay(var(1), var(0), constant_motive(3), var(0), var(0), Term::refl(var(0))),
                right: var(0),
                equal: true,
            },
            Sample {
                name: "a J blocked on a variable is not its base case",
                cx: identified,
                ty: var(3),
                left: Term::jay(var(3), var(2), constant_motive(5), var(2), var(1), var(0)),
                right: var(2),
                equal: false,
            },
            Sample {
                name: "distinct variables",
                cx: a_xy,
                ty: var(2),
                left: var(1),
                right: var(0),
                equal: false,
            },
            Sample {
                name: "distinct functions, both η-expanded",
                cx: two_functions,
                ty: Term::pi("z", var(2), var(3)),
                left: var(0),
                right: var(1),
                equal: false,
            },
            Sample {
                name: "universes are not cumulative",
                cx: empty,
                ty: Term::Universe(Level::ZERO.succ().succ()),
                left: type0(),
                right: Term::Universe(Level::ZERO.succ()),
                equal: false,
            },
            Sample {
                name: "a function type is a term like any other",
                cx: a,
                ty: type0(),
                left: arrow.clone(),
                right: arrow,
                equal: true,
            },
            Sample {
                name: "a record type is a term like any other",
                cx: pair,
                ty: type0(),
                left: Term::record_type([("fst", var(1)), ("snd", var(2))]),
                right: Term::record_type([("fst", var(1)), ("snd", var(2))]),
                equal: true,
            },
        ])
    }
}
