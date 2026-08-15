//! Every `musa-core` integration test, linked as one binary.
//!
//! One test target per file means one more link of the whole workspace on every
//! build, and one more set of object files that cargo never reclaims from
//! `target/debug/deps`. See `docs/notes/toolchain/slow-test-suite.md`.
//!
//! The suites state `docs/rules/language/02-core-calculus.md`'s §2, §3, §4, and
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
mod coverage_laws;
mod elaboration_laws;
mod family_laws;
mod normalization_laws;
mod provenance_laws;
mod termination_laws;
mod unification_laws;

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

/// The raw programs the elaboration suites are stated over.
///
/// Closed, every one of them, and that is forced rather than chosen: a raw term
/// resolves names against binders it introduced itself, and a binder the caller's
/// context already held has no name for it to resolve to (§2, and `scope.rs`).
///
/// The corpus has no `data` and no primitives, because prompt 134 has neither.
/// What stands in for them is the **empty record**: `{}` as a type is the
/// smallest thing in `Type 0`, `{}` as a literal is its one inhabitant, and
/// `Id (Type 1) (Type 0) (Type 0)` supplies a second family with a constructor.
/// Everything below is built from those.
pub(crate) mod programs {
    use musa_core::{ElabError, Level, Origin, Raw, Refusal, Term};

    /// Where every raw term in the corpus says it was written.
    ///
    /// One origin is enough here: `provenance_laws.rs` is the suite about which
    /// origin lands where, and these are about what elaborates to what.
    pub(crate) const WRITTEN: Origin = Origin::node(400);

    /// A program elaboration must accept.
    pub(crate) struct Program {
        /// What the program exercises.
        pub(crate) name: &'static str,
        pub(crate) raw: Raw,
        /// The type to check it against, or `None` to infer one.
        pub(crate) ty: Option<Term>,
    }

    /// A program elaboration must refuse, and the refusal it owes.
    pub(crate) struct Refused {
        pub(crate) name: &'static str,
        pub(crate) raw: Raw,
        pub(crate) ty: Option<Term>,
        /// Whether the refusal is the one this program is about.
        pub(crate) expected: fn(&Refusal) -> bool,
    }

    /// `{}` as a type: the unit of this corpus, in `Type 0`.
    pub(crate) fn unit_type() -> Raw {
        Raw::record_type(WRITTEN, [])
    }

    /// `{}` as a value: unit's one inhabitant.
    pub(crate) fn unit() -> Raw {
        Raw::record(WRITTEN, [])
    }

    /// The same type as a core term, for a checking question.
    pub(crate) fn core_unit_type() -> Term {
        Term::record_type(WRITTEN, [])
    }

    /// `({} : {})` — the unit value where a type has to be *inferred* from it.
    ///
    /// An argument filling an explicit binder whose type is still a
    /// metavariable is checked against that metavariable, and §2 gives a record
    /// literal no rule there: the literal is an introduction form, and a
    /// metavariable is not a record type it could check field by field. Writing
    /// the annotation is what an author does, and it is what determines the
    /// implicit.
    pub(crate) fn annotated_unit() -> Raw {
        Raw::annot(WRITTEN, unit(), unit_type())
    }

    fn type0() -> Raw {
        Raw::universe(WRITTEN, Level::ZERO)
    }

    fn var(name: &'static str) -> Raw {
        Raw::var(WRITTEN, name)
    }

    /// `{X : Type 0} → X → X`, the polymorphic identity's type.
    fn implicit_identity_type() -> Raw {
        Raw::implicit_pi(WRITTEN, "X", type0(), Raw::pi(WRITTEN, "_", var("X"), var("X")))
    }

    /// `λ{X}. λx. x`.
    fn implicit_identity() -> Raw {
        Raw::implicit_lam(WRITTEN, "X", Raw::lam(WRITTEN, "x", var("x")))
    }

    /// `(y : {}) → Id {} {} y → Type 0`'s inhabitant, written unannotated so
    /// that both of its binder types are metavariables.
    /// `{X : Type 0} → X → X` as a core term, for a checking question.
    fn core_implicit_identity_type() -> Term {
        Term::implicit_pi(
            WRITTEN,
            "X",
            Term::universe(WRITTEN, Level::ZERO),
            Term::pi(
                WRITTEN,
                "_",
                Term::var(WRITTEN, musa_core::Index(0)),
                Term::var(WRITTEN, musa_core::Index(1)),
            ),
        )
    }

    fn constant_motive() -> Raw {
        Raw::lam(WRITTEN, "y", Raw::lam(WRITTEN, "e", unit_type()))
    }

    pub(crate) fn accepted() -> Vec<Program> {
        vec![
            Program {
                name: "the unit value at its type",
                raw: unit(),
                ty: Some(core_unit_type()),
            },
            Program {
                name: "an annotated identity function, inferred",
                raw: Raw::annotated_lam(WRITTEN, "x", unit_type(), var("x")),
                ty: None,
            },
            Program {
                name: "an unannotated identity function, checked",
                raw: Raw::lam(WRITTEN, "x", var("x")),
                ty: Some(Term::pi(WRITTEN, "x", core_unit_type(), core_unit_type())),
            },
            Program {
                name: "a binder shadowing an outer one",
                raw: Raw::annotated_lam(
                    WRITTEN,
                    "x",
                    unit_type(),
                    Raw::annotated_lam(WRITTEN, "x", unit_type(), var("x")),
                ),
                ty: None,
            },
            Program {
                name: "an implicit abstraction, checked against an implicit Pi",
                raw: implicit_identity(),
                ty: Some(core_implicit_identity_type()),
            },
            Program {
                name: "an implicit inserted at a use site",
                raw: Raw::annotated_bind(
                    WRITTEN,
                    "id",
                    implicit_identity_type(),
                    implicit_identity(),
                    Raw::app(WRITTEN, var("id"), annotated_unit()),
                ),
                ty: None,
            },
            Program {
                name: "an implicit written at the use site rather than inserted",
                raw: Raw::annotated_bind(
                    WRITTEN,
                    "id",
                    implicit_identity_type(),
                    implicit_identity(),
                    Raw::app(WRITTEN, Raw::implicit_app(WRITTEN, var("id"), unit_type()), unit()),
                ),
                ty: None,
            },
            Program {
                name: "a term checked against an implicit Pi is abstracted, not switched",
                raw: Raw::lam(WRITTEN, "x", var("x")),
                ty: Some(Term::implicit_pi(
                    WRITTEN,
                    "X",
                    Term::universe(WRITTEN, Level::ZERO),
                    Term::pi(
                        WRITTEN,
                        "x",
                        Term::var(WRITTEN, musa_core::Index(0)),
                        Term::var(WRITTEN, musa_core::Index(1)),
                    ),
                )),
            },
            Program {
                name: "a dependent record type",
                raw: Raw::record_type(WRITTEN, [("ty", type0()), ("val", var("ty"))]),
                ty: None,
            },
            Program {
                name: "a dependent record literal",
                raw: Raw::record(WRITTEN, [("ty", unit_type()), ("val", unit())]),
                ty: Some(Term::record_type(
                    WRITTEN,
                    [
                        ("ty", Term::universe(WRITTEN, Level::ZERO)),
                        ("val", Term::var(WRITTEN, musa_core::Index(0))),
                    ],
                )),
            },
            Program {
                name: "a projection",
                // Through a `let` rather than straight out of the literal,
                // because §2 gives a record literal no inference rule: the
                // author names the type once and both the projection and the
                // re-checker read it from there.
                raw: Raw::annotated_bind(
                    WRITTEN,
                    "r",
                    Raw::record_type(WRITTEN, [("ty", type0()), ("val", var("ty"))]),
                    Raw::record(WRITTEN, [("ty", unit_type()), ("val", unit())]),
                    Raw::project(WRITTEN, var("r"), "val"),
                ),
                ty: None,
            },
            Program {
                name: "reflexivity at unit",
                raw: Raw::refl(WRITTEN, unit()),
                ty: Some(Term::identity(
                    WRITTEN,
                    core_unit_type(),
                    Term::record(WRITTEN, []),
                    Term::record(WRITTEN, []),
                )),
            },
            Program {
                name: "J at a constant motive, with both binder types inferred",
                raw: Raw::jay(
                    WRITTEN,
                    unit_type(),
                    unit(),
                    constant_motive(),
                    unit(),
                    unit(),
                    Raw::refl(WRITTEN, unit()),
                ),
                ty: None,
            },
            Program {
                name: "an annotation re-entering checking mode",
                raw: Raw::annot(WRITTEN, unit(), unit_type()),
                ty: None,
            },
            Program {
                name: "a let whose type is inferred",
                raw: Raw::bind(WRITTEN, "u", annotated_unit(), var("u")),
                ty: None,
            },
            Program {
                name: "a universe",
                raw: type0(),
                ty: Some(Term::universe(WRITTEN, Level::ZERO.succ())),
            },
        ]
    }

    pub(crate) fn refused() -> Vec<Refused> {
        vec![
            Refused {
                name: "a name with no binder",
                raw: var("nowhere"),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::UnknownName { .. }),
            },
            Refused {
                name: "a universe checked one level too low",
                raw: type0(),
                ty: Some(Term::universe(WRITTEN, Level::ZERO)),
                expected: |refusal| matches!(refusal, Refusal::Mismatch(_)),
            },
            Refused {
                name: "applying something that is not a function",
                // A universe rather than a record literal: the literal has no
                // inference rule at all, so it would be refused a step earlier
                // and for a different reason.
                raw: Raw::app(WRITTEN, type0(), unit()),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::NotAFunction { .. }),
            },
            Refused {
                name: "an implicit argument at an explicit binder",
                raw: Raw::implicit_app(WRITTEN, Raw::annotated_lam(WRITTEN, "x", unit_type(), var("x")), unit()),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::PlicityMismatch { .. }),
            },
            Refused {
                name: "an implicit abstraction at an explicit Pi",
                raw: Raw::implicit_lam(WRITTEN, "x", unit()),
                ty: Some(Term::pi(WRITTEN, "x", core_unit_type(), core_unit_type())),
                expected: |refusal| matches!(refusal, Refusal::PlicityMismatch { .. }),
            },
            Refused {
                name: "projecting something that is not a record",
                raw: Raw::project(WRITTEN, type0(), "f"),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::NotARecord { .. }),
            },
            Refused {
                name: "projecting a field the record type does not have",
                raw: Raw::annotated_bind(
                    WRITTEN,
                    "r",
                    Raw::record_type(WRITTEN, [("a", unit_type())]),
                    Raw::record(WRITTEN, [("a", unit())]),
                    Raw::project(WRITTEN, var("r"), "b"),
                ),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::NoSuchField { .. }),
            },
            Refused {
                name: "a record literal with no type to check against",
                // §2: introduction forms check. The type such a literal
                // "obviously" has is a guess — this one inhabits both
                // `{ ty : Type 0, val : ty }` and `{ ty : Type 0, val : {} }` —
                // so elaboration asks rather than picks.
                raw: Raw::record(WRITTEN, [("ty", unit_type()), ("val", unit())]),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::Uninferable { .. }),
            },
            Refused {
                name: "a record literal whose fields are not the type's",
                raw: Raw::record(WRITTEN, [("b", unit())]),
                ty: Some(Term::record_type(WRITTEN, [("a", core_unit_type())])),
                expected: |refusal| matches!(refusal, Refusal::RecordShape { .. }),
            },
            Refused {
                name: "a value standing in type position",
                // Annotated, so that it has a type at all: a bare literal is
                // refused for want of one before anything asks whether it is a
                // universe.
                raw: Raw::pi(WRITTEN, "x", annotated_unit(), unit_type()),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::NotAType { .. }),
            },
            Refused {
                name: "a binder type nothing determines",
                raw: Raw::lam(WRITTEN, "x", var("x")),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::Unsolved { .. }),
            },
        ]
    }

    /// The refusal an error carries, or a panic naming what arrived instead.
    ///
    /// Exhaustion and malformedness are *not* refusals (§4), so a suite that
    /// accepted either as "the program was rejected" would be testing the
    /// opposite of what §4 says.
    ///
    /// # Panics
    ///
    /// When the error is not a refusal.
    pub(crate) fn refusal(name: &str, error: ElabError) -> Refusal {
        match error {
            ElabError::Refused(refusal) => refusal,
            ElabError::Exhausted(exhausted) => {
                panic!("{name}: the budget ended the judgment ({exhausted}) rather than refusing it")
            }
            ElabError::Malformed(malformed) => panic!("{name}: {malformed}"),
        }
    }
}
