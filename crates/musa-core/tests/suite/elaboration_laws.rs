//! Bidirectional elaboration: what it accepts, what it refuses, and the one
//! invariant everything else rests on.
//!
//! `docs/rules/language/02-core-calculus.md` §2 fixes the judgments; prompt 134
//! names the laws. The load-bearing one is
//! [`an_elaborated_term_type_checks_in_the_core`]: elaboration may infer
//! whatever it likes, but what comes out has to survive a checker that shares
//! none of its reasoning. Every other test here is a way of failing that one
//! earlier and more legibly.
//!
//! As in the other suites, these are laws stated over a corpus and therefore
//! discharged at the terms in it. Prompt 148 owes the metatheory matrix.

use musa_core::{Cx, ElabError, Index, Level, Raw, Refusal, Term, check, infer, well_typed};

use crate::programs::{Program, Refused, WRITTEN, accepted, core_unit_type, refusal, refused, unit, unit_type};

/// Elaborate a program the way its corpus entry asks, answering the term and
/// the type it ended up at.
fn elaborate(program: &Program) -> Result<(Term, Term), ElabError> {
    let cx = Cx::new();
    match &program.ty {
        Some(ty) => check(&cx, ty, &program.raw).map(|term| (term, ty.clone())),
        None => infer(&cx, &program.raw),
    }
}

/// **The invariant this prompt exists for.** An elaborated term type-checks in
/// the core, judged by [`well_typed`], which shares no rule with the elaborator.
#[test]
fn an_elaborated_term_type_checks_in_the_core() {
    let cx = Cx::new();
    for program in accepted() {
        let (term, ty) = elaborate(&program).unwrap_or_else(|error| panic!("{}: {error}", program.name));
        assert_eq!(
            well_typed(&cx, &ty, &term),
            Ok(()),
            "{}: the elaborator's own output must re-check",
            program.name
        );
    }
}

/// §2: elaboration is deterministic. The same program elaborates to the same
/// term, at the same type, however many times it is asked.
#[test]
fn elaboration_is_deterministic() {
    for program in accepted() {
        let once = elaborate(&program).unwrap_or_else(|error| panic!("{}: {error}", program.name));
        let twice = elaborate(&program).unwrap_or_else(|error| panic!("{}: {error}", program.name));
        assert_eq!(once.0, twice.0, "{}: the same term", program.name);
        assert_eq!(once.1, twice.1, "{}: at the same type", program.name);
    }
}

/// §2.1: an accepted term holds no metavariables.
///
/// Not a stylistic preference — it is what lets the next stage treat the output
/// as an ordinary core term. A leftover metavariable would be a hole every later
/// pass had to know about.
#[test]
fn an_accepted_term_holds_no_metavariables() {
    for program in accepted() {
        let (term, ty) = elaborate(&program).unwrap_or_else(|error| panic!("{}: {error}", program.name));
        for (what, term) in [("the term", &term), ("its type", &ty)] {
            assert!(
                meta_free(term),
                "{}: {what} still mentions a metavariable",
                program.name
            );
        }
    }
}

/// §2: a term that writes no implicits and annotates its binders elaborates to
/// itself.
///
/// The elaborator is allowed to insert, to solve, and to read back; it is not
/// allowed to *rewrite* a program that asked for none of that. Written out per
/// pair rather than derived, because deriving the expected term from the raw one
/// would be the same walk under test.
#[test]
fn a_program_with_no_implicits_elaborates_to_itself() {
    let cx = Cx::new();
    let record_literal = Term::record(
        WRITTEN,
        [
            ("ty", Term::record_type(WRITTEN, [])),
            ("val", Term::record(WRITTEN, [])),
        ],
    );
    let pairs = [
        (
            "a variable under a binder",
            Raw::annotated_lam(WRITTEN, "x", unit_type(), Raw::var(WRITTEN, "x")),
            None,
            Term::lam(WRITTEN, "x", Term::var(WRITTEN, Index(0))),
        ),
        (
            // Checked rather than inferred, because §2 gives a record literal
            // no inference rule. The law is the same either way: the term the
            // author wrote comes back unchanged.
            "a record literal",
            Raw::record(WRITTEN, [("ty", unit_type()), ("val", unit())]),
            Some(Term::record_type(
                WRITTEN,
                [("ty", Term::universe(WRITTEN, Level::ZERO)), ("val", core_unit_type())],
            )),
            record_literal,
        ),
        (
            "a function type",
            Raw::pi(WRITTEN, "x", unit_type(), unit_type()),
            None,
            Term::pi(WRITTEN, "x", core_unit_type(), core_unit_type()),
        ),
        (
            // Checked, for the same reason as the literal: `refl` infers its
            // witness, and the witness here is a record literal.
            "an identity type and its constructor",
            Raw::refl(WRITTEN, unit()),
            Some(Term::identity(
                WRITTEN,
                core_unit_type(),
                Term::record(WRITTEN, []),
                Term::record(WRITTEN, []),
            )),
            Term::refl(WRITTEN, Term::record(WRITTEN, [])),
        ),
    ];
    for (name, raw, ty, expected) in pairs {
        let term = match &ty {
            Some(ty) => check(&cx, ty, &raw),
            None => infer(&cx, &raw).map(|(term, _)| term),
        };
        let term = term.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(term, expected, "{name}");
    }
}

/// §2: every refusal the elaborator can reach is reachable, and reached by the
/// program it is about.
///
/// The compile-fail half of the suite, and a coverage gate as well as a
/// behaviour test: [`kind`] matches every variant, so a new [`Refusal`] with no
/// program to reach it fails here rather than shipping as an error message
/// nobody has read.
#[test]
fn each_refusal_is_reached_by_the_program_it_is_about() {
    let cx = Cx::new();
    let mut reached = std::collections::BTreeSet::new();
    for Refused {
        name,
        raw,
        ty,
        expected,
    } in refused()
    {
        let outcome = match &ty {
            Some(ty) => check(&cx, ty, &raw).map(|term| (term, ty.clone())),
            None => infer(&cx, &raw),
        };
        let Err(error) = outcome else {
            panic!("{name}: elaboration accepted a program it must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // A declaration is a program too, and two refusals are reached only by one.
    // The gate spans both corpora rather than being satisfied by whichever half
    // this file happens to hold.
    let (declaring, _) = crate::family_laws::nat_context();
    for crate::family_laws::RefusedData {
        name,
        declaration,
        expected,
    } in crate::family_laws::refused_declarations()
    {
        let Err(error) = musa_core::declare(&declaring, &declaration) else {
            panic!("{name}: the declaration was admitted");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // A `match` and a `rec` are programs too, and five refusals are reached only
    // by one. Both corpora are checked in their own suites as well; here they
    // are what keeps the gate from being satisfied by whichever half of the
    // language this file happens to hold.
    let declared = crate::coverage_laws::nat_vec_context();
    let written: Vec<(&str, Raw, Raw, fn(&Refusal) -> bool)> = crate::coverage_laws::refused_matches()
        .into_iter()
        .map(|refused| (refused.name, refused.raw, refused.ty, refused.expected))
        .chain(
            crate::termination_laws::refused_definitions()
                .into_iter()
                .map(|refused| (refused.name, refused.raw, refused.ty, refused.expected)),
        )
        .collect();
    for (name, raw, ty, expected) in written {
        let (ty, _) = infer(&declared, &ty).unwrap_or_else(|error| panic!("{name}: {error}"));
        let Err(error) = check(&declared, &ty, &raw) else {
            panic!("{name}: elaboration accepted a program it must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // Records and enums are programs too, and four refusals are reached only by
    // one. They need their own context rather than joining the loop above: the
    // collision they are about is two families declaring one case name, which no
    // other corpus has a reason to declare.
    let writing = crate::record_laws::tying_context();
    for crate::record_laws::RefusedRecord {
        name,
        raw,
        ty,
        expected,
    } in crate::record_laws::refused_records()
    {
        let (ty, _) = infer(&writing, &ty).unwrap_or_else(|error| panic!("{name}: {error}"));
        let Err(error) = check(&writing, &ty, &raw) else {
            panic!("{name}: elaboration accepted a program it must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // Two refusals are questions about *where* a program is written rather than
    // about what it says, so they need a context standing outside the module
    // that declared what they name.
    let (outside, refused) = crate::visibility_laws::refused_outside();
    for crate::visibility_laws::RefusedOutside {
        name,
        raw,
        ty,
        expected,
    } in refused
    {
        let Err(error) = check(&outside, &ty, &raw) else {
            panic!("{name}: elaboration accepted a program it must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // Traits and instances are refused at a *declaration* rather than at a term
    // checked against a type, so they carry their outcome rather than a program:
    // §4 makes that the rule, and a suite that reached these through a use site
    // would be testing the opposite of it.
    for crate::trait_laws::RefusedDeclaration {
        name,
        outcome,
        expected,
    } in crate::trait_laws::refused_declarations()
    {
        let Err(error) = outcome else {
            panic!("{name}: elaboration accepted a declaration it must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    assert_eq!(
        reached,
        ALL_REFUSALS.iter().copied().collect(),
        "every refusal needs a program that reaches it"
    );
}

/// Every refusal this crate can answer with.
const ALL_REFUSALS: [&str; 39] = [
    "unknown-name",
    "mismatch",
    "unsolved",
    "not-a-function",
    "plicity-mismatch",
    "not-a-record",
    "no-such-field",
    "record-shape",
    "not-a-type",
    "uninferable",
    "non-positive",
    "index-count",
    "no-such-constructor",
    "incomplete-match",
    "unreachable-branch",
    "forced-index",
    "unchecked-recursion",
    "duplicate-field",
    "duplicate-case",
    "overlapping-update",
    "bare-constructor",
    "private",
    "mixed-visibility",
    "abstract-match",
    "reserved-class",
    "headless-class",
    "duplicate-method",
    "class-arity",
    "hand-written-storable",
    "blanket-instance",
    "duplicate-instance",
    "orphan-instance",
    "unbounded-instance",
    "derived-method",
    "no-such-method",
    "missing-method",
    "unresolved-instance",
    "unconstrained-variable",
    "unkeyed-constraint",
];

/// Which refusal this is, as a tag the coverage gate can compare.
///
/// Exhaustive on purpose: a new variant is a compile error here, and then a
/// missing entry in [`ALL_REFUSALS`], and then a missing program.
fn kind(refusal: &Refusal) -> &'static str {
    match refusal {
        Refusal::UnknownName { .. } => "unknown-name",
        Refusal::Mismatch(_) => "mismatch",
        Refusal::Unsolved { .. } => "unsolved",
        Refusal::NotAFunction { .. } => "not-a-function",
        Refusal::PlicityMismatch { .. } => "plicity-mismatch",
        Refusal::NotARecord { .. } => "not-a-record",
        Refusal::NoSuchField { .. } => "no-such-field",
        Refusal::RecordShape { .. } => "record-shape",
        Refusal::NotAType { .. } => "not-a-type",
        Refusal::Uninferable { .. } => "uninferable",
        Refusal::NonPositive { .. } => "non-positive",
        Refusal::IndexCount { .. } => "index-count",
        Refusal::NoSuchConstructor { .. } => "no-such-constructor",
        Refusal::IncompleteMatch { .. } => "incomplete-match",
        Refusal::UnreachableBranch { .. } => "unreachable-branch",
        Refusal::ForcedIndex { .. } => "forced-index",
        Refusal::UncheckedRecursion { .. } => "unchecked-recursion",
        Refusal::DuplicateField { .. } => "duplicate-field",
        Refusal::DuplicateCase { .. } => "duplicate-case",
        Refusal::OverlappingUpdate { .. } => "overlapping-update",
        Refusal::BareConstructor { .. } => "bare-constructor",
        Refusal::Private { .. } => "private",
        Refusal::MixedVisibility { .. } => "mixed-visibility",
        Refusal::AbstractMatch { .. } => "abstract-match",
        Refusal::ReservedClass { .. } => "reserved-class",
        Refusal::HeadlessClass { .. } => "headless-class",
        Refusal::DuplicateMethod { .. } => "duplicate-method",
        Refusal::ClassArity { .. } => "class-arity",
        Refusal::HandWrittenStorable { .. } => "hand-written-storable",
        Refusal::BlanketInstance { .. } => "blanket-instance",
        Refusal::DuplicateInstance { .. } => "duplicate-instance",
        Refusal::OrphanInstance { .. } => "orphan-instance",
        Refusal::UnboundedInstance { .. } => "unbounded-instance",
        Refusal::DerivedMethod { .. } => "derived-method",
        Refusal::NoSuchMethod { .. } => "no-such-method",
        Refusal::MissingMethod { .. } => "missing-method",
        Refusal::UnresolvedInstance { .. } => "unresolved-instance",
        Refusal::UnconstrainedVariable { .. } => "unconstrained-variable",
        Refusal::UnkeyedConstraint { .. } => "unkeyed-constraint",
    }
}

/// §2.1: an unsolved metavariable names its site, where it was created, and
/// what was still blocked.
///
/// The diagnostic's content is the test, not merely its variant: "could not
/// determine something, somewhere" would satisfy a `matches!` and help nobody.
#[test]
fn an_unsolved_metavariable_says_what_could_not_be_determined() {
    let cx = Cx::new();
    let Err(error) = infer(&cx, &Raw::lam(WRITTEN, "x", Raw::var(WRITTEN, "x"))) else {
        panic!("a binder whose type nothing determines must be refused");
    };
    let refusal = refusal("an unannotated identity function", error);
    let Refusal::Unsolved { site, created, .. } = &refusal else {
        panic!("expected an unsolved metavariable, got `{refusal}`");
    };
    assert_eq!(site.describe(), "the type of a binder");
    assert_eq!(*created, WRITTEN, "the report points at the term that made it");
    assert_eq!(refusal.to_string(), "could not determine the type of a binder");
}

/// §4: exhaustion is its own outcome and never a refusal.
///
/// A budget narrow enough to end elaboration must say so, because a resource
/// limit reported as a type error would let the machine decide what a program
/// means.
#[test]
fn a_narrow_budget_exhausts_rather_than_refusing() {
    let cx = Cx::with_budget(musa_core::Budget::LANGUAGE.scaled(4096));
    let program = Raw::annotated_bind(
        WRITTEN,
        "id",
        Raw::pi(WRITTEN, "_", unit_type(), unit_type()),
        Raw::annotated_lam(WRITTEN, "x", unit_type(), Raw::var(WRITTEN, "x")),
        Raw::app(WRITTEN, Raw::var(WRITTEN, "id"), unit()),
    );
    match infer(&cx, &program) {
        Err(ElabError::Exhausted(_)) | Ok(_) => {}
        Err(ElabError::Refused(refusal)) => {
            panic!("a narrow budget refused the program instead of running out of room: {refusal}")
        }
        Err(ElabError::Malformed(malformed)) => panic!("{malformed}"),
    }
}

/// §1 as amended at prompt 134: plicity rides on the core Π and no rule reads
/// it, so the two spellings of one function type are convertible.
///
/// Conversion, not elaboration. Elaboration *does* read plicity — that is what
/// the field is for, and a term checked against an implicit Π is wrapped in an
/// implicit λ rather than switched. What the amendment claims is narrower and is
/// what this asserts: no rule in the *core* reads the field, so the two spellings
/// are one type to equality and one type to normalization.
#[test]
fn plicity_is_not_part_of_conversion() {
    let cx = Cx::new();
    let explicit = Term::pi(WRITTEN, "x", core_unit_type(), core_unit_type());
    let implicit = Term::implicit_pi(WRITTEN, "y", core_unit_type(), core_unit_type());
    assert_eq!(
        explicit, implicit,
        "conversion looks at neither plicity nor binder name"
    );
    assert_eq!(
        musa_core::convertible_types(&cx, &explicit, &implicit),
        Ok(true),
        "and normalization does not reintroduce the distinction"
    );
}

/// Whether a term mentions no metavariable anywhere.
///
/// Written by walking the shape rather than by a `Debug` string, so that a new
/// [`musa_core::Shape`] variant holding a term is a compile error here.
fn meta_free(term: &Term) -> bool {
    use musa_core::Shape;

    match term.shape() {
        Shape::Meta(_) => false,
        Shape::Var(_) | Shape::Universe(_) | Shape::Const(_) => true,
        Shape::Pi { domain, codomain, .. } => meta_free(domain) && meta_free(codomain),
        Shape::Lam { body, .. } => meta_free(body),
        Shape::App { function, argument } => meta_free(function) && meta_free(argument),
        Shape::RecordType(fields) | Shape::Record(fields) => fields.iter().all(|field| meta_free(&field.term)),
        Shape::Project { record, .. } => meta_free(record),
        Shape::Id { ty, left, right } => meta_free(ty) && meta_free(left) && meta_free(right),
        Shape::Refl(value) => meta_free(value),
        Shape::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => {
            meta_free(ty)
                && meta_free(from)
                && meta_free(motive)
                && meta_free(base)
                && meta_free(to)
                && meta_free(proof)
        }
        Shape::Let { ty, value, body, .. } => meta_free(ty) && meta_free(value) && meta_free(body),
    }
}
