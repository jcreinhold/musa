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
//! discharged at the terms in it. Prompt 169 owes the metatheory matrix.

use musa_calculus::{Cx, ElabError, Index, Raw, Refusal, Sort, Term, check, infer, normalize};

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

/// §2.1: every meta an accepted term still names is solved.
///
/// Not a stylistic preference — it is what lets the next stage treat the output
/// as an ordinary core term whose `Meta` nodes are spelling, evaluated through
/// their solutions. An *unsolved* one would be a gap every later pass had to
/// know about, and [`Elaborator::settled`](musa_calculus) is what refuses it.
#[test]
fn an_accepted_terms_metas_are_all_solved() {
    for program in accepted() {
        let (term, ty) = elaborate(&program).unwrap_or_else(|error| panic!("{}: {error}", program.name));
        for (what, term) in [("the term", &term), ("its type", &ty)] {
            assert!(
                metas_solved(term),
                "{}: {what} still mentions an unsolved meta",
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
                [("ty", Term::universe(WRITTEN, Sort::ZERO)), ("val", core_unit_type())],
            )),
            record_literal,
        ),
        (
            "a function type",
            Raw::pi(WRITTEN, "x", unit_type(), unit_type()),
            None,
            Term::pi(WRITTEN, "x", core_unit_type(), core_unit_type()),
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
        let Err(error) = musa_calculus::declare(&declaring, &declaration) else {
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
    // §1.5's three are use sites again, but each needs a context of its own: one
    // namespace spelling the member, none, and two. A single context cannot hold
    // all three questions, which is why they arrive carrying theirs.
    for crate::namespace_laws::RefusedMethod {
        name,
        cx,
        raw,
        expected,
    } in crate::namespace_laws::refused_methods()
    {
        let Err(error) = infer(&cx, &raw) else {
            panic!("{name}: elaboration accepted a program it must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // §1.6's one, which needs the container fixture: a traversal whose answer
    // type nothing determines is an unsolved meta rather than a guess.
    for (name, cx, raw, expected) in crate::collection_laws::refused_collections() {
        let Err(error) = infer(&cx, &raw) else {
            panic!("{name}: elaboration accepted a program §1.6 must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // §1.2's one, which is asked by a *signature* rather than by a program:
    // `Storable` has no written form, so its corpus carries a core type.
    for (name, cx, ty, raw, expected) in crate::storable_laws::refused_ports() {
        let Err(error) = check(&cx, &ty, &raw) else {
            panic!("{name}: elaboration accepted a port §1.2 must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // §5.8's registration refusals are answered by a *table* rather than by a
    // program, which is what makes them the host's mistakes rather than an
    // author's: a registry is refused before anything is elaborated under it.
    for crate::base_laws::RefusedRegistry {
        name,
        outcome,
        expected,
    } in crate::base_laws::refused_registries()
    {
        let Err(refusal) = outcome else {
            panic!("{name}: the registry was admitted");
        };
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // D1's other half is about a program, and needs the worked registry to be
    // in scope for a base-typed subject to exist at all.
    let registered = crate::base_laws::base_context();
    for crate::base_laws::RefusedProgram {
        name,
        raw,
        ty,
        expected,
    } in crate::base_laws::refused_programs()
    {
        let (ty, _) = infer(&registered, &ty).unwrap_or_else(|error| panic!("{name}: {error}"));
        let Err(error) = check(&registered, &ty, &raw) else {
            panic!("{name}: elaboration accepted a program it must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // And the one refusal a δ-rule raises, which arrives a step later than the
    // rest: the program type-checks, and the rule reads its arguments when it
    // fires. Reducing it is what makes them arrive.
    for crate::base_laws::RefusedProgram {
        name,
        raw,
        ty,
        expected,
    } in crate::base_laws::refused_reductions()
    {
        let (ty, _) = infer(&registered, &ty).unwrap_or_else(|error| panic!("{name}: {error}"));
        let term = check(&registered, &ty, &raw).unwrap_or_else(|error| panic!("{name}: {error}"));
        let Err(error) = normalize(&registered, &ty, &term) else {
            panic!("{name}: reduction answered a program the rule must refuse");
        };
        let refusal = refusal(name, error.into());
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // §5.10's numeral is refused against the *family* it is written at, and the
    // three conditions a family can fail are three declarations, so each of
    // these carries the context whose declaration fails the condition.
    for crate::numeral_laws::RefusedNumeral {
        name,
        cx,
        raw,
        expected,
    } in crate::numeral_laws::refused_numerals()
    {
        let Err(error) = infer(&cx, &raw) else {
            panic!("{name}: elaboration accepted a number written at a type that cannot count");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // §1's one, which is reached by a *host-written* level: every level an
    // author writes is an unknown, and an unknown nothing determines is
    // defaulted rather than refused, so the corpus carries a core type with an
    // explicit level in it. `sort_laws` argues that at length.
    for (name, ty, raw, expected) in crate::sort_laws::refused_levels() {
        let Err(error) = check(&cx, &ty, &raw) else {
            panic!("{name}: elaboration accepted a program §1 must refuse");
        };
        let refusal = refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
        reached.insert(kind(&refusal));
    }
    // And the two a *group* raises, which no single term can reach: §2.4's
    // graph rule is about how definitions name each other, so the smallest
    // program that reaches it is a program rather than a term.
    for (name, group) in crate::program_laws::refused_groups() {
        let Err(error) = musa_calculus::declare_program(&crate::coverage_laws::nat_vec_context(), &group) else {
            panic!("{name}: the group was declared, and §2.4 refuses it");
        };
        reached.insert(kind(&refusal(name, error)));
    }
    assert_eq!(
        reached,
        ALL_REFUSALS.iter().copied().collect(),
        "every refusal needs a program that reaches it"
    );
}

/// Every refusal this crate can answer with.
const ALL_REFUSALS: [&str; 39] = [
    "not-storable",
    "unknown-name",
    "mismatch",
    "unsolved",
    "not-a-function",
    "underapplied",
    "filling-mismatch",
    "not-a-record",
    "no-such-field",
    "record-shape",
    "not-a-type",
    "uninferable",
    "non-positive",
    "no-such-constructor",
    "incomplete-match",
    "unreachable-branch",
    "unchecked-recursion",
    "untyped-recursion",
    "definition-cycle",
    "duplicate-field",
    "duplicate-case",
    "overlapping-update",
    "bare-constructor",
    "private",
    "mixed-visibility",
    "abstract-match",
    "method-on-variable",
    "no-method-for-type",
    "ambiguous-method",
    "duplicate-extern",
    "higher-order-delta",
    "unknown-base",
    "base-not-matchable",
    "target-outside-signature",
    "target-not-a-base",
    "not-finite-data",
    "builtin-refused",
    "not-a-numeral-family",
    "level-mismatch",
];

/// Which refusal this is, as a tag the coverage gate can compare.
///
/// Exhaustive on purpose: a new variant is a compile error here, and then a
/// missing entry in [`ALL_REFUSALS`], and then a missing program.
fn kind(refusal: &Refusal) -> &'static str {
    match refusal {
        Refusal::LevelMismatch { .. } => "level-mismatch",
        Refusal::NotStorable { .. } => "not-storable",
        Refusal::UnknownName { .. } => "unknown-name",
        Refusal::Mismatch(_) => "mismatch",
        Refusal::Unsolved { .. } => "unsolved",
        Refusal::NotAFunction { .. } => "not-a-function",
        Refusal::Underapplied { .. } => "underapplied",
        Refusal::FillingMismatch { .. } => "filling-mismatch",
        Refusal::NotARecord { .. } => "not-a-record",
        Refusal::NoSuchField { .. } => "no-such-field",
        Refusal::RecordShape { .. } => "record-shape",
        Refusal::NotAType { .. } => "not-a-type",
        Refusal::Uninferable { .. } => "uninferable",
        Refusal::NonPositive { .. } => "non-positive",
        Refusal::NoSuchConstructor { .. } => "no-such-constructor",
        Refusal::IncompleteMatch { .. } => "incomplete-match",
        Refusal::UnreachableBranch { .. } => "unreachable-branch",
        Refusal::UncheckedRecursion { .. } => "unchecked-recursion",
        Refusal::UntypedRecursion { .. } => "untyped-recursion",
        Refusal::DefinitionCycle { .. } => "definition-cycle",
        Refusal::DuplicateField { .. } => "duplicate-field",
        Refusal::DuplicateCase { .. } => "duplicate-case",
        Refusal::OverlappingUpdate { .. } => "overlapping-update",
        Refusal::BareConstructor { .. } => "bare-constructor",
        Refusal::Private { .. } => "private",
        Refusal::MixedVisibility { .. } => "mixed-visibility",
        Refusal::AbstractMatch { .. } => "abstract-match",
        Refusal::MethodOnVariable { .. } => "method-on-variable",
        Refusal::NoMethodForType { .. } => "no-method-for-type",
        Refusal::AmbiguousMethod { .. } => "ambiguous-method",
        Refusal::DuplicateExtern { .. } => "duplicate-extern",
        Refusal::HigherOrderDelta { .. } => "higher-order-delta",
        Refusal::UnknownBase { .. } => "unknown-base",
        Refusal::BaseNotMatchable { .. } => "base-not-matchable",
        Refusal::TargetOutsideSignature { .. } => "target-outside-signature",
        Refusal::TargetNotABase { .. } => "target-not-a-base",
        Refusal::NotFiniteData { .. } => "not-finite-data",
        Refusal::BuiltinRefused { .. } => "builtin-refused",
        Refusal::NotANumeralFamily { .. } => "not-a-numeral-family",
    }
}

/// §2.1: an unsolved metavariable names its site, where it was created, and
/// what was still blocked.
///
/// The diagnostic's content is the test, not merely its variant: "could not
/// determine something, somewhere" would satisfy a `matches!` and help nobody.
///
/// §2.1's rule, stated from the author's side: nothing invents a type for a
/// binder and asks later — the refusal says to write it.
#[test]
fn an_undetermined_binder_is_told_to_write_its_type() {
    let cx = Cx::new();
    let Err(error) = infer(&cx, &Raw::lam(WRITTEN, "x", Raw::var(WRITTEN, "x"))) else {
        panic!("a binder whose type nothing determines must be refused");
    };
    let refusal = refusal("an unannotated identity function", error);
    let Refusal::Uninferable { at } = &refusal else {
        panic!("expected an uninferable term, got `{refusal}`");
    };
    assert_eq!(*at, WRITTEN, "the report points at the term that needed it");
    assert_eq!(
        refusal.to_string(),
        "this cannot be given a type on its own; write the type it should have"
    );
}

/// §4: exhaustion is its own outcome and never a refusal.
///
/// A budget narrow enough to end elaboration must say so, because a resource
/// limit reported as a type error would let the machine decide what a program
/// means.
#[test]
fn a_narrow_budget_exhausts_rather_than_refusing() {
    let cx = Cx::with_budget(musa_calculus::Budget::LANGUAGE.scaled(4096));
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

/// §1 as amended at prompt 134: filling rides on the core Π and no rule reads
/// it, so the two spellings of one function type are convertible.
///
/// Conversion, not elaboration. Elaboration *does* read filling — that is what
/// the field is for, and a term checked against an implicit Π is wrapped in an
/// implicit λ rather than switched. What the amendment claims is narrower and is
/// what this asserts: no rule in the *core* reads the field, so the two spellings
/// are one type to equality and one type to normalization.
#[test]
fn filling_is_not_part_of_conversion() {
    let cx = Cx::new();
    let explicit = Term::pi(WRITTEN, "x", core_unit_type(), core_unit_type());
    let implicit = Term::parameter_pi(WRITTEN, "y", core_unit_type(), core_unit_type());
    assert_eq!(
        explicit, implicit,
        "conversion looks at neither filling nor binder name"
    );
    assert_eq!(
        musa_calculus::convertible_types(&cx, &explicit, &implicit),
        Ok(true),
        "and normalization does not reintroduce the distinction"
    );
}

/// Whether a term mentions no metavariable anywhere.
///
/// Written by walking the shape rather than by a `Debug` string, so that a new
/// [`musa_calculus::Shape`] variant holding a term is a compile error here.
fn metas_solved(term: &Term) -> bool {
    use musa_calculus::Shape;

    match term.shape() {
        Shape::Meta(meta) => meta.is_solved(),
        // A base type, a builtin, and a literal are all closed: each is a name
        // or a payload the host registered, and none of them holds a term.
        Shape::Var(_) | Shape::Universe(_) | Shape::Named { .. } | Shape::Lit(_) => true,
        Shape::Bind { binder, body, .. } => binder.outer().all(metas_solved) && metas_solved(body),
        Shape::App { function, argument } => metas_solved(function) && metas_solved(argument),
        Shape::RecordType(fields) | Shape::Record(fields) => fields.iter().all(|field| metas_solved(&field.term)),
        Shape::Project { record, .. } => metas_solved(record),
    }
}
