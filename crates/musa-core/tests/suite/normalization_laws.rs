//! Normalization: that it terminates on the corpus, that it is deterministic
//! and stable, and that what it produces is the η-long normal form
//! `docs/rules/language/02-core-calculus.md` §3 describes rather than merely
//! *a* term with the same meaning.
//!
//! §5's obligations these partially discharge are named per test. What no
//! example-based suite can supply is §5's *strong* normalization — that every
//! well-typed term has a normal form — which prompt 148 owes as a proof
//! obligation and prompt 135's termination checker owes for the recursive
//! definitions this crate does not yet have.

use musa_core::{Cx, Index, Level, Term, normalize, normalize_type};

use crate::fixtures::{Sample, corpus};

/// §5, evaluation is deterministic.
///
/// The same term in the same context normalizes to the same term, every time.
/// A normalizer that consulted a hash map's iteration order, a clock, or an
/// address would fail this, and each of those is a real way to lose it.
#[test]
fn normalization_is_deterministic() {
    for Sample { name, cx, ty, left, .. } in corpus() {
        let once = normalize(&cx, &ty, &left).expect("the corpus normalizes");
        let twice = normalize(&cx, &ty, &left).expect("the corpus normalizes");
        assert_eq!(once, twice, "{name}");
    }
}

/// §3, normalization is stable: a normal form is its own normal form.
#[test]
fn normalization_is_idempotent() {
    for Sample { name, cx, ty, left, .. } in corpus() {
        let ty = normalize_type(&cx, &ty).expect("the corpus normalizes");
        let once = normalize(&cx, &ty, &left).expect("the corpus normalizes");
        let again = normalize(&cx, &ty, &once).expect("a normal form normalizes");
        assert_eq!(once, again, "{name}");
    }
}

/// §3, what comes back has no redex left in it.
///
/// Not a restatement of idempotence: a normalizer that stopped early but
/// stopped *consistently* would be idempotent and would still hand prompt 134's
/// elaborator a `let` to look through. This checks the shape directly — no
/// `let`, no lambda in function position, no projection of a literal, no `J` on
/// `refl`.
#[test]
fn a_normal_form_contains_no_redex() {
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
            let normal = normalize(&cx, &ty, term).expect("the corpus normalizes");
            assert!(is_normal(&normal), "{name}: {normal:?} still contains a redex");
        }
    }
}

/// §3, η is performed by quotation, so a normal form at a Π *is* a lambda and a
/// normal form at a record type *is* a literal — whatever the input looked like.
#[test]
fn normal_forms_are_eta_long() {
    let cx = Cx::new();
    let a = cx.assume(&Term::Universe(Level::ZERO)).expect("A : Type 0");

    let arrow = Term::pi("z", Term::Var(Index(0)), Term::Var(Index(1)));
    let f = a.assume(&arrow).expect("f : A → A");
    let arrow_in_f = Term::pi("z", Term::Var(Index(1)), Term::Var(Index(2)));
    let function = normalize(&f, &arrow_in_f, &Term::Var(Index(0))).expect("f normalizes");
    assert!(
        matches!(function, Term::Lam { .. }),
        "a variable at a function type reads back as a lambda, not as itself"
    );

    let pair_type = Term::record_type([("fst", Term::Var(Index(0))), ("snd", Term::Var(Index(1)))]);
    let r = a.assume(&pair_type).expect("r : { fst : A, snd : A }");
    let pair_in_r = Term::record_type([("fst", Term::Var(Index(1))), ("snd", Term::Var(Index(2)))]);
    let record = normalize(&r, &pair_in_r, &Term::Var(Index(0))).expect("r normalizes");
    assert!(
        matches!(record, Term::Record(_)),
        "a variable at a record type reads back as a literal holding its projections"
    );
}

/// §3, α-equivalent inputs have *identical* normal forms.
///
/// Stronger than "are convertible", and it is what makes conversion a `==`:
/// terms carry de Bruijn indices, so a binder name is not part of the term, and
/// quotation takes the name it writes from the Π it is quoting at.
#[test]
fn alpha_equivalent_terms_normalize_to_the_same_term() {
    let cx = Cx::new();
    let a = cx.assume(&Term::Universe(Level::ZERO)).expect("A : Type 0");
    let arrow = Term::pi("z", Term::Var(Index(0)), Term::Var(Index(1)));

    let by_one_name = normalize(&a, &arrow, &Term::lam("first", Term::Var(Index(0)))).expect("normalizes");
    let by_another = normalize(&a, &arrow, &Term::lam("second", Term::Var(Index(0)))).expect("normalizes");
    assert_eq!(by_one_name, by_another);
    assert_eq!(
        by_one_name,
        Term::lam("z", Term::Var(Index(0))),
        "the name that survives is the function type's, which is the one a reader was shown"
    );
}

/// §3, δ: a definition is unfolded, whether it came from a `let` or from the
/// context.
#[test]
fn definitions_are_unfolded() {
    let cx = Cx::new();
    let type0 = Term::Universe(Level::ZERO);
    let a = cx.assume(&type0).expect("A : Type 0");
    let x = a.assume(&Term::Var(Index(0))).expect("x : A");

    let through_let = normalize(
        &x,
        &Term::Var(Index(1)),
        &Term::bind("z", Term::Var(Index(1)), Term::Var(Index(0)), Term::Var(Index(0))),
    )
    .expect("normalizes");
    assert_eq!(through_let, Term::Var(Index(0)), "let x = v in x is v");

    let defined = x.define(&Term::Var(Index(1)), &Term::Var(Index(0))).expect("d := x");
    let through_context = normalize(&defined, &Term::Var(Index(2)), &Term::Var(Index(0))).expect("normalizes");
    assert_eq!(
        through_context,
        Term::Var(Index(1)),
        "a defined variable reads back as what it was defined to be"
    );
}

/// Whether a term has no redex anywhere inside it.
///
/// Written out rather than matched with a wildcard: a variant added later must
/// make this fail to compile, because a new form of redex that nobody taught
/// this function about would silently pass every test above.
fn is_normal(term: &Term) -> bool {
    match term {
        Term::Var(_) | Term::Universe(_) => true,
        Term::Pi { domain, codomain, .. } => is_normal(domain) && is_normal(codomain),
        Term::Lam { body, .. } => is_normal(body),
        Term::App { function, argument } => {
            !matches!(**function, Term::Lam { .. }) && is_normal(function) && is_normal(argument)
        }
        Term::RecordType(fields) | Term::Record(fields) => fields.iter().all(|field| is_normal(&field.term)),
        Term::Project { record, field: _ } => !matches!(**record, Term::Record(_)) && is_normal(record),
        Term::Id { ty, left, right } => is_normal(ty) && is_normal(left) && is_normal(right),
        Term::Refl(value) => is_normal(value),
        Term::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => {
            !matches!(**proof, Term::Refl(_))
                && is_normal(ty)
                && is_normal(from)
                && is_normal(motive)
                && is_normal(base)
                && is_normal(to)
                && is_normal(proof)
        }
        Term::Let { .. } => false,
    }
}
