//! Numerals: that they mean the tower, that they cost nothing like it, and
//! which types they may be written at.
//!
//! `02-core-calculus.md` §5.10 admits a *representation* rather than a new kind
//! of value, so the obligation is conservativity: for every closed count, the
//! numeral and the constructor tower it stands for are definitionally equal, and
//! no program's meaning moves. That obligation is the first law here, and the
//! other two are why the representation exists at all — the tower costs one
//! evaluator frame and one term node per unit, and a number an author writes is
//! not small.
//!
//! Stated through the facade, like `family_laws.rs`: nothing below names
//! `Shape::Numeral` or a counting family's private shape, because what is being
//! claimed is that `3` and `Succ (Succ (Succ Zero))` are the same *program*.

use musa_calculus::{Budget, Cx, ElabError, Raw, Refusal, Shape, Sort, Term};

use crate::family_laws::{apply, container_context, nat_context, var};
use crate::programs::{WRITTEN, refusal};

/// A number at `Nat`, as one node.
fn numeral(count: u64) -> Raw {
    Raw::numeral(WRITTEN, "Nat", count)
}

/// The same number as `count` applications of `Nat.Succ` to `Nat.Zero`.
fn tower(count: u64) -> Raw {
    (0..count).fold(var("Nat.Zero"), |built, _| apply(var("Nat.Succ"), [built]))
}

/// `Nat` as a core term, for a checking question.
///
/// # Panics
///
/// If `Nat` is not a type, which would be a defect in this crate.
fn nat(cx: &Cx) -> Term {
    musa_calculus::infer(cx, &var("Nat")).expect("`Nat` is a type").0
}

/// `n`'s predecessor, written as a `match`.
///
/// The elimination the conservativity law needs: it reaches both arms, binds the
/// step's field, and answers at `Nat` — so a numeral and a tower that computed
/// different answers here would differ in a program rather than only in a
/// representation.
fn predecessor(subject: Raw) -> Raw {
    Raw::match_on(
        WRITTEN,
        [subject],
        vec![
            musa_calculus::RawArm {
                patterns: vec![musa_calculus::RawPattern::constructor(WRITTEN, "Nat.Zero", [])],
                body: var("Nat.Zero"),
            },
            musa_calculus::RawArm {
                patterns: vec![musa_calculus::RawPattern::constructor(
                    WRITTEN,
                    "Nat.Succ",
                    [musa_calculus::RawPattern::bind(WRITTEN, "k")],
                )],
                body: var("k"),
            },
        ],
    )
}

/// How deep a term nests, in nodes.
///
/// The measurement the whole prompt is about, and it is written here rather than
/// inferred from a printed form so that "one node" is a claim about the term and
/// not about the printer. Exhaustive on `Shape`: a variant added to the core has
/// to be classified here before this suite builds again.
fn depth(term: &Term) -> u32 {
    let deeper = |inner: &Term| depth(inner).saturating_add(1);
    match *term.shape() {
        Shape::App {
            ref function,
            ref argument,
        } => deeper(function).max(deeper(argument)),
        Shape::Indexed { ref ty, ref index } => deeper(ty).max(deeper(index)),
        Shape::Bind {
            ref binder, ref body, ..
        } => binder
            .outer()
            .fold(deeper(body), |so_far, term| so_far.max(deeper(term))),
        Shape::Project { ref record, .. } => deeper(record),
        Shape::RecordType(ref fields) | Shape::Record(ref fields) => {
            fields.iter().fold(1, |so_far, field| so_far.max(deeper(&field.term)))
        }
        // The leaves, and the numeral is one of them — which is the claim.
        Shape::Var(_) | Shape::Named { .. } | Shape::Lit(_) | Shape::Meta(_) | Shape::Universe(_) => 1,
    }
}

/// §5.10's conservativity obligation: a numeral *is* the tower it stands for.
///
/// Two claims per count, because either alone would be too weak. Convertibility
/// says the two terms are equal under §3, and computing the same answer through
/// a `match` says the equality survives an elimination — which is where a
/// representation that agreed only on closed comparisons would come apart.
///
/// The counts stop where the tower does, and that bound is the finding rather
/// than a gap in the law: a tower of depth `n` costs `n` evaluator frames, and
/// past a couple of hundred there is no second term to be equal to. The law at
/// large counts is stated the other way round, one test down.
#[test]
fn a_numeral_and_the_tower_it_stands_for_are_one_program() {
    let (cx, _) = nat_context();
    let nat = nat(&cx);
    for count in [0, 1, 2, 7, 63, 127] {
        let counted =
            musa_calculus::check(&cx, &nat, &numeral(count)).unwrap_or_else(|error| panic!("{count}: {error}"));
        let built = musa_calculus::check(&cx, &nat, &tower(count)).unwrap_or_else(|error| panic!("{count}: {error}"));
        assert!(
            musa_calculus::convertible(&cx, &nat, &counted, &built).unwrap_or_else(|error| panic!("{count}: {error}")),
            "the numeral {count} and the tower it stands for are not convertible"
        );

        let from_count = musa_calculus::check(&cx, &nat, &predecessor(numeral(count)))
            .and_then(|term| musa_calculus::normalize(&cx, &nat, &term).map_err(ElabError::from))
            .unwrap_or_else(|error| panic!("{count}: {error}"));
        let from_tower = musa_calculus::check(&cx, &nat, &predecessor(tower(count)))
            .and_then(|term| musa_calculus::normalize(&cx, &nat, &term).map_err(ElabError::from))
            .unwrap_or_else(|error| panic!("{count}: {error}"));
        assert!(
            musa_calculus::convertible(&cx, &nat, &from_count, &from_tower)
                .unwrap_or_else(|error| panic!("{count}: {error}")),
            "matching on the numeral {count} and on its tower computed different answers"
        );
    }
}

/// `Nat` under a budget divided by sixteen, which leaves sixteen nesting levels.
///
/// Sixteen levels rather than 256 so that the law below can put a count of five
/// thousand and a count of five under the *same* narrow limit: three orders of
/// magnitude apart and both fitting, which is a sharper way of saying that no
/// per-unit nesting is charged than 256 levels would be.
///
/// It used to be here for a worse reason — at the full budget a tower deep
/// enough to reach 256 aborted the process around 215 levels, so the narrow
/// budget was dodging a crash rather than making a point. `musa-calculus`'s `room`
/// module discharges §4.1's second half now, and the tower law that used to
/// share this context is stated at the language budget again.
///
/// # Panics
///
/// If `Nat` is not a declaration, which would be a defect in this crate.
fn narrow_nat_context() -> Cx {
    let cx = Cx::with_budget(Budget::LANGUAGE.scaled(16));
    let group = musa_calculus::declare(&cx, &crate::family_laws::nat()).expect("Nat is a declaration");
    cx.declaring(&group)
}

/// The finding this representation exists for, as a law.
///
/// Past the nesting limit the tower is not a term the calculus can evaluate:
/// `eval` charges one level per level of the term, so a tower of depth `n` costs
/// `n` levels and a count is not a small number. The pair here is not two terms
/// to compare — it is one term and one exhaustion, and the exhaustion is what
/// `repeat 384` used to be.
///
/// The two counts are an order of magnitude apart in the direction that makes
/// the point: the *smaller* one is the one that cannot be written.
///
/// Stated at [`Budget::LANGUAGE`] rather than at a narrowed share of it, which
/// is what §4.1's room obligation buys. Before `musa-calculus`'s `room` module a
/// tower deep enough to reach the real limit of 256 aborted the process at
/// about 215 levels, so this law could only be stated at a divided budget —
/// about the language's limit, but not at it.
#[test]
fn a_tower_past_the_nesting_limit_exhausts_where_the_numeral_answers() {
    let (cx, _) = nat_context();
    let nat = nat(&cx);
    let counted = musa_calculus::check(&cx, &nat, &numeral(5_000)).expect("a numeral is one node at any count");
    musa_calculus::normalize(&cx, &nat, &counted).expect("and evaluating one is reading it");

    let past = Budget::NESTING.saturating_add(1);
    match musa_calculus::check(&cx, &nat, &tower(past)) {
        Err(ElabError::Exhausted(exhausted)) => {
            assert_eq!(
                exhausted.metric,
                musa_calculus::Metric::Nesting,
                "the tower ran out of depth"
            );
        }
        Err(other) => panic!("the tower was refused rather than exhausted: {other}"),
        Ok(_) => panic!(
            "a tower {past} deep elaborated under {} nesting levels",
            Budget::NESTING
        ),
    }
}

/// §4.1's nesting metric does not see the count.
///
/// Stated as a budget rather than as a measurement, because the meter's
/// high-water mark is private and exposing it for a test would be a knob added
/// for a test. Both counts elaborate and evaluate under sixteen nesting levels,
/// and they are three orders of magnitude apart, so no per-unit charge is being
/// paid.
#[test]
fn a_numeral_costs_a_nesting_depth_its_count_never_reaches() {
    for count in [5_u64, 5_000] {
        let cx = narrow_nat_context();
        let nat = nat(&cx);
        let term = musa_calculus::check(&cx, &nat, &numeral(count)).unwrap_or_else(|error| panic!("{count}: {error}"));
        musa_calculus::normalize(&cx, &nat, &term).unwrap_or_else(|error| panic!("{count}: {error}"));
    }
}

/// Fifty thousand, in one node, on both sides of the data boundary.
///
/// `nat_fold(0, keep, 50000)` did not refuse before this representation — it
/// *aborted*, because building a fifty-thousand-node left spine overflows the
/// host stack before any budget can speak, and §4 says a total language may
/// refuse but may not crash. The claim has two halves because there are two
/// places a tower could come back: the term, and the [`musa_calculus::Datum`] a
/// δ-rule reads. A tower in either would cost a node per unit and recurse on
/// drop.
#[test]
fn a_numeral_of_fifty_thousand_neither_overflows_nor_deepens() {
    let (cx, _) = nat_context();
    let nat = nat(&cx);
    let count = 50_000;
    let term = musa_calculus::check(&cx, &nat, &numeral(count)).expect("fifty thousand is one node");
    let normal = musa_calculus::normalize(&cx, &nat, &term).expect("and evaluating it is reading it");
    assert_eq!(depth(&term), 1, "the elaborated numeral is one node");
    assert_eq!(depth(&normal), 1, "and so is its normal form");

    let datum = musa_calculus::canonical(&cx, &normal).expect("a closed numeral is canonical data");
    assert_eq!(
        datum,
        musa_calculus::Datum::Count {
            family: std::sync::Arc::from("Nat"),
            count,
        },
        "and it crosses the data boundary as a count rather than as a tower"
    );
}

/// The programs §5.10 refuses, for the coverage gate in `elaboration_laws.rs`.
///
/// They carry their own contexts because the question is about a *declaration*:
/// what a family has to look like for a number to be writable at it. One context
/// could not hold a parameterized family and an unparameterized one under the
/// same name.
pub(crate) struct RefusedNumeral {
    pub(crate) name: &'static str,
    pub(crate) cx: Cx,
    pub(crate) raw: Raw,
    pub(crate) expected: fn(&Refusal) -> bool,
}

pub(crate) fn refused_numerals() -> Vec<RefusedNumeral> {
    vec![
        RefusedNumeral {
            name: "a number at a family that takes a parameter",
            cx: container_context(),
            raw: Raw::numeral(WRITTEN, "Option", 3),
            expected: |refusal| matches!(*refusal, Refusal::NotANumeralFamily { .. }),
        },
        RefusedNumeral {
            name: "a number at a case rather than at its type",
            cx: nat_context().0,
            raw: Raw::numeral(WRITTEN, "Nat.Succ", 3),
            expected: |refusal| matches!(*refusal, Refusal::NotANumeralFamily { .. }),
        },
    ]
}

/// The reason names the condition that failed, and not merely that one did.
///
/// The whole value of the report: four conditions fail for four different edits,
/// and "`Option` is not a numeral family" tells whoever wrote the reading none of
/// them.
#[test]
fn a_refused_numeral_says_which_condition_the_family_fails() {
    let cx = container_context();
    let Err(error) = musa_calculus::infer(&cx, &Raw::numeral(WRITTEN, "Option", 3)) else {
        panic!("`Option` takes a parameter, so a number cannot be written at it");
    };
    let refusal = refusal("a number at a parameterized family", error);
    let Refusal::NotANumeralFamily { ref name, reason, .. } = refusal else {
        panic!("expected a numeral-family refusal, got `{refusal}`");
    };
    assert_eq!(&**name, "Option");
    assert!(
        reason.contains("parameters"),
        "the reason named something other than the parameters: `{reason}`"
    );
}

/// A family of the right *shape* counts, whatever it is called.
///
/// The property is derived from the declaration rather than nominated by the
/// host, which is what makes this statable at all: `Nat` is not privileged, and a
/// second family of the same shape gets the representation without asking. Not a
/// feature to build a surface for — a consequence of deriving, and the test that
/// says deriving is what happened.
#[test]
fn any_family_of_the_counting_shape_takes_a_numeral() {
    let cx = Cx::new();
    let declaration = crate::family_laws::data(
        Vec::new(),
        vec![crate::family_laws::family(
            "Level",
            vec![
                // Declared step-first, so that the recognition cannot be reading
                // constructor *order* instead of constructor shape.
                crate::family_laws::constructor("Deeper", vec![crate::family_laws::binder("under", var("Level"))]),
                crate::family_laws::constructor("Surface", Vec::new()),
            ],
        )],
    );
    let group = musa_calculus::declare(&cx, &declaration).expect("Level is a declaration");
    let cx = cx.declaring(&group);
    let ty = musa_calculus::infer(&cx, &var("Level")).expect("`Level` is a type").0;
    let counted = musa_calculus::check(&cx, &ty, &Raw::numeral(WRITTEN, "Level", 4)).expect("four is a `Level`");
    let built = musa_calculus::check(
        &cx,
        &ty,
        &(0..4).fold(var("Level.Surface"), |built, _| apply(var("Level.Deeper"), [built])),
    )
    .expect("and so is the tower");
    assert!(
        musa_calculus::convertible(&cx, &ty, &counted, &built).expect("both are closed"),
        "a family of the counting shape did not count"
    );
}

/// A universe is not a counting family, and the report says so rather than
/// guessing.
///
/// The path a *reader* takes when it names something that is not a declared type
/// at all: elaboration answers with the name's own resolution failure, and never
/// with a numeral-family sentence about a name nobody declared.
#[test]
fn a_number_at_an_undeclared_name_is_an_unknown_name() {
    let (cx, _) = nat_context();
    let Err(error) = musa_calculus::infer(&cx, &Raw::numeral(WRITTEN, "Nowhere", 3)) else {
        panic!("`Nowhere` is not declared");
    };
    let refusal = refusal("a number at an undeclared name", error);
    assert!(
        matches!(refusal, Refusal::UnknownName { .. }),
        "expected an unknown name, got `{refusal}`"
    );
}

/// A numeral is a term of its family and never a type.
///
/// The one thing a representation at a *declared* family could get wrong that a
/// base literal could not: `Nat` is a type and `3` is not, and a rule that read
/// the numeral's family off the numeral would have to say which.
#[test]
fn a_numeral_is_not_a_type() {
    let (cx, _) = nat_context();
    let Err(error) = musa_calculus::check(&cx, &Term::universe(WRITTEN, Sort::ZERO), &numeral(3)) else {
        panic!("`3` is not a type");
    };
    let refusal = refusal("a number in type position", error);
    assert!(
        matches!(refusal, Refusal::Mismatch(_)),
        "expected a conversion mismatch, got `{refusal}`"
    );
}
