//! Every `musa-calculus` integration test, linked as one binary.
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

mod base_laws;
mod boundary_laws;
mod budget_laws;
mod collection_laws;
mod conversion_laws;
mod coverage_laws;
mod elaboration_laws;
mod family_laws;
mod generated_laws;
mod glued_laws;
mod implicit_laws;
mod lifting_laws;
mod malformed_laws;
mod namespace_laws;
mod nesting_laws;
mod normalization_laws;
mod numeral_laws;
mod program_laws;
mod provenance_laws;
mod recheck_laws;
mod record_laws;
mod sort_laws;
mod storable_laws;
mod termination_laws;
mod tree_laws;
mod unification_laws;
mod unify_laws;
mod visibility_laws;

/// The terms every law suite is stated over.
pub(crate) mod fixtures {
    use musa_calculus::{Budget, Cx, ElabError, Index, Origin, Role, Sort, Term};

    use crate::family_laws::{binder, constructor, data, family, var as raw_var};

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
    /// The type is not decoration. §3 performs η at Π and at a one-constructor
    /// family during quotation, so "are these two terms equal" is not a question
    /// that can be asked without saying equal *at what*.
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
        Term::universe(TYPES, Sort::ZERO)
    }

    fn arrow(domain: Term, codomain: Term) -> Term {
        Term::pi(TYPES, "z", domain, codomain)
    }

    /// `Pair A`, the corpus's one-constructor family at a parameter.
    fn pair_of(argument: Term) -> Term {
        Term::app(TYPES, Term::named(TYPES, "Pair", Role::TypeConstructor), argument)
    }

    /// `Pair.<field> A r` — the generated accessor, applied to the parameter it
    /// takes before its subject.
    ///
    /// A projection is a constant and not a shape after prompt 157, so reading a
    /// field is an ordinary two-argument application. That is exactly what makes
    /// the η sample below a statement about a *family* rather than about a term
    /// former the core no longer has.
    fn field_of(field: &str, parameter: Term, subject: Term) -> Term {
        Term::app(
            TERMS,
            Term::app(
                TERMS,
                Term::named(TERMS, format!("Pair.{field}"), Role::Projection),
                parameter,
            ),
            subject,
        )
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
    /// [`ElabError::Exhausted`] when `budget` cannot evaluate one of the types a
    /// context is extended by, or declare one of the two families the η samples
    /// are stated over.
    pub(crate) fn corpus_at(budget: Budget) -> Result<Vec<Sample>, ElabError> {
        let empty = Cx::with_budget(budget);
        // `data Pair (A : Type 0) { Pair(fst: A, snd: A) }` and
        // `data Dep { Dep(ty: Type 0, val: ty) }`: the two families η is stated
        // over, the second one so that a later field standing under an earlier
        // one is in the corpus rather than only in `record_laws.rs`.
        let pairs = musa_calculus::declare(
            &empty,
            &data(
                vec![binder("A", musa_calculus::Raw::universe(TYPES, Sort::ZERO))],
                vec![family(
                    "Pair",
                    vec![constructor(
                        "Pair",
                        vec![binder("fst", raw_var("A")), binder("snd", raw_var("A"))],
                    )],
                )],
            ),
        )?;
        let empty = empty.declaring(&pairs);
        let deps = musa_calculus::declare(
            &empty,
            &data(
                Vec::new(),
                vec![family(
                    "Dep",
                    vec![constructor(
                        "Dep",
                        vec![
                            binder("ty", musa_calculus::Raw::universe(TYPES, Sort::ZERO)),
                            binder("val", raw_var("ty")),
                        ],
                    )],
                )],
            ),
        )?;
        let empty = empty.declaring(&deps);
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

        // A : Type 0, r : Pair A
        let pair = a.assume(BINDERS, &pair_of(type_var(0)))?;
        // d : Dep
        let dependent_pair_type = Term::named(TYPES, "Dep", Role::TypeConstructor);
        let dependent_pair = empty.assume(BINDERS, &dependent_pair_type)?;

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
                name: "η at a one-constructor family",
                cx: pair.clone(),
                ty: pair_of(type_var(1)),
                left: var(0),
                right: Term::app(
                    TERMS,
                    Term::app(
                        TERMS,
                        Term::app(
                            TERMS,
                            Term::named(TERMS, "Pair.Pair", Role::Constructor),
                            Term::var(TERMS, Index(1)),
                        ),
                        field_of("fst", Term::var(TERMS, Index(1)), var(0)),
                    ),
                    field_of("snd", Term::var(TERMS, Index(1)), var(0)),
                ),
                equal: true,
            },
            Sample {
                name: "η at a family whose later field depends on an earlier one",
                cx: dependent_pair,
                ty: dependent_pair_type,
                left: var(0),
                right: Term::app(
                    TERMS,
                    Term::app(
                        TERMS,
                        Term::named(TERMS, "Dep.Dep", Role::Constructor),
                        Term::app(TERMS, Term::named(TERMS, "Dep.ty", Role::Projection), var(0)),
                    ),
                    Term::app(TERMS, Term::named(TERMS, "Dep.val", Role::Projection), var(0)),
                ),
                equal: true,
            },
            Sample {
                name: "ι at a generated accessor",
                cx: a_x,
                ty: type_var(1),
                left: field_of(
                    "fst",
                    Term::var(TERMS, Index(1)),
                    Term::app(
                        TERMS,
                        Term::app(
                            TERMS,
                            Term::app(
                                TERMS,
                                Term::named(TERMS, "Pair.Pair", Role::Constructor),
                                Term::var(TERMS, Index(1)),
                            ),
                            var(0),
                        ),
                        var(0),
                    ),
                ),
                right: var(0),
                equal: true,
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
                ty: Term::universe(TYPES, Sort::ONE),
                left: Term::universe(TERMS, Sort::ZERO),
                right: Term::pi(TYPES, "_", type0(), type0()),
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
                name: "a family's type is a term like any other",
                cx: pair,
                ty: type0(),
                left: pair_of(type_var(1)),
                right: pair_of(type_var(1)),
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
/// What stands in for primitives is two declarations, in [`cx`]. `Unit` is the
/// smallest thing in `Type 0` and `Unit.Unit` is its one inhabitant; `Cell` is a
/// one-constructor family whose second field stands under its first, which is
/// what a record is after prompt 157. Before that prompt the same two roles were
/// played by the empty record and `{ ty : Type 0, val : ty }`, written inline
/// because the core had shapes for them; the terms below are the same programs
/// against the declarations that replaced those shapes.
///
/// A program here is closed *given those two declarations*: a constant is a name
/// and not an index, so `recheck_laws.rs`'s closure law still reads.
pub(crate) mod programs {
    use musa_calculus::{Cx, ElabError, Origin, Raw, RawData, Refusal, Role, Sort, Term};

    use crate::family_laws::{binder, constructor, data, family};

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

    /// `data Unit { Unit }` and `data Cell { Cell(ty: Type 0, val: ty) }`.
    pub(crate) fn declarations() -> [RawData; 2] {
        [
            data(Vec::new(), vec![family("Unit", vec![constructor("Unit", Vec::new())])]),
            data(
                Vec::new(),
                vec![family(
                    "Cell",
                    vec![constructor(
                        "Cell",
                        vec![binder("ty", type0()), binder("val", var("ty"))],
                    )],
                )],
            ),
        ]
    }

    /// The context every program in this corpus is elaborated in.
    ///
    /// # Panics
    ///
    /// If either declaration is refused, which would be a defect in this crate.
    pub(crate) fn cx() -> Cx {
        declarations().iter().fold(Cx::new(), |cx, declared| {
            let group = musa_calculus::declare(&cx, declared).expect("the corpus declares two families");
            cx.declaring(&group)
        })
    }

    /// `Unit`: the smallest thing in `Type 0`.
    pub(crate) fn unit_type() -> Raw {
        var("Unit")
    }

    /// `Unit.Unit`: its one inhabitant.
    pub(crate) fn unit() -> Raw {
        var("Unit.Unit")
    }

    /// The same type as a core term, for a checking question.
    pub(crate) fn core_unit_type() -> Term {
        Term::named(WRITTEN, "Unit", Role::TypeConstructor)
    }

    /// `Cell` as a core term.
    pub(crate) fn core_cell_type() -> Term {
        Term::named(WRITTEN, "Cell", Role::TypeConstructor)
    }

    /// `(Unit.Unit : Unit)` — the unit value where a type has to be *inferred*
    /// from it.
    ///
    /// An argument filling an explicit binder whose type is still a
    /// metavariable is checked against that metavariable, and §2 gives a record
    /// literal no rule there: the literal is an introduction form, and a
    /// metavariable is not a family it could check field by field. Writing the
    /// annotation is what an author does, and it is what determines the
    /// implicit.
    pub(crate) fn annotated_unit() -> Raw {
        Raw::annot(WRITTEN, unit(), unit_type())
    }

    fn type0() -> Raw {
        Raw::universe(WRITTEN, Sort::ZERO)
    }

    fn var(name: &'static str) -> Raw {
        Raw::var(WRITTEN, name)
    }

    /// `{X : Type 0} → X → X`, the polymorphic identity's type.
    fn implicit_identity_type() -> Raw {
        Raw::parameter_pi(WRITTEN, "X", type0(), Raw::pi(WRITTEN, "_", var("X"), var("X")))
    }

    /// `λ{X}. λx. x`.
    fn implicit_identity() -> Raw {
        Raw::parameter_lam(WRITTEN, "X", Raw::lam(WRITTEN, "x", var("x")))
    }

    /// `{X : Type 0} → X → X` as a core term, for a checking question.
    fn core_implicit_identity_type() -> Term {
        Term::parameter_pi(
            WRITTEN,
            "X",
            Term::universe(WRITTEN, Sort::ZERO),
            Term::pi(
                WRITTEN,
                "_",
                Term::var(WRITTEN, musa_calculus::Index(0)),
                Term::var(WRITTEN, musa_calculus::Index(1)),
            ),
        )
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
                name: "an implicit supplied by the name of the binder it fills",
                // `01-surface.md` §1's `{ IDENT = expr }`. In the re-checked
                // term it is an ordinary application, which is the point: the
                // name is a way of *writing* the argument and not a second
                // kind of argument.
                raw: Raw::annotated_bind(
                    WRITTEN,
                    "id",
                    implicit_identity_type(),
                    implicit_identity(),
                    Raw::call_supplying(WRITTEN, var("id"), [unit()], [("X", unit_type())]),
                ),
                ty: Some(core_unit_type()),
            },
            Program {
                name: "a scheme kept where its own type is expected",
                // Prompt 154's stopping rule: neither insertion rule fires, so
                // the term is the definition itself and the re-checker sees a
                // Π where a λ would otherwise stand.
                raw: Raw::annotated_bind(WRITTEN, "id", implicit_identity_type(), implicit_identity(), var("id")),
                ty: Some(core_implicit_identity_type()),
            },
            Program {
                name: "a term checked against an implicit Pi is abstracted, not switched",
                raw: Raw::lam(WRITTEN, "x", var("x")),
                ty: Some(Term::parameter_pi(
                    WRITTEN,
                    "X",
                    Term::universe(WRITTEN, Sort::ZERO),
                    Term::pi(
                        WRITTEN,
                        "x",
                        Term::var(WRITTEN, musa_calculus::Index(0)),
                        Term::var(WRITTEN, musa_calculus::Index(1)),
                    ),
                )),
            },
            Program {
                name: "a one-constructor family's type",
                raw: var("Cell"),
                ty: None,
            },
            Program {
                name: "a record literal at a dependent one-constructor family",
                raw: Raw::record(WRITTEN, [("ty", unit_type()), ("val", unit())]),
                ty: Some(core_cell_type()),
            },
            Program {
                name: "a projection",
                // Through a `let` rather than straight out of the literal,
                // because §2 gives a record literal no inference rule: the
                // author names the type once and both the projection and the
                // re-checker read it from there. This is prompt 157's
                // re-checker obligation — the generated accessor has to survive
                // `recheck_laws.rs` like any other constant.
                raw: Raw::annotated_bind(
                    WRITTEN,
                    "r",
                    var("Cell"),
                    Raw::record(WRITTEN, [("ty", unit_type()), ("val", unit())]),
                    Raw::project(WRITTEN, var("r"), "val"),
                ),
                ty: None,
            },
            Program {
                name: "a lambda applied through the let in front of it",
                // What `generated_laws.rs` found. Elaboration reads the domain
                // off `fn (x : Unit)`'s annotation and a core λ has nowhere to
                // keep it, so the audit has to recover the domain from the
                // argument — which it can only do by reading the `let` and the
                // λ as the one nested binding the evaluator reduces them to.
                raw: Raw::app(
                    WRITTEN,
                    Raw::annotated_bind(
                        WRITTEN,
                        "u",
                        unit_type(),
                        unit(),
                        Raw::annotated_lam(WRITTEN, "x", unit_type(), var("x")),
                    ),
                    unit(),
                ),
                ty: None,
            },
            Program {
                name: "a lambda applied through the lets interleaved with its binders",
                // The same shape with the two kinds of binder alternating, so
                // the peel is exercised where a `let` stands *under* a binder
                // an argument already filled and the later argument still does
                // not stand under either.
                raw: Raw::app(
                    WRITTEN,
                    Raw::app(
                        WRITTEN,
                        Raw::annotated_bind(
                            WRITTEN,
                            "u",
                            unit_type(),
                            unit(),
                            Raw::annotated_lam(
                                WRITTEN,
                                "x",
                                unit_type(),
                                Raw::annotated_bind(
                                    WRITTEN,
                                    "v",
                                    unit_type(),
                                    var("x"),
                                    Raw::annotated_lam(WRITTEN, "y", unit_type(), var("v")),
                                ),
                            ),
                        ),
                        unit(),
                    ),
                    unit(),
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
                ty: Some(Term::universe(WRITTEN, Sort::ONE)),
            },
            Program {
                name: "an implicit written against an implicit binder",
                // The one written-implicit rule: a host scheme's implicit
                // binder may be filled in braces at the use site.
                raw: Raw::annotated_bind(
                    WRITTEN,
                    "id",
                    implicit_identity_type(),
                    implicit_identity(),
                    Raw::app(WRITTEN, Raw::parameter_app(WRITTEN, var("id"), unit_type()), unit()),
                ),
                ty: Some(core_unit_type()),
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
                name: "an implicit written against an explicit binder",
                // A written `{…}` fills the implicit binder of a
                // host-generated scheme and nothing else; against an ordinary
                // function it is refused where it is written.
                raw: Raw::annotated_bind(
                    WRITTEN,
                    "id",
                    Raw::pi(WRITTEN, "X", type0(), Raw::pi(WRITTEN, "_", var("X"), var("X"))),
                    Raw::lam(WRITTEN, "X", Raw::lam(WRITTEN, "x", var("x"))),
                    Raw::app(WRITTEN, Raw::parameter_app(WRITTEN, var("id"), unit_type()), unit()),
                ),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::FillingMismatch { .. }),
            },
            Refused {
                name: "a type parameter supplied by a name the callee does not bear",
                // `01-surface.md` §1's `{ IDENT = expr }` names the *callee's*
                // binder, so a name that matches none of them is a name the
                // author believed the signature had.
                raw: Raw::annotated_bind(
                    WRITTEN,
                    "id",
                    implicit_identity_type(),
                    implicit_identity(),
                    Raw::call_supplying(WRITTEN, var("id"), [annotated_unit()], [("B", unit_type())]),
                ),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::NoSuchParameter { .. }),
            },
            Refused {
                name: "a universe checked one level too low",
                raw: type0(),
                ty: Some(Term::universe(WRITTEN, Sort::ZERO)),
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
                name: "a call that supplies none of the one parameter",
                // §1.3: "a call must be complete". The other direction of the
                // program above — that one applied a non-function, and this one
                // under-applies a function — and written as a *call* rather than
                // as a bare name, because a name whose type is a function is a
                // perfectly good value and this rule is about argument lists.
                raw: Raw::call(
                    WRITTEN,
                    Raw::annotated_lam(WRITTEN, "x", unit_type(), var("x")),
                    Vec::new(),
                ),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::Underapplied { .. }),
            },
            Refused {
                name: "an implicit argument at an explicit binder",
                raw: Raw::parameter_app(WRITTEN, Raw::annotated_lam(WRITTEN, "x", unit_type(), var("x")), unit()),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::FillingMismatch { .. }),
            },
            Refused {
                name: "an implicit abstraction at an explicit Pi",
                raw: Raw::parameter_lam(WRITTEN, "x", unit()),
                ty: Some(Term::pi(WRITTEN, "x", core_unit_type(), core_unit_type())),
                expected: |refusal| matches!(refusal, Refusal::FillingMismatch { .. }),
            },
            Refused {
                name: "projecting something that is not a record",
                raw: Raw::project(WRITTEN, type0(), "f"),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::NotARecord { .. }),
            },
            Refused {
                name: "projecting a field the family does not declare",
                raw: Raw::annotated_bind(
                    WRITTEN,
                    "r",
                    var("Cell"),
                    Raw::record(WRITTEN, [("ty", unit_type()), ("val", unit())]),
                    Raw::project(WRITTEN, var("r"), "b"),
                ),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::NoSuchField { .. }),
            },
            Refused {
                name: "a record literal with no type to check against",
                // §2: introduction forms check. Which family a literal belongs
                // to is not written anywhere in it — after prompt 157 two
                // declarations may name the same fields at the same types and
                // still be two types — so elaboration asks rather than picks.
                raw: Raw::record(WRITTEN, [("ty", unit_type()), ("val", unit())]),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::Uninferable { .. }),
            },
            Refused {
                name: "a record literal whose fields are not the family's",
                raw: Raw::record(WRITTEN, [("b", unit())]),
                ty: Some(core_cell_type()),
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
                // §2.1: no meta is invented for a binder's type and defaulted
                // later — the λ is checking-only, and at `infer` the report is
                // that the author must write the type.
                raw: Raw::lam(WRITTEN, "x", var("x")),
                ty: None,
                expected: |refusal| matches!(refusal, Refusal::Uninferable { .. }),
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
