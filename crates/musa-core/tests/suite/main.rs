//! Every `musa-core` integration test, linked as one binary.
//!
//! One test target per file means one more link of the whole workspace on every
//! build, and one more set of object files that cargo never reclaims from
//! `target/debug/deps`. See `docs/notes/toolchain/slow-test-suite.md`.
//!
//! The four suites state `docs/rules/language/02-core-calculus.md`'s §3, §4, and
//! §7 obligations as tests. They share one corpus, deliberately: a law that held
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
mod provenance_laws;

/// The terms every law suite is stated over.
pub(crate) mod fixtures {
    use musa_core::{Budget, CoreError, Cx, Index, Level, Origin, Term};

    /// The origin every node of a sample's *type* carries.
    ///
    /// Types and terms are given different origins on purpose. §7 fixes which
    /// origin each quoted node takes, and almost every way of getting it wrong
    /// takes the origin of the type that drove quotation instead of the value
    /// that was quoted. With one origin for both, no test could tell.
    pub(crate) const TYPES: Origin = Origin::node(100);

    /// The origin every node of a sample's *terms* carries.
    pub(crate) const TERMS: Origin = Origin::node(200);

    /// The origin every binder in a sample's context was written at.
    ///
    /// A variable occurrence in a normal form carries *this* rather than
    /// [`TERMS`], and that is §7's substitution clause rather than an accident:
    /// in `e[a/x]` the occurrences of `x` become `a` and carry `a`'s origins, so
    /// an assumption's occurrences carry the assumption's.
    pub(crate) const BINDERS: Origin = Origin::node(300);

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
        Term::var(TERMS, Index(index))
    }

    fn type_var(index: u32) -> Term {
        Term::var(TYPES, Index(index))
    }

    fn type0() -> Term {
        Term::universe(TYPES, Level::ZERO)
    }

    fn arrow(domain: Term, codomain: Term) -> Term {
        Term::pi(TYPES, "z", domain, codomain)
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
        let a = empty.assume(BINDERS, &type0())?;
        // A : Type 0, x : A
        let a_x = a.assume(BINDERS, &type_var(0))?;
        // A : Type 0, x : A, y : A
        let a_xy = a_x.assume(BINDERS, &type_var(1))?;
        // A : Type 0, x : A, d := x : A
        let a_x_d = a_x.define(&type_var(1), &var(0))?;

        // A : Type 0, g : A → A
        let a_to_a = arrow(type_var(0), type_var(1));
        let g = a.assume(BINDERS, &a_to_a)?;
        // A : Type 0, g : A → A, f : (A → A) → A
        let f_of_arrow = g.assume(
            BINDERS,
            &Term::pi(TYPES, "h", arrow(type_var(1), type_var(2)), type_var(2)),
        )?;
        // A : Type 0, g : A → A, f : A → A
        let two_functions = g.assume(BINDERS, &arrow(type_var(1), type_var(2)))?;

        // A : Type 0, r : { fst : A, snd : A }
        let pair = a.assume(
            BINDERS,
            &Term::record_type(TYPES, [("fst", type_var(0)), ("snd", type_var(1))]),
        )?;
        // r : { ty : Type 0, val : ty }
        let dependent_pair_type = Term::record_type(TYPES, [("ty", type0()), ("val", type_var(0))]);
        let dependent_pair = empty.assume(BINDERS, &dependent_pair_type)?;

        // A : Type 0, x : A, y : A, p : Id A x y
        let identified = a_xy.assume(BINDERS, &Term::identity(TYPES, type_var(2), type_var(1), type_var(0)))?;
        // λy. λe. A, a constant motive: `J`'s result type is then `A` at every
        // endpoint, which is what lets ι be stated without a second family.
        let constant_motive = |depth: u32| Term::lam(TERMS, "y", Term::lam(TERMS, "e", var(depth)));

        Ok(vec![
            Sample {
                name: "β",
                cx: a_x.clone(),
                ty: type_var(1),
                left: Term::app(TERMS, Term::lam(TERMS, "z", var(0)), var(0)),
                right: var(0),
                equal: true,
            },
            Sample {
                name: "δ at a let",
                cx: a_x.clone(),
                ty: type_var(1),
                left: Term::bind(TERMS, "z", type_var(1), var(0), var(0)),
                right: var(0),
                equal: true,
            },
            Sample {
                name: "δ at a context definition",
                cx: a_x_d,
                ty: type_var(2),
                left: var(0),
                right: var(1),
                equal: true,
            },
            Sample {
                name: "η at Π",
                cx: g,
                ty: arrow(type_var(1), type_var(2)),
                left: var(0),
                right: Term::lam(TERMS, "z", Term::app(TERMS, var(1), var(0))),
                equal: true,
            },
            Sample {
                name: "η under a blocked application",
                cx: f_of_arrow,
                ty: type_var(2),
                left: Term::app(TERMS, var(0), var(1)),
                right: Term::app(TERMS, var(0), Term::lam(TERMS, "z", Term::app(TERMS, var(2), var(0)))),
                equal: true,
            },
            Sample {
                name: "η at a record",
                cx: pair.clone(),
                ty: Term::record_type(TYPES, [("fst", type_var(1)), ("snd", type_var(2))]),
                left: var(0),
                right: Term::record(
                    TERMS,
                    [
                        ("fst", Term::project(TERMS, var(0), "fst")),
                        ("snd", Term::project(TERMS, var(0), "snd")),
                    ],
                ),
                equal: true,
            },
            Sample {
                name: "η at a record whose later field depends on an earlier one",
                cx: dependent_pair,
                ty: dependent_pair_type,
                left: var(0),
                right: Term::record(
                    TERMS,
                    [
                        ("ty", Term::project(TERMS, var(0), "ty")),
                        ("val", Term::project(TERMS, var(0), "val")),
                    ],
                ),
                equal: true,
            },
            Sample {
                name: "projection",
                cx: a_x.clone(),
                ty: type_var(1),
                left: Term::project(TERMS, Term::record(TERMS, [("fst", var(0)), ("snd", var(0))]), "fst"),
                right: var(0),
                equal: true,
            },
            Sample {
                name: "ι at refl",
                cx: a_x,
                ty: type_var(1),
                left: Term::jay(
                    TERMS,
                    type_var(1),
                    var(0),
                    constant_motive(3),
                    var(0),
                    var(0),
                    Term::refl(TERMS, var(0)),
                ),
                right: var(0),
                equal: true,
            },
            Sample {
                name: "a J blocked on a variable is not its base case",
                cx: identified,
                ty: type_var(3),
                left: Term::jay(TERMS, type_var(3), var(2), constant_motive(5), var(2), var(1), var(0)),
                right: var(2),
                equal: false,
            },
            Sample {
                name: "distinct variables",
                cx: a_xy,
                ty: type_var(2),
                left: var(1),
                right: var(0),
                equal: false,
            },
            Sample {
                name: "distinct functions, both η-expanded",
                cx: two_functions,
                ty: arrow(type_var(2), type_var(3)),
                left: var(0),
                right: var(1),
                equal: false,
            },
            Sample {
                name: "universes are not cumulative",
                cx: empty,
                ty: Term::universe(TYPES, Level::ZERO.succ().succ()),
                left: Term::universe(TERMS, Level::ZERO),
                right: Term::universe(TERMS, Level::ZERO.succ()),
                equal: false,
            },
            Sample {
                name: "a function type is a term like any other",
                cx: a,
                ty: type0(),
                left: a_to_a.clone(),
                right: a_to_a,
                equal: true,
            },
            Sample {
                name: "a record type is a term like any other",
                cx: pair,
                ty: type0(),
                left: Term::record_type(TYPES, [("fst", type_var(1)), ("snd", type_var(2))]),
                right: Term::record_type(TYPES, [("fst", type_var(1)), ("snd", type_var(2))]),
                equal: true,
            },
        ])
    }
}
