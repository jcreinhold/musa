//! Definitional equality: that it is an equivalence, that it decides the rules
//! `docs/rules/language/02-core-calculus.md` §3 names, and that it decides them
//! *only* where §3 says so.
//!
//! Each test names the §5 obligation it partially discharges. "Partially" is the
//! honest word: a law tested over a corpus is tested at the terms in that
//! corpus, and prompt 169 owes the metatheory matrix — soundness and
//! completeness of `NbE` against the declarative rules, decidability, subject
//! reduction, canonicity — which no example-based suite can supply.

use std::sync::Arc;

use musa_calculus::{
    Base, Budget, CoreError, Cx, Index, Literal, Origin, Payload, Registry, Sort, Term, convertible, convertible_types,
};

use crate::fixtures::{Sample, corpus, corpus_at};

/// Everything built here is written by the test rather than by an author, so
/// one origin is enough; `provenance_laws.rs` is where origins are the subject.
const HERE: Origin = Origin::node(900);

/// §5.1, conversion is decidable and an equivalence — the reflexive half.
#[test]
fn conversion_is_reflexive() {
    for Sample {
        name,
        cx,
        ty,
        left,
        right,
        ..
    } in corpus()
    {
        for term in [&left, &right] {
            assert_eq!(
                convertible(&cx, &ty, term, term),
                Ok(true),
                "{name}: every term is convertible with itself"
            );
        }
    }
}

/// §5.1, the symmetric half.
#[test]
fn conversion_is_symmetric() {
    for Sample {
        name,
        cx,
        ty,
        left,
        right,
        equal,
    } in corpus()
    {
        assert_eq!(convertible(&cx, &ty, &left, &right), Ok(equal), "{name}");
        assert_eq!(
            convertible(&cx, &ty, &right, &left),
            Ok(equal),
            "{name}: convertibility does not depend on which side is asked first"
        );
    }
}

/// §5.1, the transitive half.
///
/// Stated through the normal form, which is the only third term available
/// without a second corpus: `left ≡ nf` and `nf ≡ right` must give `left ≡
/// right`. That is transitivity at the one shape `NbE` actually produces.
#[test]
fn conversion_is_transitive() {
    for Sample {
        name,
        cx,
        ty,
        left,
        right,
        equal,
    } in corpus()
    {
        let middle = musa_calculus::normalize(&cx, &ty, &left).expect("the corpus normalizes");
        assert_eq!(convertible(&cx, &ty, &left, &middle), Ok(true), "{name}: left ≡ nf");
        assert_eq!(
            convertible(&cx, &ty, &middle, &right),
            Ok(equal),
            "{name}: nf ≡ right exactly when left ≡ right"
        );
    }
}

/// §3's rules, each at the sample that exercises it: β, δ, ι, η at Π, η at
/// records, and projection.
///
/// The corpus is the statement. A sample marked `equal: false` is as much a part
/// of the law as one marked `true`, because a conversion that answered `true`
/// everywhere would pass every test above.
#[test]
fn conversion_decides_each_rule_the_specification_names() {
    for Sample {
        name,
        cx,
        ty,
        left,
        right,
        equal,
    } in corpus()
    {
        assert_eq!(convertible(&cx, &ty, &left, &right), Ok(equal), "{name}");
    }
}

/// §3, conversion agrees with normalization.
///
/// Today this holds by construction — [`convertible`] normalizes both sides and
/// compares — and the test is worth keeping anyway, because prompt 134's
/// elaborator will want a conversion that stops at the first difference rather
/// than quoting two whole normal forms, and this is the law that fast path has
/// to keep.
#[test]
fn conversion_agrees_with_normalization() {
    for Sample {
        name,
        cx,
        ty,
        left,
        right,
        ..
    } in corpus()
    {
        let left_nf = musa_calculus::normalize(&cx, &ty, &left).expect("the corpus normalizes");
        let right_nf = musa_calculus::normalize(&cx, &ty, &right).expect("the corpus normalizes");
        assert_eq!(
            convertible(&cx, &ty, &left, &right),
            Ok(left_nf == right_nf),
            "{name}: convertible exactly when the normal forms are α-equal"
        );
    }
}

/// §1, universes are predicative and **not** cumulative, so conversion compares
/// levels for equality and never for inclusion.
#[test]
fn conversion_at_universes_is_equality_and_not_inclusion() {
    let cx = Cx::new();
    let zero = Term::universe(HERE, Sort::ZERO);
    let one = Term::universe(HERE, Sort::ONE);
    assert_eq!(convertible_types(&cx, &zero, &zero), Ok(true));
    assert_eq!(
        convertible_types(&cx, &zero, &one),
        Ok(false),
        "Type 0 is not a Type 1: a subtyping rule here would be cumulativity"
    );
}

/// §3, a name is not part of a term's meaning.
///
/// Quotation writes a Π's binder name onto the λ it η-expands, so two functions
/// differing only in what they call their argument are not merely convertible —
/// they read back as the same term.
#[test]
fn conversion_ignores_binder_names() {
    let cx = Cx::new();
    let a = cx.assume(HERE, &Term::universe(HERE, Sort::ZERO)).expect("A : Type 0");
    let arrow = Term::pi(HERE, "z", Term::var(HERE, Index(0)), Term::var(HERE, Index(1)));
    let by_one_name = Term::lam(HERE, "first", Term::var(HERE, Index(0)));
    let by_another = Term::lam(HERE, "second", Term::var(HERE, Index(0)));
    assert_eq!(convertible(&a, &arrow, &by_one_name, &by_another), Ok(true));
}

/// §4, a term this crate cannot make sense of is reported, not panicked on.
///
/// A total language whose checker aborts has replaced a diagnostic with a crash.
/// These are caller defects — prompt 134's elaborator refuses them before they
/// reach here — and the point of the test is that they arrive as values.
#[test]
fn a_malformed_term_is_reported_rather_than_aborting() {
    let cx = Cx::new();
    let unbound = Term::var(HERE, Index(0));
    let type0 = Term::universe(HERE, Sort::ZERO);
    assert!(
        matches!(
            musa_calculus::normalize_type(&cx, &unbound),
            Err(CoreError::Malformed(_))
        ),
        "a variable with no binder is a defect with a name"
    );

    let a = cx.assume(HERE, &type0).expect("A : Type 0");
    let applied_universe = Term::app(HERE, Term::universe(HERE, Sort::ZERO), Term::var(HERE, Index(0)));
    assert!(
        matches!(
            musa_calculus::normalize(&a, &Term::var(HERE, Index(0)), &applied_universe),
            Err(CoreError::Malformed(_))
        ),
        "a universe is not a function"
    );
}

/// §4, the three outcomes stay three.
///
/// `Ok(false)` and `Err(Exhausted)` are different answers, and this is where the
/// distinction is checked rather than assumed: the same question, asked under
/// two budgets, gives an answer under one and an exhaustion under the other.
#[test]
fn exhaustion_is_not_a_negative_answer() {
    let generous = Cx::with_budget(Budget::LANGUAGE);
    let narrow = Cx::with_budget(Budget::LANGUAGE.scaled(200_000));
    let type0 = Term::universe(HERE, Sort::ZERO);
    let deep = (0..64).fold(type0.clone(), |body, _| {
        Term::bind(HERE, "z", type0.clone(), type0.clone(), body)
    });

    assert_eq!(convertible_types(&generous, &deep, &type0), Ok(true));
    assert!(
        matches!(convertible_types(&narrow, &deep, &type0), Err(CoreError::Exhausted(_))),
        "a budget that runs out must say so rather than answering false"
    );
}

/// A fixed quotation allowance, for the samples over a declaration.
///
/// Fixed is the whole point: it is written here rather than derived from the
/// corpus, so a conversion whose read-back grew with the terms it was handed
/// would cross it as soon as a sample got bigger.
const READING_A_DECLARATION: u64 = 512;

/// §3's conversion decides by walking values, and reading back is the failure
/// path's job — so a conversion that says `true` quotes nothing of the terms it
/// was given.
///
/// Stated with the budget rather than with a counter, which is what makes it a
/// law about the language's own accounting instead of an assertion about one
/// implementation's call graph. Before prompt 143 `convertible` normalized both
/// sides and compared, so every sample here would have exhausted; the ones that
/// disagree still read back, because a mismatch's message *is* the two normal
/// forms and §4's report is not optional.
///
/// The samples over a declared family read back one more thing, and it is not
/// the terms: a constant's type is assembled from its declaration on demand
/// (`family.rs` says why it is not stored), and applying one asks for it. Those
/// are held to [`READING_A_DECLARATION`] instead — a *fixed* allowance, so the
/// law still says the read-back does not grow with the question.
#[test]
fn a_conversion_that_agrees_reads_nothing_back() {
    let strict = corpus_at(Budget::LANGUAGE.without_quotation()).expect("the corpus builds without quoting");
    let allowed = corpus_at(Budget::LANGUAGE.quoting(READING_A_DECLARATION)).expect("and at a fixed allowance");
    for (sample, spare) in strict.into_iter().zip(allowed) {
        let Sample {
            name,
            cx,
            ty,
            left,
            right,
            equal,
        } = sample;
        if !equal {
            continue;
        }
        match convertible(&cx, &ty, &left, &right) {
            Ok(answer) => assert!(answer, "{name}: the corpus says these agree"),
            Err(CoreError::Exhausted(_)) => assert_eq!(
                convertible(&spare.cx, &spare.ty, &spare.left, &spare.right),
                Ok(true),
                "{name}: deciding this read back more than the declarations it names"
            ),
            Err(fault @ (CoreError::Malformed(_) | CoreError::Refused { .. })) => panic!("{name}: {fault}"),
        }
    }
}

/// The naive conversion — normalize both sides completely and compare up to α —
/// as a **test-local oracle**, and the second-path audit stated as a test.
///
/// This was `convertible`'s body until Finding G made the facade a call into the
/// unifier. Keeping it here rather than deleting it is the point: it is §3 read
/// literally, with no early exit, no type-directed dispatch, and nothing shared
/// with the procedure it checks. Two implementations of one question are a
/// hazard when both ship and a specification when only one does.
///
/// # Panics
///
/// If either side has no normal form, which the corpus guarantees it does.
fn oracle(cx: &Cx, ty: &Term, left: &Term, right: &Term) -> bool {
    let left = musa_calculus::normalize(cx, ty, left).expect("the corpus normalizes");
    let right = musa_calculus::normalize(cx, ty, right).expect("the corpus normalizes");
    left == right
}

/// The same, for two types.
///
/// # Panics
///
/// As [`oracle`].
fn type_oracle(cx: &Cx, left: &Term, right: &Term) -> bool {
    let left = musa_calculus::normalize_type(cx, left).expect("the corpus normalizes");
    let right = musa_calculus::normalize_type(cx, right).expect("the corpus normalizes");
    left == right
}

/// Conversion and the oracle agree on every sample, at terms and at types.
#[test]
fn conversion_agrees_with_the_naive_oracle() {
    for Sample {
        name,
        cx,
        ty,
        left,
        right,
        ..
    } in corpus()
    {
        assert_eq!(
            convertible(&cx, &ty, &left, &right),
            Ok(oracle(&cx, &ty, &left, &right)),
            "{name}: the unifier and normalize-and-compare disagree"
        );
        // The types the samples are asked at are themselves a corpus of types,
        // and comparing each with itself is the one question available without
        // inventing a second one.
        assert_eq!(
            convertible_types(&cx, &ty, &ty),
            Ok(type_oracle(&cx, &ty, &ty)),
            "{name}: the type disagrees with itself"
        );
    }
}

// ---- a type constructor applied to a value ----------------------------------
//
// `Pc(12)` is written in the surface and reaches the core as the application
// `Pc 12`. Prompt 151 deleted the index stratum that once gave it a shape of its
// own, and the law below is what that deletion bought: §3 says `A ≡ B iff
// quote(A) = quote(B)`, and the erasing wrapper made that equation false about
// the implementation — `quote` dropped the index, so read-back could not tell
// `Pc(12)` from `Pc(24)` and the two were kept apart by comparing values before
// quoting. With no wrapper the equation holds as written.

/// A whole number, as a host would carry one.
#[derive(Debug)]
struct Count(i128);

impl Payload for Count {
    fn same(&self, other: &dyn Payload) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|it| it.0 == self.0)
    }

    fn shown(&self) -> String {
        self.0.to_string()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// A host holding `Count : Type 0` and `Pc : Count → Type 0`.
///
/// # Panics
///
/// If the registry refuses its own signatures, which would be a defect here.
fn counted() -> Cx {
    let count = Base::new("Count", Term::universe(HERE, Sort::ZERO));
    let pc = Base::new(
        "Pc",
        Term::pi(HERE, "n", count.term(HERE), Term::universe(HERE, Sort::ZERO)),
    );
    let registry = Registry::new(vec![count, pc], Vec::new()).expect("the two signatures are admissible");
    Cx::with_budget(Budget::LANGUAGE).with_externs(Arc::new(registry))
}

/// `Pc n`, as a type.
///
/// # Panics
///
/// If the host above did not register the two names it says it does.
fn pc_at(cx: &Cx, count: i128) -> Term {
    let Some(musa_calculus::Extern::Base(counts)) = cx.extern_named("Count") else {
        panic!("`Count` is registered");
    };
    let Some(musa_calculus::Extern::Base(pc)) = cx.extern_named("Pc") else {
        panic!("`Pc` is registered");
    };
    let literal = Literal::new(counts.term(HERE), Arc::new(Count(count)));
    Term::app(HERE, pc.term(HERE), literal.term(HERE))
}

/// §5.1 and §3: a type constructor at two different values is two types, and
/// read-back is what says so.
#[test]
fn a_type_constructor_at_two_values_is_two_types() {
    let cx = counted();
    let twelve = pc_at(&cx, 12);
    let twenty_four = pc_at(&cx, 24);

    assert_eq!(
        convertible_types(&cx, &twelve, &twelve),
        Ok(true),
        "`Pc 12` is the type it is"
    );
    assert_eq!(
        convertible_types(&cx, &twelve, &twenty_four),
        Ok(false),
        "`Pc 12` and `Pc 24` are two types"
    );

    // The second half, and the one the deleted wrapper made false: §3 decides
    // by read-back, so the two normal forms have to differ too. A `quote` that
    // dropped the argument would pass the line above and fail this one.
    let twelve = musa_calculus::normalize_type(&cx, &twelve).expect("`Pc 12` normalizes");
    let twenty_four = musa_calculus::normalize_type(&cx, &twenty_four).expect("`Pc 24` normalizes");
    assert_ne!(twelve, twenty_four, "read-back keeps `Pc 12` and `Pc 24` apart");
}
