//! Provenance: `docs/rules/language/02-core-calculus.md` §7, stated as laws.
//!
//! > Every core term records the surface node it was elaborated from.
//! > Provenance is preserved by substitution, by instantiation, and by
//! > quotation. Two terms with different origins and the same normal form are
//! > convertible.
//!
//! Two halves that pull against each other, which is why both are tested here
//! rather than one being taken on trust. Origins must survive everything
//! evaluation and quotation do to a term, *and* must be invisible to every
//! comparison — a compiler that type-checked differently after a file was moved
//! would be what "origins are part of equality" buys.
//!
//! The corpus in `main.rs` gives its types, its terms, and its context binders
//! three different origins on purpose. Almost every way of getting §7 wrong
//! takes the origin of the type that drove quotation instead of the value that
//! was quoted, and with one origin for all three nothing below could tell.

use std::collections::BTreeSet;
use std::sync::Arc;

use musa_core::{Cx, Field, Index, Level, Name, Origin, Shape, Term, convertible, normalize};

use crate::fixtures::{BINDERS, Sample, corpus};

/// The origin every node of a hand-built type in this file carries.
const TYPE: Origin = Origin::node(400);
/// Where a variable was *written*, as opposed to where its value came from.
const USE: Origin = Origin::node(401);
/// Where the thing a definition stands for was written.
const DEFINITION: Origin = Origin::node(402);
const BINDER_A: Origin = Origin::node(403);
const BINDER_F: Origin = Origin::node(404);
const BINDER_R: Origin = Origin::node(405);
const BINDER_X: Origin = Origin::node(406);
/// Two origins no fixture uses, for restamping a corpus term.
const FOREIGN: Origin = Origin::node(407);
const ELSEWHERE: Origin = Origin::node(408);

/// §7, a term is convertible with the same term written somewhere else.
///
/// The sharpest form of "origins are not part of conversion": same shape, every
/// node stamped differently, and the answer is `true` at every sample —
/// including the samples where the two *sides* disagree, because this compares
/// each side with itself.
#[test]
fn a_term_is_convertible_with_itself_written_somewhere_else() {
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
            let elsewhere = restamp(term, FOREIGN);
            assert_eq!(
                convertible(&cx, &ty, term, &elsewhere),
                Ok(true),
                "{name}: moving a term does not change what it means"
            );
        }
    }
}

/// §7, conversion answers the same question however the terms are stamped.
///
/// The negative samples carry this one: a conversion that had begun to compare
/// origins would answer `false` everywhere, which the test above could not see.
#[test]
fn conversion_answers_the_same_however_the_terms_are_stamped() {
    for Sample {
        name,
        cx,
        ty,
        left,
        right,
        equal,
    } in corpus()
    {
        let ty = restamp(&ty, FOREIGN);
        let left = restamp(&left, FOREIGN);
        let right = restamp(&right, ELSEWHERE);
        assert_eq!(convertible(&cx, &ty, &left, &right), Ok(equal), "{name}");
    }
}

/// §7, normalization invents no provenance.
///
/// Every origin in a normal form is one the question already carried — from the
/// term, from the type, or from a context binder — with [`Origin::UNKNOWN`] the
/// one exception, because the variables quotation creates during η-expansion
/// have no surface node to point at and a plausible-looking guess would be
/// worse than an honest absence.
#[test]
fn a_normal_form_carries_no_origin_the_question_did_not() {
    for Sample { name, cx, ty, left, .. } in corpus() {
        let mut asked = BTreeSet::from([Origin::UNKNOWN, BINDERS]);
        origins(&ty, &mut asked);
        origins(&left, &mut asked);

        let normal = normalize(&cx, &ty, &left).expect("the corpus normalizes");
        let mut answered = BTreeSet::new();
        origins(&normal, &mut answered);

        let invented: Vec<Origin> = answered.difference(&asked).copied().collect();
        assert!(invented.is_empty(), "{name}: {invented:?} came from nowhere");
    }
}

/// §7, β answers the body's origins rather than the application's.
///
/// This is the preservation-by-substitution clause read forwards: the result of
/// `(λz. e) a` is `e`, and `e`'s nodes were written where they were written. An
/// evaluator that stamped its result with the redex's origin would point every
/// diagnostic about an expanded call at the call site.
#[test]
fn beta_answers_the_body_rather_than_the_application() {
    let cx = Cx::new();
    let type1 = Term::universe(TYPE, Level::ZERO.succ());
    let redex = Term::app(
        USE,
        Term::lam(USE, "z", Term::universe(DEFINITION, Level::ZERO)),
        Term::universe(USE, Level::ZERO),
    );

    let normal = normalize(&cx, &type1, &redex).expect("normalizes");
    assert_eq!(
        normal.origin(),
        DEFINITION,
        "the body is where the answer was written, and the redex is not"
    );
}

/// §7, δ carries the definition's origin — from a `let` and from the context
/// alike, because they are the same rule and this crate implements them as one.
#[test]
fn delta_carries_the_definitions_origin() {
    let cx = Cx::new();
    let type1 = Term::universe(TYPE, Level::ZERO.succ());
    let definition = Term::universe(DEFINITION, Level::ZERO);

    let through_let = normalize(
        &cx,
        &type1,
        &Term::bind(USE, "z", type1.clone(), definition.clone(), Term::var(USE, Index(0))),
    )
    .expect("normalizes");
    assert_eq!(
        through_let.origin(),
        DEFINITION,
        "unfolding a let answers what the let was bound to"
    );

    let defined = cx.define(&type1, &definition).expect("d := Type 0");
    let through_context = normalize(&defined, &type1, &Term::var(USE, Index(0))).expect("normalizes");
    assert_eq!(
        through_context.origin(),
        DEFINITION,
        "a context definition unfolds to the same term, from the same place"
    );
}

/// §7, an assumption's occurrences point at its binder.
///
/// An assumption has no value to take an origin from, so [`Cx::assume`] supplies
/// one and every occurrence in a normal form carries it. The alternative is not
/// "the occurrence's own origin" — quotation *invents* most of the variable
/// occurrences in an η-long normal form, and those never had a use site — it is
/// a normal form full of variables pointing nowhere.
#[test]
fn an_assumptions_occurrences_point_at_its_binder() {
    let cx = Cx::new();
    let a = cx
        .assume(BINDER_A, &Term::universe(TYPE, Level::ZERO))
        .expect("A : Type 0");

    let normal =
        normalize(&a, &Term::universe(TYPE, Level::ZERO.succ()), &Term::var(USE, Index(0))).expect("normalizes");
    assert_eq!(normal.origin(), BINDER_A, "A was written where A was assumed");
}

/// §7, "the expansion carries the origin of the term it expanded", at Π.
///
/// `f` normalizes to `λz. f z`, and every node of that except the invented
/// variable points at `f`. Not at `f`'s *type*, which is what drove the
/// expansion and belongs to a different term: a reader told that a function's
/// normal form came from the signature would be told nothing they could act on.
#[test]
fn eta_at_a_function_type_carries_the_expanded_terms_origin() {
    let cx = Cx::new();
    let a = cx
        .assume(BINDER_A, &Term::universe(TYPE, Level::ZERO))
        .expect("A : Type 0");
    let arrow = Term::pi(TYPE, "z", Term::var(TYPE, Index(0)), Term::var(TYPE, Index(1)));
    let f = a.assume(BINDER_F, &arrow).expect("f : A → A");

    let arrow_in_f = Term::pi(TYPE, "z", Term::var(TYPE, Index(1)), Term::var(TYPE, Index(2)));
    let normal = normalize(&f, &arrow_in_f, &Term::var(USE, Index(0))).expect("normalizes");

    assert_eq!(normal.origin(), BINDER_F, "the λ η wrote points at f");
    let Shape::Lam { body, .. } = normal.shape() else {
        panic!("a variable at a function type reads back as a lambda, got {normal:?}")
    };
    assert_eq!(body.origin(), BINDER_F, "so does the application inside it");
    let Shape::App { function, argument } = body.shape() else {
        panic!("η writes an application, got {body:?}")
    };
    assert_eq!(function.origin(), BINDER_F);
    assert_eq!(
        argument.origin(),
        Origin::UNKNOWN,
        "the variable quotation invented has no surface node, and says so"
    );
}

/// §7's η clause again, at a record type.
///
/// Both cases are tested because they are two code paths, and the record one is
/// where the type is most tempting to reach for: quotation is walking the
/// telescope's fields when it writes each projection.
#[test]
fn eta_at_a_record_type_carries_the_expanded_terms_origin() {
    let cx = Cx::new();
    let a = cx
        .assume(BINDER_A, &Term::universe(TYPE, Level::ZERO))
        .expect("A : Type 0");
    let pair_type = Term::record_type(
        TYPE,
        [("fst", Term::var(TYPE, Index(0))), ("snd", Term::var(TYPE, Index(1)))],
    );
    let r = a.assume(BINDER_R, &pair_type).expect("r : { fst : A, snd : A }");

    let pair_in_r = Term::record_type(
        TYPE,
        [("fst", Term::var(TYPE, Index(1))), ("snd", Term::var(TYPE, Index(2)))],
    );
    let normal = normalize(&r, &pair_in_r, &Term::var(USE, Index(0))).expect("normalizes");

    assert_eq!(normal.origin(), BINDER_R, "the literal η wrote points at r");
    let Shape::Record(fields) = normal.shape() else {
        panic!("a variable at a record type reads back as a literal, got {normal:?}")
    };
    for Field { name, term } in fields.iter() {
        assert_eq!(term.origin(), BINDER_R, "the projection written for {name} points at r");
        let Shape::Project { record, field: _ } = term.shape() else {
            panic!("η writes a projection per field, got {term:?}")
        };
        assert_eq!(record.origin(), BINDER_R);
    }
}

/// §7, a blocked elimination keeps its *own* origin.
///
/// The other side of the substitution clause, and the reason evaluation's
/// eliminators take an origin at all. `f x` computes to nothing, so the node
/// that survives into the normal form is the application the author wrote, and
/// it is the only node that can describe itself. Its children still point where
/// their values came from.
#[test]
fn a_blocked_elimination_keeps_the_origin_it_was_written_at() {
    let cx = Cx::new();
    let a = cx
        .assume(BINDER_A, &Term::universe(TYPE, Level::ZERO))
        .expect("A : Type 0");
    let arrow = Term::pi(TYPE, "z", Term::var(TYPE, Index(0)), Term::var(TYPE, Index(1)));
    let f = a.assume(BINDER_F, &arrow).expect("f : A → A");
    let x = f.assume(BINDER_X, &Term::var(TYPE, Index(1))).expect("x : A");

    let applied = Term::app(USE, Term::var(USE, Index(1)), Term::var(USE, Index(0)));
    let normal = normalize(&x, &Term::var(TYPE, Index(2)), &applied).expect("normalizes");

    assert_eq!(normal.origin(), USE, "nothing computed, so the written node survives");
    let Shape::App { function, argument } = normal.shape() else {
        panic!("a blocked application reads back as one, got {normal:?}")
    };
    assert_eq!(function.origin(), BINDER_F, "the head still points at f's binder");
    assert_eq!(argument.origin(), BINDER_X, "and the argument at x's");
}

/// Every origin anywhere in `term`.
fn origins(term: &Term, into: &mut BTreeSet<Origin>) {
    into.insert(term.origin());
    for child in children(term) {
        origins(child, into);
    }
}

/// The same term with every node, at every depth, stamped `origin`.
///
/// Written out per shape rather than through a generic map because there is no
/// generic map: the point of the exercise is that a term differing from another
/// *only* in provenance is still the same term, and a rewrite that quietly
/// dropped a subterm would make that claim about something else.
fn restamp(term: &Term, origin: Origin) -> Term {
    let shape = match term.shape() {
        Shape::Var(index) => Shape::Var(*index),
        Shape::Const(constant) => Shape::Const(constant.clone()),
        Shape::Base(base) => Shape::Base(base.clone()),
        Shape::Builtin(builtin) => Shape::Builtin(builtin.clone()),
        Shape::Lit(literal) => Shape::Lit(literal.clone()),
        Shape::Universe(level) => Shape::Universe(level.clone()),
        Shape::Pi {
            plicity,
            name,
            domain,
            codomain,
        } => Shape::Pi {
            plicity: *plicity,
            name: Arc::clone(name),
            domain: restamp(domain, origin),
            codomain: restamp(codomain, origin),
        },
        Shape::Lam { name, body } => Shape::Lam {
            name: Arc::clone(name),
            body: restamp(body, origin),
        },
        Shape::App { function, argument } => Shape::App {
            function: restamp(function, origin),
            argument: restamp(argument, origin),
        },
        Shape::RecordType(fields) => Shape::RecordType(restamp_fields(fields, origin)),
        Shape::Record(fields) => Shape::Record(restamp_fields(fields, origin)),
        Shape::Project { record, field } => Shape::Project {
            record: restamp(record, origin),
            field: Name::clone(field),
        },
        Shape::Id { ty, left, right } => Shape::Id {
            ty: restamp(ty, origin),
            left: restamp(left, origin),
            right: restamp(right, origin),
        },
        Shape::Refl(value) => Shape::Refl(restamp(value, origin)),
        Shape::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => Shape::J {
            ty: restamp(ty, origin),
            from: restamp(from, origin),
            motive: restamp(motive, origin),
            base: restamp(base, origin),
            to: restamp(to, origin),
            proof: restamp(proof, origin),
        },
        Shape::Let { name, ty, value, body } => Shape::Let {
            name: Arc::clone(name),
            ty: restamp(ty, origin),
            value: restamp(value, origin),
            body: restamp(body, origin),
        },
        // A meta has no subterms to restamp, and its identity is the cell rather
        // than anything written here — cloning it keeps the same unknown.
        Shape::Meta(meta) => Shape::Meta(meta.clone()),
    };
    Term::new(origin, shape)
}

fn restamp_fields(fields: &[Field], origin: Origin) -> Arc<[Field]> {
    fields
        .iter()
        .map(|Field { name, term }| Field {
            name: Arc::clone(name),
            term: restamp(term, origin),
        })
        .collect()
}

/// A term's immediate subterms, in the order they were written.
fn children(term: &Term) -> Vec<&Term> {
    match term.shape() {
        Shape::Var(_) | Shape::Universe(_) | Shape::Const(_) | Shape::Base(_) | Shape::Builtin(_) | Shape::Lit(_) => {
            Vec::new()
        }
        Shape::Pi { domain, codomain, .. } => vec![domain, codomain],
        Shape::Lam { body, .. } => vec![body],
        Shape::App { function, argument } => vec![function, argument],
        Shape::RecordType(fields) | Shape::Record(fields) => fields.iter().map(|field| &field.term).collect(),
        Shape::Project { record, field: _ } => vec![record],
        Shape::Id { ty, left, right } => vec![ty, left, right],
        Shape::Refl(value) => vec![value],
        Shape::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => vec![ty, from, motive, base, to, proof],
        Shape::Let { ty, value, body, .. } => vec![ty, value, body],
        Shape::Meta(_) => Vec::new(),
    }
}
